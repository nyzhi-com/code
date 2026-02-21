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
        results.push(run_hook_command(&command, hook.timeout, cwd).await);
    }
    results
}

pub async fn run_after_turn_hooks(hooks: &[HookConfig], cwd: &Path) -> Vec<HookResult> {
    let mut results = Vec::new();
    for hook in hooks {
        if hook.event != HookEvent::AfterTurn {
            continue;
        }
        results.push(run_hook_command(&hook.command, hook.timeout, cwd).await);
    }
    results
}

async fn run_hook_command(command: &str, timeout_secs: u64, cwd: &Path) -> HookResult {
    let result = tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .output(),
    )
    .await;

    match result {
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
