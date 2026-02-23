use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::permission::ToolPermission;
use super::{Tool, ToolContext, ToolResult};
use crate::agent::AgentEvent;

const MAX_OUTPUT_BYTES: usize = 100 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_TIMEOUT_SECS: u64 = 120;

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Run a shell command and return stdout, stderr, and exit code. \
         Commands are executed in the working directory. \
         Use `timeout` parameter to set a timeout in seconds (default 30, max 120)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default 30, max 120)"
                }
            },
            "required": ["command"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: command"))?;

        let timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(MAX_TIMEOUT_SECS);

        let spawn_result = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&ctx.cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match spawn_result {
            Ok(c) => c,
            Err(e) => return Err(anyhow::anyhow!("Failed to spawn command: {e}")),
        };

        let stdout_pipe = child.stdout.take().unwrap();
        let stderr_pipe = child.stderr.take().unwrap();

        let mut stdout_lines = BufReader::new(stdout_pipe).lines();
        let mut stderr_lines = BufReader::new(stderr_pipe).lines();

        let mut accumulated = String::new();
        let mut stdout_done = false;
        let mut stderr_done = false;
        let mut timed_out = false;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);

        while !stdout_done || !stderr_done {
            tokio::select! {
                biased;
                result = stdout_lines.next_line(), if !stdout_done => {
                    match result {
                        Ok(Some(line)) => {
                            emit_delta(ctx, &line);
                            if accumulated.len() < MAX_OUTPUT_BYTES {
                                if !accumulated.is_empty() {
                                    accumulated.push('\n');
                                }
                                accumulated.push_str(&line);
                            }
                        }
                        Ok(None) => stdout_done = true,
                        Err(_) => stdout_done = true,
                    }
                }
                result = stderr_lines.next_line(), if !stderr_done => {
                    match result {
                        Ok(Some(line)) => {
                            emit_delta(ctx, &line);
                            if accumulated.len() < MAX_OUTPUT_BYTES {
                                if !accumulated.is_empty() {
                                    accumulated.push('\n');
                                }
                                accumulated.push_str(&line);
                            }
                        }
                        Ok(None) => stderr_done = true,
                        Err(_) => stderr_done = true,
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    timed_out = true;
                    let _ = child.kill().await;
                    break;
                }
            }
        }

        if timed_out {
            return Ok(ToolResult {
                output: if accumulated.is_empty() {
                    format!("Command timed out after {timeout_secs}s")
                } else {
                    truncate_output(&mut accumulated);
                    format!("{accumulated}\n\n(command timed out after {timeout_secs}s)")
                },
                title: format!("bash (timeout): {}", truncate_title(command)),
                metadata: json!({ "exit_code": -1, "timeout": true }),
            });
        }

        let status = child.wait().await;
        let exit_code = status.ok().and_then(|s| s.code()).unwrap_or(-1);

        if accumulated.is_empty() {
            accumulated.push_str("(no output)");
        } else {
            truncate_output(&mut accumulated);
        }

        Ok(ToolResult {
            output: accumulated,
            title: format!("bash: {}", truncate_title(command)),
            metadata: json!({ "exit_code": exit_code }),
        })
    }
}

fn emit_delta(ctx: &ToolContext, line: &str) {
    if let Some(tx) = &ctx.event_tx {
        let _ = tx.send(AgentEvent::ToolOutputDelta {
            tool_name: "bash".to_string(),
            delta: line.to_string(),
        });
    }
}

fn truncate_output(s: &mut String) {
    if s.len() > MAX_OUTPUT_BYTES {
        s.truncate(MAX_OUTPUT_BYTES);
        s.push_str("\n... (output truncated)");
    }
}

fn truncate_title(cmd: &str) -> String {
    if cmd.len() > 60 {
        format!("{}...", &cmd[..57])
    } else {
        cmd.to_string()
    }
}
