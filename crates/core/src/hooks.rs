use std::path::Path;
use std::time::Duration;

use nyzhi_config::{HookConfig, HookEvent};
use tokio::process::Command;

pub struct HookResult {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

impl HookResult {
    pub fn summary(&self) -> String {
        let status = if self.timed_out {
            "timed out".to_string()
        } else if let Some(code) = self.exit_code {
            if code == 0 {
                "ok".to_string()
            } else {
                format!("exit {code}")
            }
        } else {
            "killed".to_string()
        };

        let mut out = format!("[hook] {} ({})", self.command, status);
        let combined = if !self.stdout.is_empty() && !self.stderr.is_empty() {
            format!("{}\n{}", self.stdout.trim(), self.stderr.trim())
        } else if !self.stdout.is_empty() {
            self.stdout.trim().to_string()
        } else {
            self.stderr.trim().to_string()
        };
        if !combined.is_empty() {
            let lines: Vec<&str> = combined.lines().collect();
            let display: String = if lines.len() > 10 {
                let tail = &lines[lines.len() - 10..];
                format!("...({} lines trimmed)\n{}", lines.len() - 10, tail.join("\n"))
            } else {
                combined
            };
            out.push('\n');
            out.push_str(&display);
        }
        out
    }
}

fn matches_pattern(pattern: &str, path: &str) -> bool {
    let pat = pattern.trim();
    if pat.is_empty() {
        return true;
    }
    for single in pat.split(',') {
        let single = single.trim();
        if single.is_empty() {
            continue;
        }
        if single.starts_with("*.") {
            let ext = &single[1..];
            if path.ends_with(ext) {
                return true;
            }
        } else if path.contains(single) {
            return true;
        }
    }
    false
}

pub async fn run_after_edit_hooks(
    hooks: &[HookConfig],
    changed_file: &str,
    cwd: &Path,
) -> Vec<HookResult> {
    let mut results = Vec::new();
    for hook in hooks {
        if hook.event != HookEvent::AfterEdit {
            continue;
        }
        if let Some(ref pattern) = hook.pattern {
            if !matches_pattern(pattern, changed_file) {
                continue;
            }
        }
        let command = hook.command.replace("{file}", changed_file);
        results.push(run_hook_command(&command, hook.timeout, cwd, None).await);
    }
    results
}

pub async fn run_after_turn_hooks(hooks: &[HookConfig], cwd: &Path) -> Vec<HookResult> {
    let mut results = Vec::new();
    for hook in hooks {
        if hook.event != HookEvent::AfterTurn {
            continue;
        }
        results.push(run_hook_command(&hook.command, hook.timeout, cwd, None).await);
    }
    results
}

pub async fn run_hooks_for_event(
    hooks: &[HookConfig],
    event: HookEvent,
    context: &serde_json::Value,
    cwd: &Path,
) -> Vec<HookResult> {
    let mut results = Vec::new();
    for hook in hooks {
        if hook.event != event {
            continue;
        }
        if let Some(ref tool_name_filter) = hook.tool_name {
            let ctx_tool = context
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !tool_name_filter
                .split(',')
                .any(|t| t.trim().eq_ignore_ascii_case(ctx_tool))
            {
                continue;
            }
        }
        if let Some(ref pattern) = hook.pattern {
            let file = context
                .get("file")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !file.is_empty() && !matches_pattern(pattern, file) {
                continue;
            }
        }
        let stdin_json = serde_json::to_string(context).unwrap_or_default();
        results.push(
            run_hook_command(&hook.command, hook.timeout, cwd, Some(&stdin_json)).await,
        );
    }
    results
}

/// Returns true if the tool should be blocked (any blocking PreToolUse hook returned non-zero).
pub async fn run_pre_tool_hooks(
    hooks: &[HookConfig],
    tool_name: &str,
    tool_args: &serde_json::Value,
    cwd: &Path,
) -> (Vec<HookResult>, bool) {
    let context = serde_json::json!({
        "tool_name": tool_name,
        "tool_args": tool_args,
    });
    let results =
        run_hooks_for_event(hooks, HookEvent::PreToolUse, &context, cwd).await;
    let blocked = results.iter().any(|r| {
        r.exit_code.map(|c| c != 0).unwrap_or(false)
    }) && hooks
        .iter()
        .any(|h| h.event == HookEvent::PreToolUse && h.block);
    (results, blocked)
}

pub async fn run_post_tool_hooks(
    hooks: &[HookConfig],
    tool_name: &str,
    tool_args: &serde_json::Value,
    output: &str,
    success: bool,
    cwd: &Path,
) -> Vec<HookResult> {
    let event = if success {
        HookEvent::PostToolUse
    } else {
        HookEvent::PostToolUseFailure
    };
    let context = serde_json::json!({
        "tool_name": tool_name,
        "tool_args": tool_args,
        "output": output,
        "success": success,
    });
    run_hooks_for_event(hooks, event, &context, cwd).await
}

async fn run_hook_command(
    command: &str,
    timeout_secs: u64,
    cwd: &Path,
    stdin_data: Option<&str>,
) -> HookResult {
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdin(if stdin_data.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return HookResult {
                command: command.to_string(),
                stdout: String::new(),
                stderr: format!("Failed to spawn hook: {e}"),
                exit_code: None,
                timed_out: false,
            };
        }
    };

    if let Some(data) = stdin_data {
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(data.as_bytes()).await;
            let _ = stdin.shutdown().await;
        }
    }

    match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        child.wait_with_output(),
    )
    .await
    {
        Ok(Ok(output)) => HookResult {
            command: command.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            timed_out: false,
        },
        Ok(Err(e)) => HookResult {
            command: command.to_string(),
            stdout: String::new(),
            stderr: format!("Failed to run hook: {e}"),
            exit_code: None,
            timed_out: false,
        },
        Err(_) => HookResult {
            command: command.to_string(),
            stdout: String::new(),
            stderr: "Hook timed out".to_string(),
            exit_code: None,
            timed_out: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_matches_extension() {
        assert!(matches_pattern("*.rs", "src/main.rs"));
        assert!(matches_pattern("*.rs", "crates/core/lib.rs"));
        assert!(!matches_pattern("*.rs", "package.json"));
    }

    #[test]
    fn pattern_matches_multiple() {
        assert!(matches_pattern("*.ts, *.tsx", "app/page.tsx"));
        assert!(matches_pattern("*.ts, *.tsx", "utils/helper.ts"));
        assert!(!matches_pattern("*.ts, *.tsx", "style.css"));
    }

    #[test]
    fn pattern_matches_substring() {
        assert!(matches_pattern("src/", "src/main.rs"));
        assert!(!matches_pattern("src/", "tests/test.rs"));
    }

    #[test]
    fn empty_pattern_matches_all() {
        assert!(matches_pattern("", "anything.rs"));
    }
}
