use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const WORKTREE_DIR: &str = ".nyzhi/worktrees";

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub has_changes: bool,
}

static ADJECTIVES: &[&str] = &[
    "bold", "calm", "dark", "fast", "keen", "pure", "warm", "wise", "cool", "deep",
];
static NOUNS: &[&str] = &[
    "arch", "beam", "core", "edge", "flux", "grid", "helm", "iris", "jade", "knot",
];

fn generate_name() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let adj = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.random_range(0..NOUNS.len())];
    format!("{adj}-{noun}")
}

/// Create a git worktree for isolated agent work.
pub fn create_worktree(project_root: &Path, name: Option<&str>) -> Result<WorktreeInfo> {
    let worktree_name = name
        .map(String::from)
        .unwrap_or_else(generate_name);
    let worktree_path = project_root.join(WORKTREE_DIR).join(&worktree_name);
    let branch = format!("worktree-{worktree_name}");

    ensure_gitignore(project_root)?;

    let output = std::process::Command::new("git")
        .args(["worktree", "add", "-b", &branch])
        .arg(&worktree_path)
        .current_dir(project_root)
        .output()
        .context("Failed to run git worktree add")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already exists") {
            let output2 = std::process::Command::new("git")
                .args(["worktree", "add"])
                .arg(&worktree_path)
                .arg(&branch)
                .current_dir(project_root)
                .output()?;
            if !output2.status.success() {
                anyhow::bail!(
                    "Failed to create worktree: {}",
                    String::from_utf8_lossy(&output2.stderr)
                );
            }
        } else {
            anyhow::bail!("Failed to create worktree: {}", stderr);
        }
    }

    Ok(WorktreeInfo {
        name: worktree_name,
        path: worktree_path,
        branch,
        has_changes: false,
    })
}

/// Remove a worktree. Returns whether it had uncommitted changes.
pub fn remove_worktree(project_root: &Path, name: &str, force: bool) -> Result<bool> {
    let worktree_path = project_root.join(WORKTREE_DIR).join(name);

    let has_changes = if worktree_path.exists() {
        let output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&worktree_path)
            .output();
        output
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false)
    } else {
        false
    };

    if has_changes && !force {
        return Ok(true);
    }

    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(name);

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(project_root)
        .output();

    match output {
        Ok(o) if o.status.success() => {}
        _ => {
            if worktree_path.exists() {
                std::fs::remove_dir_all(&worktree_path).ok();
            }
            let _ = std::process::Command::new("git")
                .args(["worktree", "prune"])
                .current_dir(project_root)
                .output();
        }
    }

    let branch = format!("worktree-{name}");
    let _ = std::process::Command::new("git")
        .args(["branch", "-D", &branch])
        .current_dir(project_root)
        .output();

    Ok(has_changes)
}

/// List active worktrees.
pub fn list_worktrees(project_root: &Path) -> Vec<WorktreeInfo> {
    let dir = project_root.join(WORKTREE_DIR);
    if !dir.exists() {
        return vec![];
    }

    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let branch = format!("worktree-{name}");
            let has_changes = std::process::Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(&path)
                .output()
                .map(|o| !o.stdout.is_empty())
                .unwrap_or(false);

            result.push(WorktreeInfo {
                name,
                path,
                branch,
                has_changes,
            });
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

fn ensure_gitignore(project_root: &Path) -> Result<()> {
    let gitignore = project_root.join(".gitignore");
    let pattern = ".nyzhi/worktrees/";

    if gitignore.exists() {
        let content = std::fs::read_to_string(&gitignore)?;
        if content.contains(pattern) {
            return Ok(());
        }
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&gitignore)?;
        use std::io::Write;
        writeln!(file, "\n# nyzhi worktrees\n{pattern}")?;
    } else {
        std::fs::write(&gitignore, format!("# nyzhi worktrees\n{pattern}\n"))?;
    }
    Ok(())
}
