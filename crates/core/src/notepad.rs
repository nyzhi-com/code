use anyhow::Result;
use std::path::{Path, PathBuf};

fn notepad_dir(project_root: &Path, plan_name: &str) -> PathBuf {
    project_root.join(".nyzhi").join("notepads").join(plan_name)
}

pub fn ensure_notepad(project_root: &Path, plan_name: &str) -> Result<PathBuf> {
    let dir = notepad_dir(project_root, plan_name);
    std::fs::create_dir_all(&dir)?;
    for file in &["learnings.md", "decisions.md", "issues.md"] {
        let path = dir.join(file);
        if !path.exists() {
            std::fs::write(
                &path,
                format!("# {}\n\n", file.replace(".md", "").to_uppercase()),
            )?;
        }
    }
    Ok(dir)
}

pub fn append_entry(
    project_root: &Path,
    plan_name: &str,
    category: &str,
    entry: &str,
) -> Result<String> {
    let dir = ensure_notepad(project_root, plan_name)?;
    let file = match category {
        "learning" | "learnings" => "learnings.md",
        "decision" | "decisions" => "decisions.md",
        "issue" | "issues" => "issues.md",
        _ => {
            return Err(anyhow::anyhow!(
                "Unknown category: {category} (use learnings, decisions, or issues)"
            ))
        }
    };
    let path = dir.join(file);

    let ts = chrono_timestamp();
    let formatted = format!("\n## [{ts}]\n{entry}\n");

    let mut content = std::fs::read_to_string(&path).unwrap_or_default();
    content.push_str(&formatted);
    std::fs::write(&path, &content)?;

    Ok(format!("Added to {file} in plan '{plan_name}'"))
}

pub fn read_notepad(project_root: &Path, plan_name: &str) -> Result<String> {
    let dir = notepad_dir(project_root, plan_name);
    if !dir.exists() {
        return Ok(format!("No notepad found for plan '{plan_name}'"));
    }

    let mut output = format!("=== Notepad: {plan_name} ===\n\n");
    for file in &["learnings.md", "decisions.md", "issues.md"] {
        let path = dir.join(file);
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            if content.lines().count() > 2 {
                output.push_str(&content);
                output.push_str("\n---\n\n");
            }
        }
    }
    Ok(output)
}

pub fn list_notepads(project_root: &Path) -> Result<Vec<String>> {
    let dir = project_root.join(".nyzhi").join("notepads");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names = vec![];
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

fn chrono_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs = now % 60;
    let mins = (now / 60) % 60;
    let hours = (now / 3600) % 24;
    let days = now / 86400;
    format!("{days}d {hours:02}:{mins:02}:{secs:02}")
}
