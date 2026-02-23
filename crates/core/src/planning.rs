use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub name: String,
    pub rounds: Vec<PlanRound>,
    pub final_plan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRound {
    pub round: u32,
    pub planner_output: String,
    pub critic_output: String,
}

pub struct PlanConfig {
    pub max_rounds: u32,
}

impl Default for PlanConfig {
    fn default() -> Self {
        Self { max_rounds: 3 }
    }
}

fn plans_dir(project_root: &Path) -> PathBuf {
    project_root.join(".nyzhi").join("plans")
}

pub fn save_plan(project_root: &Path, plan: &Plan) -> Result<PathBuf> {
    let dir = plans_dir(project_root);
    std::fs::create_dir_all(&dir)?;

    let safe_name: String = plan
        .name
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

    let mut content = format!("# Plan: {}\n\n", plan.name);
    for round in &plan.rounds {
        content.push_str(&format!("## Round {}\n\n", round.round));
        content.push_str(&format!("### Planner\n{}\n\n", round.planner_output));
        content.push_str(&format!("### Critic\n{}\n\n", round.critic_output));
    }
    content.push_str(&format!("## Final Plan\n\n{}", plan.final_plan));

    std::fs::write(&path, &content)?;
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
        Ok(None)
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
            if let Some(stem) = name.strip_suffix(".md") {
                names.push(stem.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub fn build_planner_prompt(task: &str) -> String {
    format!(
        "You are a planning agent. Create a detailed, step-by-step implementation plan for:\n\n\
         {task}\n\n\
         Break it down into:\n\
         1. Requirements analysis\n\
         2. Design decisions\n\
         3. Implementation steps (ordered by dependency)\n\
         4. Testing strategy\n\
         5. Potential risks\n\n\
         Be specific about files, functions, and data structures."
    )
}

pub fn build_critic_prompt(plan: &str) -> String {
    format!(
        "You are a critic agent reviewing a plan. Challenge assumptions, find gaps, and suggest improvements:\n\n\
         {plan}\n\n\
         Focus on:\n\
         - Missing edge cases\n\
         - Unclear dependencies\n\
         - Better alternatives\n\
         - Risk mitigation\n\
         - Testing gaps\n\n\
         Be specific and actionable."
    )
}
