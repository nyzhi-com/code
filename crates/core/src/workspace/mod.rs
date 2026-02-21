use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub project_root: PathBuf,
    pub project_type: Option<ProjectType>,
    pub git_branch: Option<String>,
    pub has_nyzhi_config: bool,
    pub rules: Option<String>,
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

    WorkspaceContext {
        project_root,
        project_type,
        git_branch,
        has_nyzhi_config,
        rules,
    }
}

fn find_project_root(start: &Path) -> PathBuf {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".nyzhi").is_dir() {
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
