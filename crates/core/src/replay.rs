use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub payload: String,
}

fn replay_dir() -> PathBuf {
    nyzhi_config::Config::data_dir().join("replay")
}

fn replay_path(session_id: &str) -> PathBuf {
    replay_dir().join(format!("{session_id}.jsonl"))
}

pub fn log_event(session_id: &str, event_type: &str, payload: &str) -> Result<()> {
    use std::io::Write;
    let dir = replay_dir();
    std::fs::create_dir_all(&dir)?;
    let path = replay_path(session_id);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let event = ReplayEvent {
        timestamp: ts,
        event_type: event_type.to_string(),
        payload: payload.to_string(),
    };

    let line = serde_json::to_string(&event)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub fn load_replay(session_id: &str) -> Result<Vec<ReplayEvent>> {
    let path = replay_path(session_id);
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(&path)?;
    let events: Vec<ReplayEvent> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(events)
}

pub fn list_replays() -> Result<Vec<String>> {
    let dir = replay_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut ids = vec![];
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if let Some(id) = name.strip_suffix(".jsonl") {
                ids.push(id.to_string());
            }
        }
    }
    ids.sort();
    Ok(ids)
}

pub fn format_replay(events: &[ReplayEvent], filter: Option<&str>) -> String {
    let filtered: Vec<&ReplayEvent> = if let Some(f) = filter {
        events.iter().filter(|e| e.event_type.contains(f)).collect()
    } else {
        events.iter().collect()
    };

    if filtered.is_empty() {
        return "No events found.".to_string();
    }

    let mut lines = vec![];
    for e in &filtered {
        let truncated = if e.payload.len() > 200 {
            format!("{}...", &e.payload[..200])
        } else {
            e.payload.clone()
        };
        lines.push(format!("[{}] {}: {}", e.timestamp, e.event_type, truncated));
    }
    lines.join("\n")
}
