use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use nyzhi_provider::Message;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: u32,
    pub label: String,
    pub timestamp: u64,
    pub message_count: usize,
    pub file_snapshots: HashMap<String, FileSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub path: String,
    pub content_hash: String,
    pub content: Option<String>,
}

pub struct CheckpointManager {
    checkpoints: Vec<Checkpoint>,
    messages_snapshot: Vec<Vec<u8>>,
    next_id: u32,
    storage_dir: PathBuf,
}

impl CheckpointManager {
    pub fn new(session_id: &str, project_root: &Path) -> Self {
        let storage_dir = project_root
            .join(".nyzhi")
            .join("checkpoints")
            .join(session_id);
        Self {
            checkpoints: Vec::new(),
            messages_snapshot: Vec::new(),
            next_id: 1,
            storage_dir,
        }
    }

    pub fn create(
        &mut self,
        label: &str,
        messages: &[Message],
        changed_files: &[PathBuf],
    ) -> Result<u32> {
        let id = self.next_id;
        self.next_id += 1;

        let mut file_snapshots = HashMap::new();
        for path in changed_files {
            if path.exists() {
                let content = std::fs::read_to_string(path).ok();
                let hash = content
                    .as_ref()
                    .map(|c| {
                        use sha2::{Digest, Sha256};
                        hex::encode(Sha256::digest(c.as_bytes()))
                    })
                    .unwrap_or_default();
                file_snapshots.insert(
                    path.to_string_lossy().to_string(),
                    FileSnapshot {
                        path: path.to_string_lossy().to_string(),
                        content_hash: hash,
                        content,
                    },
                );
            }
        }

        let serialized = serde_json::to_vec(messages).unwrap_or_default();

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let checkpoint = Checkpoint {
            id,
            label: label.to_string(),
            timestamp: ts,
            message_count: messages.len(),
            file_snapshots,
        };

        self.checkpoints.push(checkpoint);
        self.messages_snapshot.push(serialized);

        self.persist(id)?;
        Ok(id)
    }

    pub fn list(&self) -> Vec<&Checkpoint> {
        self.checkpoints.iter().collect()
    }

    pub fn restore(&mut self, id: u32) -> Result<Option<(Vec<Message>, Vec<(PathBuf, String)>)>> {
        let idx = match self.checkpoints.iter().position(|c| c.id == id) {
            Some(i) => i,
            None => return Ok(None),
        };

        let checkpoint = &self.checkpoints[idx];
        let messages: Vec<Message> =
            serde_json::from_slice(&self.messages_snapshot[idx]).unwrap_or_default();

        let mut file_restores = Vec::new();
        for (_path_str, snapshot) in &checkpoint.file_snapshots {
            if let Some(content) = &snapshot.content {
                file_restores.push((PathBuf::from(&snapshot.path), content.clone()));
            }
        }

        self.checkpoints.truncate(idx + 1);
        self.messages_snapshot.truncate(idx + 1);

        Ok(Some((messages, file_restores)))
    }

    pub fn latest_id(&self) -> Option<u32> {
        self.checkpoints.last().map(|c| c.id)
    }

    fn persist(&self, id: u32) -> Result<()> {
        std::fs::create_dir_all(&self.storage_dir)?;
        if let Some(cp) = self.checkpoints.iter().find(|c| c.id == id) {
            let path = self.storage_dir.join(format!("checkpoint-{id}.json"));
            let json = serde_json::to_string_pretty(cp)?;
            std::fs::write(path, json)?;
        }
        Ok(())
    }

    pub fn format_list(&self) -> String {
        if self.checkpoints.is_empty() {
            return "No checkpoints available.".to_string();
        }
        self.checkpoints
            .iter()
            .map(|c| {
                format!(
                    "  [{}] {} - {} messages, {} files (t={})",
                    c.id,
                    c.label,
                    c.message_count,
                    c.file_snapshots.len(),
                    c.timestamp
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
