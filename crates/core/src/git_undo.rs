use std::path::Path;
use std::process::Command;

pub fn is_git_repo(project_root: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(project_root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn create_checkpoint(project_root: &Path, label: &str) -> Result<String, String> {
    let stash_msg = format!("nyzhi-checkpoint: {label}");
    let output = Command::new("git")
        .args(["stash", "push", "-u", "-m", &stash_msg])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.contains("No local changes") || stdout.contains("No stash entries") {
        return Ok("no-changes".to_string());
    }

    let _ = Command::new("git")
        .args(["stash", "pop"])
        .current_dir(project_root)
        .output();

    let hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("git rev-parse failed: {e}"))?;

    Ok(String::from_utf8_lossy(&hash.stdout).trim().to_string())
}

pub fn git_undo_file(project_root: &Path, file_path: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", "--", file_path])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("git checkout failed: {e}"))?;

    if output.status.success() {
        Ok(format!("Restored {file_path} from git"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git checkout failed: {stderr}"))
    }
}

pub fn git_undo_all(project_root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", "--", "."])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("git checkout failed: {e}"))?;

    let clean = Command::new("git")
        .args(["clean", "-fd"])
        .current_dir(project_root)
        .output();

    if output.status.success() {
        let mut msg = "Restored all tracked files from git.".to_string();
        if let Ok(c) = clean {
            if c.status.success() {
                let cleaned = String::from_utf8_lossy(&c.stdout);
                if !cleaned.trim().is_empty() {
                    msg.push_str(&format!("\nRemoved untracked files:\n{}", cleaned.trim()));
                }
            }
        }
        Ok(msg)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git checkout failed: {stderr}"))
    }
}

pub fn git_diff_stat(project_root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("git diff failed: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let untracked = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(project_root)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut result = stdout;
    if !untracked.trim().is_empty() {
        result.push_str("\nUntracked files:\n");
        for f in untracked.trim().lines() {
            result.push_str(&format!("  {f}\n"));
        }
    }
    Ok(result)
}

pub fn git_diff_full(project_root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["diff"])
        .current_dir(project_root)
        .output()
        .map_err(|e| format!("git diff failed: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
