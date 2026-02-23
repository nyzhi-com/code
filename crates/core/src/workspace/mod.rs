use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource {
    Nyzhi,
    ClaudeCode,
    Cursor,
    GitOnly,
    None,
}

impl ConfigSource {
    pub fn label(&self) -> &'static str {
        match self {
            ConfigSource::Nyzhi => "nyzhi (.nyzhi/)",
            ConfigSource::ClaudeCode => "Claude Code (.claude/)",
            ConfigSource::Cursor => "Cursor (.cursorrules)",
            ConfigSource::GitOnly => "git only",
            ConfigSource::None => "none",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub project_root: PathBuf,
    pub project_type: Option<ProjectType>,
    pub git_branch: Option<String>,
    pub has_nyzhi_config: bool,
    pub config_source: ConfigSource,
    pub rules: Option<String>,
    pub rules_file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Unknown,
}

impl ProjectType {
    pub fn name(&self) -> &'static str {
        match self {
            ProjectType::Rust => "rust",
            ProjectType::Node => "node",
            ProjectType::Python => "python",
            ProjectType::Go => "go",
            ProjectType::Unknown => "unknown",
        }
    }
}

pub fn detect_workspace(cwd: &Path) -> WorkspaceContext {
    let project_root = find_project_root(cwd);
    let project_type = detect_project_type(&project_root);
    let git_branch = detect_git_branch(&project_root);
    let has_nyzhi_config = project_root.join(".nyzhi").join("config.toml").exists();
    let rules = load_rules(&project_root);
    let rules_file = rules_source(&project_root);

    let config_source = if project_root.join(".nyzhi").is_dir() {
        ConfigSource::Nyzhi
    } else if project_root.join(".claude").is_dir() {
        ConfigSource::ClaudeCode
    } else if project_root.join(".cursorrules").exists() {
        ConfigSource::Cursor
    } else if project_root.join(".git").exists() {
        ConfigSource::GitOnly
    } else {
        ConfigSource::None
    };

    WorkspaceContext {
        project_root,
        project_type,
        git_branch,
        has_nyzhi_config,
        config_source,
        rules,
        rules_file,
    }
}

fn find_project_root(start: &Path) -> PathBuf {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".nyzhi").is_dir() {
            return current;
        }
        if current.join(".claude").is_dir() {
            return current;
        }
        if current.join(".git").exists() {
            return current;
        }
        if !current.pop() {
            return start.to_path_buf();
        }
    }
}

fn detect_project_type(root: &Path) -> Option<ProjectType> {
    if root.join("Cargo.toml").exists() {
        Some(ProjectType::Rust)
    } else if root.join("package.json").exists() {
        Some(ProjectType::Node)
    } else if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("requirements.txt").exists()
    {
        Some(ProjectType::Python)
    } else if root.join("go.mod").exists() {
        Some(ProjectType::Go)
    } else {
        None
    }
}

fn detect_git_branch(root: &Path) -> Option<String> {
    let head_path = root.join(".git").join("HEAD");
    let content = std::fs::read_to_string(head_path).ok()?;
    let content = content.trim();
    if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
        Some(branch.to_string())
    } else if content.len() >= 8 {
        Some(content[..8].to_string())
    } else {
        None
    }
}

pub fn load_rules(root: &Path) -> Option<String> {
    let candidates = [
        root.join("AGENTS.md"),
        root.join(".nyzhi").join("rules.md"),
        root.join(".nyzhi").join("instructions.md"),
        root.join("CLAUDE.md"),
        root.join(".cursorrules"),
    ];

    for path in &candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if !content.trim().is_empty() {
                return Some(content);
            }
        }
    }
    None
}

pub fn rules_source(root: &Path) -> Option<String> {
    let candidates: &[(&str, PathBuf)] = &[
        ("AGENTS.md", root.join("AGENTS.md")),
        (".nyzhi/rules.md", root.join(".nyzhi").join("rules.md")),
        (
            ".nyzhi/instructions.md",
            root.join(".nyzhi").join("instructions.md"),
        ),
        ("CLAUDE.md", root.join("CLAUDE.md")),
        (".cursorrules", root.join(".cursorrules")),
    ];
    for (label, path) in candidates {
        if let Ok(content) = std::fs::read_to_string(path) {
            if !content.trim().is_empty() {
                return Some(label.to_string());
            }
        }
    }
    None
}

