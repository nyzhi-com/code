use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTodo {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFrontmatter {
    pub name: String,
    #[serde(default)]
    pub overview: String,
    #[serde(default)]
    pub todos: Vec<PlanTodo>,
}

#[derive(Debug, Clone)]
pub struct PlanFile {
    pub frontmatter: PlanFrontmatter,
    pub body: String,
}

impl PlanFile {
    pub fn progress(&self) -> (usize, usize) {
        let total = self.frontmatter.todos.len();
        let done = self
            .frontmatter
            .todos
            .iter()
            .filter(|t| matches!(t.status, TodoStatus::Completed | TodoStatus::Cancelled))
            .count();
        (done, total)
    }
}

fn plans_dir(project_root: &Path) -> PathBuf {
    project_root.join(".nyzhi").join("plans")
}

pub fn parse_plan(raw: &str) -> Result<PlanFile> {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return Ok(PlanFile {
            frontmatter: PlanFrontmatter {
                name: "Untitled".to_string(),
                overview: String::new(),
                todos: vec![],
            },
            body: raw.to_string(),
        });
    }

    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("Missing closing --- in frontmatter"))?;

    let yaml_str = &after_first[..end];
    let body_start = 3 + end + 4; // "---" + yaml + "\n---"
    let body = trimmed[body_start..].trim_start_matches('\n').to_string();

    let frontmatter: PlanFrontmatter = serde_yaml::from_str(yaml_str)?;
    Ok(PlanFile { frontmatter, body })
}

pub fn serialize_plan(plan: &PlanFile) -> String {
    let yaml = serde_yaml::to_string(&plan.frontmatter).unwrap_or_default();
    format!("---\n{}---\n\n{}", yaml, plan.body)
}

pub fn load_session_plan(project_root: &Path, session_id: &str) -> Result<Option<PlanFile>> {
    let path = plans_dir(project_root).join(format!("{session_id}.plan.md"));
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(Some(parse_plan(&raw)?))
}

pub fn save_session_plan(project_root: &Path, session_id: &str, plan: &PlanFile) -> Result<PathBuf> {
    let dir = plans_dir(project_root);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{session_id}.plan.md"));
    std::fs::write(&path, serialize_plan(plan))?;
    Ok(path)
}

pub fn load_plan(project_root: &Path, name: &str) -> Result<Option<String>> {
    let dir = plans_dir(project_root);
    let safe_name: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let path = dir.join(format!("{safe_name}.md"));
    if path.exists() {
        Ok(Some(std::fs::read_to_string(&path)?))
    } else {
        let plan_path = dir.join(format!("{safe_name}.plan.md"));
        if plan_path.exists() {
            Ok(Some(std::fs::read_to_string(&plan_path)?))
        } else {
            Ok(None)
        }
    }
}

pub fn list_plans(project_root: &Path) -> Result<Vec<String>> {
    let dir = plans_dir(project_root);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names = vec![];
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if let Some(stem) = name.strip_suffix(".plan.md") {
                names.push(stem.to_string());
            } else if let Some(stem) = name.strip_suffix(".md") {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}
