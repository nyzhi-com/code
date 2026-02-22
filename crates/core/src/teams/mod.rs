pub mod config;
pub mod tasks;
pub mod mailbox;

use std::path::PathBuf;

fn nyzhi_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".nyzhi")
}

pub fn teams_dir() -> PathBuf {
    nyzhi_home().join("teams")
}

pub fn team_dir(team_name: &str) -> PathBuf {
    teams_dir().join(team_name)
}

/// Tasks live at a separate top-level path: ~/.nyzhi/tasks/{team-name}/
pub fn tasks_dir() -> PathBuf {
    nyzhi_home().join("tasks")
}

pub fn team_tasks_dir(team_name: &str) -> PathBuf {
    tasks_dir().join(team_name)
}

pub fn list_teams() -> Vec<String> {
    let dir = teams_dir();
    if !dir.exists() {
        return vec![];
    }
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if team_dir(name).join("config.json").exists() {
                        names.push(name.to_string());
                    }
                }
            }
        }
    }
    names.sort();
    names
}
