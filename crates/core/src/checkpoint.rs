use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub name: String,
    pub commit_hash: String,
    pub message_count: usize,
    pub timestamp: u64,
}

pub struct CheckpointManager {
    project_root: std::path::PathBuf,
    checkpoints: Vec<Checkpoint>,
    #[allow(dead_code)]
    branch_name: String,
}

impl CheckpointManager {
    pub fn new(project_root: &Path, session_id: &str) -> Self {
        let branch_name = format!("nyzhi/checkpoints/{}", &session_id[..8.min(session_id.len())]);
        Self {
            project_root: project_root.to_path_buf(),
            checkpoints: Vec::new(),
            branch_name,
        }
    }

    fn is_git_repo(&self) -> bool {
        Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(&self.project_root)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn has_changes(&self) -> bool {
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.project_root)
            .output()
            .ok();
        status
            .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
            .unwrap_or(false)
    }

    pub fn save(&mut self, name: &str, message_count: usize) -> Result<Option<Checkpoint>, String> {
        if !self.is_git_repo() {
            return Err("Not a git repository".into());
        }
        if !self.has_changes() {
            return Ok(None);
        }

        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.project_root)
            .output();

        let stash_msg = format!("nyzhi-cp: {name}");
        let stash_out = Command::new("git")
            .args(["stash", "push", "-u", "-m", &stash_msg])
            .current_dir(&self.project_root)
            .output()
            .map_err(|e| format!("git stash failed: {e}"))?;

        let stash_text = String::from_utf8_lossy(&stash_out.stdout);
        if stash_text.contains("No local changes") {
            return Ok(None);
        }

        let hash_out = Command::new("git")
            .args(["stash", "list", "--format=%H", "-1"])
            .current_dir(&self.project_root)
            .output()
            .map_err(|e| format!("git stash list failed: {e}"))?;
        let commit_hash = String::from_utf8_lossy(&hash_out.stdout).trim().to_string();

        let _ = Command::new("git")
            .args(["stash", "pop"])
            .current_dir(&self.project_root)
            .output();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let id = format!("cp-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let cp = Checkpoint {
            id: id.clone(),
            name: name.to_string(),
            commit_hash,
            message_count,
            timestamp: now,
        };
        self.checkpoints.push(cp.clone());
        Ok(Some(cp))
    }

    pub fn auto_save(&mut self, message_count: usize) -> Option<Checkpoint> {
        let name = format!("auto-{}", self.checkpoints.len());
        self.save(&name, message_count).ok().flatten()
    }

    pub fn list(&self) -> &[Checkpoint] {
        &self.checkpoints
    }

    pub fn restore(&self, id: &str) -> Result<String, String> {
        if !self.is_git_repo() {
            return Err("Not a git repository".into());
        }

        let cp = self
            .checkpoints
            .iter()
            .find(|c| c.id == id || c.name == id)
            .ok_or_else(|| format!("Checkpoint '{id}' not found"))?;

        let _ = Command::new("git")
            .args(["checkout", "--", "."])
            .current_dir(&self.project_root)
            .output();
        let _ = Command::new("git")
            .args(["clean", "-fd"])
            .current_dir(&self.project_root)
            .output();

        if !cp.commit_hash.is_empty() {
            let _ = Command::new("git")
                .args(["stash", "apply", &cp.commit_hash])
                .current_dir(&self.project_root)
                .output();
        }

        Ok(format!("Restored checkpoint '{}' ({})", cp.name, cp.id))
    }

    pub fn format_list(&self) -> String {
        if self.checkpoints.is_empty() {
            return "No checkpoints saved.".to_string();
        }
        let mut out = String::from("Checkpoints:\n");
        for (i, cp) in self.checkpoints.iter().enumerate().rev() {
            let age = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(cp.timestamp);
            let age_str = if age < 60 {
                format!("{age}s ago")
            } else if age < 3600 {
                format!("{}m ago", age / 60)
            } else {
                format!("{}h ago", age / 3600)
            };
            out.push_str(&format!(
                "  [{i}] {} ({}) - {} messages, {}\n",
                cp.name, cp.id, cp.message_count, age_str
            ));
        }
        out
    }
}
