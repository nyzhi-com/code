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

fn scan_skills_dir(dir: &Path) -> Vec<Skill> {
    if !dir.exists() {
        return vec![];
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };
    let mut skills = vec![];
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            if let Ok(content) = std::fs::read_to_string(&path) {
                skills.push(Skill { name, content, path });
            }
        }
    }
    skills
}

/// Load skills from `.nyzhi/skills/` and `.claude/skills/`.
/// `.nyzhi/skills/` takes priority on name collisions.
pub fn load_skills(project_root: &Path) -> Result<Vec<Skill>> {
    let mut skills = scan_skills_dir(&skills_dir(project_root));
    let fallback = scan_skills_dir(&project_root.join(".claude").join("skills"));

    for skill in fallback {
        if !skills.iter().any(|s| s.name == skill.name) {
            skills.push(skill);
        }
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

pub fn format_skills_for_prompt(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n\n# Skills\n\nThe following skills are loaded from `.nyzhi/skills/`:\n\n");
    for skill in skills {
        out.push_str(&format!("## {}\n\n{}\n\n", skill.name, skill.content.trim()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_directory_nyzhi_wins() {
        let dir = tempfile::tempdir().unwrap();
        let nyzhi_skills = dir.path().join(".nyzhi").join("skills");
        let claude_skills = dir.path().join(".claude").join("skills");
        std::fs::create_dir_all(&nyzhi_skills).unwrap();
        std::fs::create_dir_all(&claude_skills).unwrap();
        std::fs::write(nyzhi_skills.join("review.md"), "nyzhi review skill").unwrap();
        std::fs::write(claude_skills.join("review.md"), "claude review skill").unwrap();
        std::fs::write(claude_skills.join("deploy.md"), "claude deploy skill").unwrap();

        let skills = load_skills(dir.path()).unwrap();
        let review = skills.iter().find(|s| s.name == "review").unwrap();
        assert!(review.content.contains("nyzhi"));
        assert!(skills.iter().any(|s| s.name == "deploy"));
    }

    #[test]
    fn claude_skills_only() {
        let dir = tempfile::tempdir().unwrap();
        let claude_skills = dir.path().join(".claude").join("skills");
        std::fs::create_dir_all(&claude_skills).unwrap();
        std::fs::write(claude_skills.join("test.md"), "test skill").unwrap();

        let skills = load_skills(dir.path()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "test");
    }

    #[test]
    fn no_dirs_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let skills = load_skills(dir.path()).unwrap();
        assert!(skills.is_empty());
    }
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