pub fn scaffold_nyzhi_dir(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let nyzhi_dir = root.join(".nyzhi");
    std::fs::create_dir_all(&nyzhi_dir)?;

    let mut created = Vec::new();

    let config_path = nyzhi_dir.join("config.toml");
    if !config_path.exists() {
        std::fs::write(
            &config_path,
            r#"# Project-level nyzhi configuration
# These settings override your global ~/.config/nyzhi/config.toml

# [provider]
# default = "anthropic"
#
# [provider.anthropic]
# model = "claude-sonnet-4-20250514"

# [agent]
# max_steps = 50
# custom_instructions = "Always write tests for new functions."
"#,
        )?;
        created.push(config_path);
    }

    let rules_path = nyzhi_dir.join("rules.md");
    if !rules_path.exists() {
        std::fs::write(
            &rules_path,
            r#"# Project Rules

These instructions are injected into every nyzhi conversation in this project.

## Guidelines

- Describe your project's coding conventions here.
- Specify preferred patterns, testing requirements, or constraints.
- Example: "Use `anyhow::Result` for all error handling."
- Example: "Run `cargo test` before considering a task complete."

## Compatibility

nyzhi also recognizes CLAUDE.md and .cursorrules as project rules.
If you already use Claude Code or Cursor, those files work automatically.
Priority: AGENTS.md > .nyzhi/rules.md > .nyzhi/instructions.md > CLAUDE.md > .cursorrules
"#,
        )?;
        created.push(rules_path);
    }

    let commands_dir = nyzhi_dir.join("commands");
    std::fs::create_dir_all(&commands_dir)?;

    let review_path = commands_dir.join("review.md");
    if !review_path.exists() {
        std::fs::write(
            &review_path,
            r#"# Review code for issues
Review $ARGUMENTS for bugs, security issues, and improvements. Be thorough and specific.
"#,
        )?;
        created.push(review_path);
    }

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_rules_agents_md_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "agents rules").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "claude rules").unwrap();
        assert_eq!(load_rules(dir.path()).unwrap(), "agents rules");
    }

    #[test]
    fn load_rules_claude_md_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "claude rules").unwrap();
        assert_eq!(load_rules(dir.path()).unwrap(), "claude rules");
        assert_eq!(rules_source(dir.path()).unwrap(), "CLAUDE.md");
    }

    #[test]
    fn load_rules_cursorrules_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".cursorrules"), "cursor rules").unwrap();
        assert_eq!(load_rules(dir.path()).unwrap(), "cursor rules");
        assert_eq!(rules_source(dir.path()).unwrap(), ".cursorrules");
    }

    #[test]
    fn load_rules_nyzhi_over_claude() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".nyzhi")).unwrap();
        std::fs::write(dir.path().join(".nyzhi").join("rules.md"), "nyzhi rules").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "claude rules").unwrap();
        assert_eq!(load_rules(dir.path()).unwrap(), "nyzhi rules");
    }

    #[test]
    fn load_rules_empty_file_skipped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "   ").unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "real rules").unwrap();
        assert_eq!(load_rules(dir.path()).unwrap(), "real rules");
    }

    #[test]
    fn load_rules_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_rules(dir.path()).is_none());
    }

    #[test]
    fn config_source_nyzhi_priority() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".nyzhi")).unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let ws = detect_workspace(dir.path());
        assert_eq!(ws.config_source, ConfigSource::Nyzhi);
    }

    #[test]
    fn config_source_claude_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let ws = detect_workspace(dir.path());
        assert_eq!(ws.config_source, ConfigSource::ClaudeCode);
    }

    #[test]
    fn find_root_claude_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("deep").join("nested");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let root = find_project_root(&sub);
        assert_eq!(root, dir.path());
    }
}
