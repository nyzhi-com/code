use std::path::PathBuf;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::conversation::Thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub provider: String,
    pub model: String,
}

#[derive(Serialize, Deserialize)]
struct SessionFile {
    meta: SessionMeta,
    thread: Thread,
}

fn sessions_dir() -> Result<PathBuf> {
    let data_dir = nyzhi_config::Config::data_dir();
    let sessions = data_dir.join("sessions");
    std::fs::create_dir_all(&sessions)?;
    Ok(sessions)
}

fn session_path(id: &str) -> Result<PathBuf> {
    Ok(sessions_dir()?.join(format!("{id}.json")))
}

pub fn save_session(
    thread: &Thread,
    provider: &str,
    model: &str,
) -> Result<SessionMeta> {
    let title = thread
        .messages()
        .iter()
        .find(|m| m.role == nyzhi_provider::Role::User)
        .map(|m| {
            let text = m.content.as_text();
            if text.len() > 80 {
                format!("{}...", &text[..77])
            } else {
                text.to_string()
            }
        })
        .unwrap_or_else(|| "untitled".to_string());

    let meta = SessionMeta {
        id: thread.id.clone(),
        title,
        created_at: thread.created_at,
        updated_at: Utc::now(),
        message_count: thread.message_count(),
        provider: provider.to_string(),
        model: model.to_string(),
    };

    let file = SessionFile {
        meta: meta.clone(),
        thread: thread.clone(),
    };

    let path = session_path(&meta.id)?;
    let json = serde_json::to_string(&file)?;
    std::fs::write(path, json)?;

    Ok(meta)
}

pub fn load_session(id: &str) -> Result<(Thread, SessionMeta)> {
    let path = session_path(id)?;
    let json = std::fs::read_to_string(path)?;
    let file: SessionFile = serde_json::from_str(&json)?;
    Ok((file.thread, file.meta))
}

pub fn list_sessions() -> Result<Vec<SessionMeta>> {
    let dir = sessions_dir()?;
    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json = match std::fs::read_to_string(&path) {
            Ok(j) => j,
            Err(_) => continue,
        };
        let file: SessionFile = match serde_json::from_str(&json) {
            Ok(f) => f,
            Err(_) => continue,
        };
        sessions.push(file.meta);
    }

    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(sessions)
}

pub fn delete_session(id: &str) -> Result<()> {
    let path = session_path(id)?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
