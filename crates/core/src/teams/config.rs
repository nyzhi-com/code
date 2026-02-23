use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{team_dir, team_tasks_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub name: String,
    pub members: Vec<TeamMemberConfig>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMemberConfig {
    pub name: String,
    #[serde(rename = "agentId")]
    pub agent_id: Option<String>,
    #[serde(rename = "agentType")]
    pub agent_type: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_path: Option<String>,
}

impl TeamConfig {
    pub fn create(name: &str, members: Vec<TeamMemberConfig>) -> Result<Self> {
        let dir = team_dir(name);
        std::fs::create_dir_all(dir.join("inboxes"))?;

        let task_dir = team_tasks_dir(name);
        std::fs::create_dir_all(&task_dir)?;
        // Initialize highwatermark for auto-incrementing task IDs
        let hwm_path = task_dir.join(".highwatermark");
        if !hwm_path.exists() {
            std::fs::write(&hwm_path, "0")?;
        }
        // Create lock file for task claiming
        let lock_path = task_dir.join(".lock");
        if !lock_path.exists() {
            std::fs::write(&lock_path, "")?;
        }

        let config = Self {
            name: name.to_string(),
            members,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(dir.join("config.json"), json)?;

        for member in &config.members {
            let inbox_path = dir.join("inboxes").join(format!("{}.json", member.name));
            if !inbox_path.exists() {
                std::fs::write(&inbox_path, "[]")?;
            }
        }

        Ok(config)
    }

    pub fn load(name: &str) -> Result<Self> {
        let path = team_dir(name).join("config.json");
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("Team '{}' not found", name))?;
        serde_json::from_str(&content).context("Failed to parse team config")
    }

    pub fn delete(name: &str) -> Result<()> {
        let dir = team_dir(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        let task_dir = team_tasks_dir(name);
        if task_dir.exists() {
            std::fs::remove_dir_all(&task_dir)?;
        }
        Ok(())
    }

    pub fn add_member(&mut self, member: TeamMemberConfig) -> Result<()> {
        let dir = team_dir(&self.name);
        let inbox_path = dir.join("inboxes").join(format!("{}.json", member.name));
        if !inbox_path.exists() {
            std::fs::write(&inbox_path, "[]")?;
        }
        self.members.push(member);
        self.save()
    }

    pub fn lead(&self) -> Option<&TeamMemberConfig> {
        self.members.iter().find(|m| m.agent_type == "leader")
    }

    pub fn lead_name(&self) -> String {
        self.lead()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "team-lead".to_string())
    }

    fn save(&self) -> Result<()> {
        let dir = team_dir(&self.name);
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(dir.join("config.json"), json)?;
        Ok(())
    }
}

const TEAM_COLORS: &[&str] = &[
    "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#d19a66", "#be5046",
    "#7ec699", "#f8c555",
];

pub fn assign_color(index: usize) -> String {
    TEAM_COLORS[index % TEAM_COLORS.len()].to_string()
}
