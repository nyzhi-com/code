use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

const MAX_INJECTION_LINES: usize = 200;

/// Compute a stable hash for a project root path.
pub fn project_hash(root: &Path) -> String {
    let canonical = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

/// Base directory for a project's auto-memory.
pub fn memory_dir(root: &Path) -> PathBuf {
    let hash = project_hash(root);
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nyzhi")
        .join("projects")
        .join(hash)
        .join("memory")
}

/// User-level memory file.
pub fn user_memory_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nyzhi")
        .join("MEMORY.md")
}

/// Read the MEMORY.md index for a project. Returns the first MAX_INJECTION_LINES lines.
pub fn load_memory_for_prompt(root: &Path) -> String {
    let mut sections = Vec::new();

    let user_mem = user_memory_path();
    if user_mem.exists() {
        if let Ok(content) = std::fs::read_to_string(&user_mem) {
            let lines: Vec<&str> = content.lines().take(MAX_INJECTION_LINES / 2).collect();
            if !lines.is_empty() {
                sections.push(format!("## User Memory\n{}", lines.join("\n")));
            }
        }
    }

    let project_mem = memory_dir(root).join("MEMORY.md");
    if project_mem.exists() {
        if let Ok(content) = std::fs::read_to_string(&project_mem) {
            let remaining = MAX_INJECTION_LINES.saturating_sub(
                sections.first().map(|s| s.lines().count()).unwrap_or(0),
            );
            let lines: Vec<&str> = content.lines().take(remaining).collect();
            if !lines.is_empty() {
                sections.push(format!("## Project Memory\n{}", lines.join("\n")));
            }
        }
    }

    if sections.is_empty() {
        return String::new();
    }

    format!("\n\n# Recalled Memories\n\n{}\n", sections.join("\n\n"))
}

/// Count total memory entries across user and project memory.
pub fn memory_count(root: &Path) -> usize {
    let mut count = 0;

    let user_mem = user_memory_path();
    if user_mem.exists() {
        if let Ok(content) = std::fs::read_to_string(&user_mem) {
            count += content.lines().filter(|l| l.starts_with("- ")).count();
        }
    }

    let project_mem = memory_dir(root).join("MEMORY.md");
    if project_mem.exists() {
        if let Ok(content) = std::fs::read_to_string(&project_mem) {
            count += content.lines().filter(|l| l.starts_with("- ")).count();
        }
    }

    let topic_dir = memory_dir(root);
    if topic_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&topic_dir) {
            count += entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name();
                    let s = name.to_string_lossy();
                    s.ends_with(".md") && s != "MEMORY.md"
                })
                .count();
        }
    }

    count
}

/// Read a topic file from project memory.
pub fn read_topic(root: &Path, topic: &str) -> Result<String> {
    let safe_name = sanitize_topic_name(topic);
    let path = memory_dir(root).join(format!("{safe_name}.md"));
    std::fs::read_to_string(&path)
        .with_context(|| format!("Topic '{}' not found at {}", topic, path.display()))
}

/// Write or append to a topic file. Updates the MEMORY.md index.
pub fn write_topic(root: &Path, topic: &str, content: &str, replace: bool) -> Result<PathBuf> {
    let dir = memory_dir(root);
    std::fs::create_dir_all(&dir)?;

    let safe_name = sanitize_topic_name(topic);
    let path = dir.join(format!("{safe_name}.md"));

    if replace || !path.exists() {
        std::fs::write(&path, content)?;
    } else {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&path)?;
        writeln!(file)?;
        write!(file, "{content}")?;
    }

    update_index(&dir, topic, &safe_name)?;
    Ok(path)
}

/// Read the MEMORY.md index file content.
pub fn read_index(root: &Path) -> Result<String> {
    let path = memory_dir(root).join("MEMORY.md");
    if path.exists() {
        Ok(std::fs::read_to_string(&path)?)
    } else {
        Ok("No project memories yet.".to_string())
    }
}

/// List all topic files.
pub fn list_topics(root: &Path) -> Vec<String> {
    let dir = memory_dir(root);
    if !dir.exists() {
        return vec![];
    }
    let mut topics = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let s = name.to_string_lossy().to_string();
            if s.ends_with(".md") && s != "MEMORY.md" {
                topics.push(s.trim_end_matches(".md").to_string());
            }
        }
    }
    topics.sort();
    topics
}

/// Clear all auto-memory for a project.
pub fn clear_memory(root: &Path) -> Result<()> {
    let dir = memory_dir(root);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

fn sanitize_topic_name(topic: &str) -> String {
    topic
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn update_index(dir: &Path, topic: &str, safe_name: &str) -> Result<()> {
    let index_path = dir.join("MEMORY.md");
    let mut content = if index_path.exists() {
        std::fs::read_to_string(&index_path)?
    } else {
        "# Project Memory\n\nAuto-managed memory index.\n\n## Topics\n".to_string()
    };

    let entry = format!("- [{}]({}.md)", topic, safe_name);
    if !content.contains(&entry) {
        content.push('\n');
        content.push_str(&entry);
        std::fs::write(&index_path, content)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_hash_is_stable() {
        let dir = tempfile::tempdir().unwrap();
        let h1 = project_hash(dir.path());
        let h2 = project_hash(dir.path());
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn write_and_read_topic() {
        let dir = tempfile::tempdir().unwrap();
        write_topic(dir.path(), "api-conventions", "Use REST style", false).unwrap();
        let content = read_topic(dir.path(), "api-conventions").unwrap();
        assert!(content.contains("Use REST style"));
    }

    #[test]
    fn write_append_mode() {
        let dir = tempfile::tempdir().unwrap();
        write_topic(dir.path(), "notes", "First note", false).unwrap();
        write_topic(dir.path(), "notes", "Second note", false).unwrap();
        let content = read_topic(dir.path(), "notes").unwrap();
        assert!(content.contains("First note"));
        assert!(content.contains("Second note"));
    }

    #[test]
    fn write_replace_mode() {
        let dir = tempfile::tempdir().unwrap();
        write_topic(dir.path(), "notes", "Old content", false).unwrap();
        write_topic(dir.path(), "notes", "New content", true).unwrap();
        let content = read_topic(dir.path(), "notes").unwrap();
        assert!(!content.contains("Old content"));
        assert!(content.contains("New content"));
    }

    #[test]
    fn index_updated_on_write() {
        let dir = tempfile::tempdir().unwrap();
        write_topic(dir.path(), "debug-tips", "Use tracing", false).unwrap();
        let index = read_index(dir.path()).unwrap();
        assert!(index.contains("debug-tips"));
    }

    #[test]
    fn list_topics_works() {
        let dir = tempfile::tempdir().unwrap();
        write_topic(dir.path(), "alpha", "a", false).unwrap();
        write_topic(dir.path(), "beta", "b", false).unwrap();
        let topics = list_topics(dir.path());
        assert_eq!(topics, vec!["alpha", "beta"]);
    }

    #[test]
    fn clear_memory_removes_all() {
        let dir = tempfile::tempdir().unwrap();
        write_topic(dir.path(), "temp", "data", false).unwrap();
        assert!(memory_dir(dir.path()).exists());
        clear_memory(dir.path()).unwrap();
        assert!(!memory_dir(dir.path()).exists());
    }
}
