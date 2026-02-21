use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub content: String,
    pub path: PathBuf,
}

fn skills_dir(project_root: &Path) -> PathBuf {
    project_root.join(".nyzhi").join("skills")
}

pub fn save_skill(project_root: &Path, name: &str, content: &str) -> Result<PathBuf> {
    let dir = skills_dir(project_root);
    std::fs::create_dir_all(&dir)?;

    let safe_name: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let path = dir.join(format!("{safe_name}.md"));
    std::fs::write(&path, content)?;
    Ok(path)
}

pub fn load_skills(project_root: &Path) -> Result<Vec<Skill>> {
    let dir = skills_dir(project_root);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut skills = vec![];
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let content = std::fs::read_to_string(&path)?;
            skills.push(Skill { name, content, path });
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

pub fn build_skill_template(name: &str, description: &str, patterns: &[String]) -> String {
    let mut content = format!("# Skill: {name}\n\n");
    content.push_str(&format!("## Description\n{description}\n\n"));
    content.push_str("## Patterns\n\n");
    for pattern in patterns {
        content.push_str(&format!("- {pattern}\n"));
    }
    content.push_str("\n## When to Apply\n\n");
    content.push_str("- [Describe conditions]\n\n");
    content.push_str("## Examples\n\n");
    content.push_str("```\n[Add examples]\n```\n");
    content
}
