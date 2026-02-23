use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use super::team_tasks_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamTask {
    pub id: String,
    pub subject: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_form: Option<String>,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default, rename = "blockedBy")]
    pub blocked_by: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
    Deleted,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::Deleted => write!(f, "deleted"),
        }
    }
}

fn next_id(task_dir: &std::path::Path) -> Result<String> {
    let hwm_path = task_dir.join(".highwatermark");
    let current: u64 = if hwm_path.exists() {
        std::fs::read_to_string(&hwm_path)?
            .trim()
            .parse()
            .unwrap_or(0)
    } else {
        0
    };
    let next = current + 1;
    std::fs::write(&hwm_path, next.to_string())?;
    Ok(next.to_string())
}

fn acquire_flock(task_dir: &std::path::Path) -> Result<std::fs::File> {
    let lock_path = task_dir.join(".lock");
    std::fs::create_dir_all(task_dir)?;
    if !lock_path.exists() {
        std::fs::write(&lock_path, "")?;
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock_path)?;
    file.lock_exclusive()
        .context("Failed to acquire task lock")?;
    Ok(file)
}

impl TeamTask {
    pub fn create(
        team_name: &str,
        subject: &str,
        description: Option<&str>,
        active_form: Option<&str>,
        blocked_by: Vec<String>,
    ) -> Result<Self> {
        let task_dir = team_tasks_dir(team_name);
        std::fs::create_dir_all(&task_dir)?;

        let lock_file = acquire_flock(&task_dir)?;
        let id = next_id(&task_dir)?;

        let now = chrono::Utc::now().to_rfc3339();
        let status = if blocked_by.is_empty() {
            TaskStatus::Pending
        } else {
            TaskStatus::Blocked
        };

        let task = Self {
            id: id.clone(),
            subject: subject.to_string(),
            description: description.map(String::from),
            active_form: active_form.map(String::from),
            status,
            owner: None,
            blocks: vec![],
            blocked_by,
            created_at: now.clone(),
            updated_at: now,
        };

        let json = serde_json::to_string_pretty(&task)?;
        std::fs::write(task_dir.join(format!("{id}.json")), json)?;

        drop(lock_file);
        Ok(task)
    }

    pub fn load(team_name: &str, task_id: &str) -> Result<Self> {
        let path = team_tasks_dir(team_name).join(format!("{task_id}.json"));
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Task '{task_id}' not found"))?;
        serde_json::from_str(&content).context("Failed to parse task")
    }

    pub fn update(
        team_name: &str,
        task_id: &str,
        status: Option<TaskStatus>,
        owner: Option<String>,
    ) -> Result<Self> {
        let task_dir = team_tasks_dir(team_name);
        let lock_file = acquire_flock(&task_dir)?;

        let mut task = Self::load(team_name, task_id)?;
        if let Some(s) = status {
            task.status = s;
        }
        if let Some(o) = owner {
            task.owner = Some(o);
        }
        task.updated_at = chrono::Utc::now().to_rfc3339();

        let json = serde_json::to_string_pretty(&task)?;
        std::fs::write(task_dir.join(format!("{task_id}.json")), json)?;

        if task.status == TaskStatus::Completed {
            unblock_dependents(team_name, task_id)?;
        }

        drop(lock_file);
        Ok(task)
    }
}

pub fn list_tasks(team_name: &str, filter: Option<&str>) -> Result<Vec<TeamTask>> {
    let task_dir = team_tasks_dir(team_name);
    if !task_dir.exists() {
        return Ok(vec![]);
    }

    let mut tasks = Vec::new();
    for entry in std::fs::read_dir(&task_dir)?.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.ends_with(".json") {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Ok(task) = serde_json::from_str::<TeamTask>(&content) {
                    if let Some(f) = filter {
                        if task.status.to_string() != f {
                            continue;
                        }
                    }
                    tasks.push(task);
                }
            }
        }
    }
    tasks.sort_by(|a, b| {
        a.id.parse::<u64>()
            .unwrap_or(0)
            .cmp(&b.id.parse::<u64>().unwrap_or(0))
    });
    Ok(tasks)
}

fn unblock_dependents(team_name: &str, completed_task_id: &str) -> Result<()> {
    let task_dir = team_tasks_dir(team_name);
    for entry in std::fs::read_dir(&task_dir)?.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.ends_with(".json") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if let Ok(mut task) = serde_json::from_str::<TeamTask>(&content) {
                if task.blocked_by.contains(&completed_task_id.to_string()) {
                    task.blocked_by.retain(|id| id != completed_task_id);
                    if task.blocked_by.is_empty() && task.status == TaskStatus::Blocked {
                        task.status = TaskStatus::Pending;
                    }
                    task.updated_at = chrono::Utc::now().to_rfc3339();
                    let json = serde_json::to_string_pretty(&task)?;
                    std::fs::write(entry.path(), json)?;
                }
            }
        }
    }
    Ok(())
}
