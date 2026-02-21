use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::process::Command;

use super::{Tool, ToolContext, ToolResult};
use super::permission::ToolPermission;

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

        let result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&ctx.cwd)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let exit_code = output.status.code().unwrap_or(-1);

                truncate_output(&mut stdout);
                truncate_output(&mut stderr);

                let mut out = String::new();
                if !stdout.is_empty() {
                    out.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !out.is_empty() {
                        out.push_str("\n--- stderr ---\n");
                    }
                    out.push_str(&stderr);
                }
                if out.is_empty() {
                    out.push_str("(no output)");
                }

                Ok(ToolResult {
                    output: out,
                    title: format!("bash: {}", truncate_title(command)),
                    metadata: json!({ "exit_code": exit_code }),
                })
            }
            Ok(Err(e)) => Err(anyhow::anyhow!("Failed to spawn command: {e}")),
            Err(_) => Ok(ToolResult {
                output: format!("Command timed out after {timeout_secs}s"),
                title: format!("bash (timeout): {}", truncate_title(command)),
                metadata: json!({ "exit_code": -1, "timeout": true }),
            }),
        }
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
