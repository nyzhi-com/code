use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct FileChange {
    pub path: PathBuf,
    /// None = file was newly created (undo deletes it)
    pub original: Option<String>,
    pub new_content: String,
    pub tool_name: String,
    pub timestamp: DateTime<Utc>,
}

pub struct ChangeTracker {
    changes: Vec<FileChange>,
}

impl ChangeTracker {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    pub fn record(&mut self, change: FileChange) {
        self.changes.push(change);
    }

    pub fn changed_files(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self.changes.iter().map(|c| c.path.clone()).collect();
        paths.sort();
        paths.dedup();
        paths
    }

    /// Pop the most recent change and restore the file to its original state.
    /// Returns the reverted change, or None if the stack is empty.
    pub async fn undo_last(&mut self) -> Result<Option<FileChange>> {
        let change = match self.changes.pop() {
            Some(c) => c,
            None => return Ok(None),
        };
        restore(&change).await?;
        Ok(Some(change))
    }

    /// Undo all changes in reverse order.
    pub async fn undo_all(&mut self) -> Result<Vec<FileChange>> {
        let mut reverted = Vec::new();
        while let Some(change) = self.changes.pop() {
            restore(&change).await?;
            reverted.push(change);
        }
        Ok(reverted)
    }

    pub fn changes(&self) -> &[FileChange] {
        &self.changes
    }

    pub fn last(&self) -> Option<&FileChange> {
        self.changes.last()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

async fn restore(change: &FileChange) -> Result<()> {
    match &change.original {
        Some(content) => {
            tokio::fs::write(&change.path, content).await?;
        }
        None => {
            // File was created by the tool; remove it
            if change.path.exists() {
                let _ = tokio::fs::remove_file(&change.path).await;
            }
        }
    }
    Ok(())
}
