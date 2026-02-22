use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use super::team_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMessage {
    pub from: String,
    pub text: String,
    pub timestamp: String,
    #[serde(default)]
    pub read: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Message,
    Broadcast,
    TaskAssignment,
    ShutdownRequest,
    ShutdownResponse,
    PlanApprovalRequest,
    PlanApprovalResponse,
    TaskCompleted,
    IdleNotification,
}

/// The `text` field in TeamMessage can contain a JSON-encoded payload with a `type` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

impl TeamMessage {
    pub fn new(from: &str, text: &str, color: Option<&str>) -> Self {
        Self {
            from: from.to_string(),
            text: text.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
            color: color.map(String::from),
        }
    }

    pub fn with_payload(from: &str, payload: &MessagePayload, color: Option<&str>) -> Self {
        Self {
            from: from.to_string(),
            text: serde_json::to_string(payload).unwrap_or_default(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            read: false,
            color: color.map(String::from),
        }
    }
}

fn acquire_inbox_lock(inbox_path: &std::path::Path) -> Result<std::fs::File> {
    let lock_path = inbox_path.with_extension("lock");
    if !lock_path.exists() {
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&lock_path, "")?;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)?;
    file.lock_exclusive()
        .context("Failed to acquire inbox lock")?;
    Ok(file)
}

pub fn send_message(team_name: &str, to: &str, msg: TeamMessage) -> Result<()> {
    let inbox_path = team_dir(team_name)
        .join("inboxes")
        .join(format!("{to}.json"));

    let lock_file = acquire_inbox_lock(&inbox_path)?;

    let mut messages = load_inbox_raw(&inbox_path)?;
    messages.push(msg);
    let json = serde_json::to_string_pretty(&messages)?;
    std::fs::write(&inbox_path, json)?;

    drop(lock_file);
    Ok(())
}

pub fn broadcast(team_name: &str, from: &str, text: &str, color: Option<&str>) -> Result<()> {
    let config = super::config::TeamConfig::load(team_name)?;
    for member in &config.members {
        if member.name != from {
            let msg = TeamMessage::new(from, text, color);
            send_message(team_name, &member.name, msg)?;
        }
    }
    Ok(())
}

pub fn read_unread(team_name: &str, agent_name: &str) -> Result<Vec<TeamMessage>> {
    let inbox_path = team_dir(team_name)
        .join("inboxes")
        .join(format!("{agent_name}.json"));

    if !inbox_path.exists() {
        return Ok(vec![]);
    }

    let lock_file = acquire_inbox_lock(&inbox_path)?;

    let mut messages = load_inbox_raw(&inbox_path)?;
    let unread: Vec<TeamMessage> = messages
        .iter()
        .filter(|m| !m.read)
        .cloned()
        .collect();

    for msg in messages.iter_mut() {
        msg.read = true;
    }
    let json = serde_json::to_string_pretty(&messages)?;
    std::fs::write(&inbox_path, json)?;

    drop(lock_file);
    Ok(unread)
}

/// Format unread messages as XML blocks for injection into the agent's conversation.
pub fn format_messages_for_injection(messages: &[TeamMessage]) -> String {
    if messages.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for msg in messages {
        let color_attr = msg
            .color
            .as_deref()
            .map(|c| format!(" color=\"{c}\""))
            .unwrap_or_default();
        out.push_str(&format!(
            "<teammate_message from=\"{}\"{}>{}</teammate_message>\n",
            msg.from, color_attr, msg.text
        ));
    }
    out
}

fn load_inbox_raw(path: &std::path::Path) -> Result<Vec<TeamMessage>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read inbox at {}", path.display()))?;
    if content.trim().is_empty() || content.trim() == "[]" {
        return Ok(vec![]);
    }
    serde_json::from_str(&content).context("Failed to parse inbox")
}
