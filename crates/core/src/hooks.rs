use std::path::Path;
use std::time::Duration;

use nyzhi_config::{HookConfig, HookEvent, HookType};
use tokio::process::Command;

pub struct HookResult {
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub hook_type: HookType,
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
                format!(
                    "...({} lines trimmed)\n{}",
                    lines.len() - 10,
                    tail.join("\n")
                )
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
        results.push(run_hook(hook, Some(&command), cwd, None).await);
    }
    results
}

pub async fn run_after_turn_hooks(hooks: &[HookConfig], cwd: &Path) -> Vec<HookResult> {
    let mut results = Vec::new();
    for hook in hooks {
        if hook.event != HookEvent::AfterTurn {
            continue;
        }
        results.push(run_hook(hook, None, cwd, None).await);
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
            let file = context.get("file").and_then(|v| v.as_str()).unwrap_or("");
            if !file.is_empty() && !matches_pattern(pattern, file) {
                continue;
            }
        }
        let stdin_json = serde_json::to_string(context).unwrap_or_default();
        results.push(run_hook(hook, None, cwd, Some(&stdin_json)).await);
    }
    results
}

/// Dispatch a hook based on its type (command, prompt, agent).
async fn run_hook(
    hook: &HookConfig,
    command_override: Option<&str>,
    cwd: &Path,
    stdin_data: Option<&str>,
) -> HookResult {
    let fallback_cmd = command_override.unwrap_or(&hook.command).trim();
    match hook.hook_type {
        HookType::Command => {
            let mut result = run_hook_command(fallback_cmd, hook.timeout, cwd, stdin_data).await;
            result.hook_type = HookType::Command;
            result
        }
        HookType::Prompt => {
            if !fallback_cmd.is_empty() {
                let mut result = run_hook_command(fallback_cmd, hook.timeout, cwd, stdin_data).await;
                result.hook_type = HookType::Prompt;
                return result;
            }
            HookResult {
                command: "prompt".to_string(),
                stdout: String::new(),
                stderr: "hook_type=prompt is not implemented yet; use hook_type=command or set a command fallback".to_string(),
                exit_code: Some(1),
                timed_out: false,
                hook_type: HookType::Prompt,
            }
        }
        HookType::Agent => {
            if !fallback_cmd.is_empty() {
                let mut result = run_hook_command(fallback_cmd, hook.timeout, cwd, stdin_data).await;
                result.hook_type = HookType::Agent;
                return result;
            }
            HookResult {
                command: "agent".to_string(),
                stdout: String::new(),
                stderr: "hook_type=agent is not implemented yet; use hook_type=command or set a command fallback".to_string(),
                exit_code: Some(1),
                timed_out: false,
                hook_type: HookType::Agent,
            }
        }
    }
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
    let results = run_hooks_for_event(hooks, HookEvent::PreToolUse, &context, cwd).await;
    let blocked = results
        .iter()
        .any(|r| r.exit_code.map(|c| c != 0).unwrap_or(false))
        && hooks
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

/// Run TeammateIdle hooks. If any hook exits with code 2, returns
/// `Some(feedback)` where feedback is stderr -- the teammate should keep working.
pub async fn run_teammate_idle_hooks(
    hooks: &[HookConfig],
    teammate_name: &str,
    team_name: &str,
    cwd: &Path,
) -> Option<String> {
    let context = serde_json::json!({
        "hook_event_name": "TeammateIdle",
        "teammate_name": teammate_name,
        "team_name": team_name,
    });
    let results = run_hooks_for_event(hooks, HookEvent::TeammateIdle, &context, cwd).await;
    for r in &results {
        if r.exit_code == Some(2) {
            return Some(r.stderr.clone());
        }
    }
    None
}

/// Run TaskCompleted hooks. If any hook exits with code 2, returns
/// `Some(feedback)` where feedback is stderr -- the task completion is rejected.
pub async fn run_task_completed_hooks(
    hooks: &[HookConfig],
    task_id: &str,
    task_subject: &str,
    teammate_name: &str,
    team_name: &str,
    cwd: &Path,
) -> Option<String> {
    let context = serde_json::json!({
        "hook_event_name": "TaskCompleted",
        "task_id": task_id,
        "task_subject": task_subject,
        "task_description": "",
        "teammate_name": teammate_name,
        "team_name": team_name,
    });
    let results = run_hooks_for_event(hooks, HookEvent::TaskCompleted, &context, cwd).await;
    for r in &results {
        if r.exit_code == Some(2) {
            return Some(r.stderr.clone());
        }
    }
    None
}

async fn run_hook_command(
    command: &str,
    timeout_secs: u64,
    cwd: &Path,
    stdin_data: Option<&str>,
) -> HookResult {
    #![allow(unused_variables)]
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
                hook_type: HookType::Command,
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

    match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await {
        Ok(Ok(output)) => HookResult {
            command: command.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            timed_out: false,
            hook_type: HookType::Command,
        },
        Ok(Err(e)) => HookResult {
            command: command.to_string(),
            stdout: String::new(),
            stderr: format!("Failed to run hook: {e}"),
            exit_code: None,
            timed_out: false,
            hook_type: HookType::Command,
        },
        Err(_) => HookResult {
            command: command.to_string(),
            stdout: String::new(),
            stderr: "Hook timed out".to_string(),
            exit_code: None,
            timed_out: true,
            hook_type: HookType::Command,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn make_hook(event: HookEvent, hook_type: HookType, command: &str) -> HookConfig {
        HookConfig {
            event,
            command: command.to_string(),
            hook_type,
            prompt: None,
            instructions: None,
            tools: None,
            model: None,
            pattern: None,
            tool_name: None,
            block: false,
            timeout: 5,
        }
    }

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

    #[tokio::test]
    async fn prompt_hook_without_command_fails_closed() {
        let hooks = vec![make_hook(HookEvent::AfterTurn, HookType::Prompt, "")];
        let results = run_after_turn_hooks(&hooks, Path::new(".")).await;
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.hook_type, HookType::Prompt);
        assert_eq!(r.exit_code, Some(1));
        assert!(r.stderr.contains("not implemented"));
    }

    #[tokio::test]
    async fn agent_hook_without_command_fails_closed() {
        let hooks = vec![make_hook(HookEvent::AfterTurn, HookType::Agent, "")];
        let results = run_after_turn_hooks(&hooks, Path::new(".")).await;
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.hook_type, HookType::Agent);
        assert_eq!(r.exit_code, Some(1));
        assert!(r.stderr.contains("not implemented"));
    }

    #[tokio::test]
    async fn prompt_hook_with_command_uses_command_fallback() {
        let hooks = vec![make_hook(HookEvent::AfterTurn, HookType::Prompt, "printf ok")];
        let results = run_after_turn_hooks(&hooks, Path::new(".")).await;
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert_eq!(r.hook_type, HookType::Prompt);
        assert_eq!(r.exit_code, Some(0));
        assert!(r.stdout.contains("ok"));
    }
}
