use std::path::Path;

#[derive(Debug, Clone)]
pub enum DiagStatus {
    Pass,
    Warn,
    Fail,
}

impl std::fmt::Display for DiagStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagStatus::Pass => write!(f, "PASS"),
            DiagStatus::Warn => write!(f, "WARN"),
            DiagStatus::Fail => write!(f, "FAIL"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub name: String,
    pub status: DiagStatus,
    pub message: String,
}

pub fn run_diagnostics(
    provider_name: &str,
    project_root: &Path,
    mcp_count: usize,
    tool_count: usize,
) -> Vec<DiagnosticResult> {
    let mut results = Vec::new();

    // Config check
    match nyzhi_config::Config::load() {
        Ok(_) => results.push(DiagnosticResult {
            name: "Global config".into(),
            status: DiagStatus::Pass,
            message: format!("{}", nyzhi_config::Config::config_path().display()),
        }),
        Err(e) => results.push(DiagnosticResult {
            name: "Global config".into(),
            status: DiagStatus::Warn,
            message: format!("Could not load: {e}"),
        }),
    }

    // Project config
    let project_config = project_root.join(".nyzhi").join("config.toml");
    if project_config.exists() {
        results.push(DiagnosticResult {
            name: "Project config".into(),
            status: DiagStatus::Pass,
            message: project_config.display().to_string(),
        });
    } else {
        results.push(DiagnosticResult {
            name: "Project config".into(),
            status: DiagStatus::Warn,
            message: "Not found (run /init to create)".into(),
        });
    }

    // Provider
    results.push(DiagnosticResult {
        name: "Provider".into(),
        status: DiagStatus::Pass,
        message: provider_name.to_string(),
    });

    // Git
    let git_dir = project_root.join(".git");
    if git_dir.exists() {
        results.push(DiagnosticResult {
            name: "Git".into(),
            status: DiagStatus::Pass,
            message: "Repository detected".into(),
        });
    } else {
        results.push(DiagnosticResult {
            name: "Git".into(),
            status: DiagStatus::Warn,
            message: "No git repository in project root".into(),
        });
    }

    // MCP servers
    if mcp_count > 0 {
        results.push(DiagnosticResult {
            name: "MCP servers".into(),
            status: DiagStatus::Pass,
            message: format!("{mcp_count} server(s) connected"),
        });
    } else {
        results.push(DiagnosticResult {
            name: "MCP servers".into(),
            status: DiagStatus::Warn,
            message: "No MCP servers configured".into(),
        });
    }

    // Tools
    results.push(DiagnosticResult {
        name: "Tools".into(),
        status: DiagStatus::Pass,
        message: format!("{tool_count} tool(s) registered"),
    });

    // Skills
    let skills_dir = project_root.join(".nyzhi").join("skills");
    let skill_count = if skills_dir.exists() {
        std::fs::read_dir(&skills_dir)
            .map(|d| d.count())
            .unwrap_or(0)
    } else {
        0
    };
    results.push(DiagnosticResult {
        name: "Skills".into(),
        status: if skill_count > 0 { DiagStatus::Pass } else { DiagStatus::Warn },
        message: format!("{skill_count} skill(s) in .nyzhi/skills/"),
    });

    // OS info
    results.push(DiagnosticResult {
        name: "Platform".into(),
        status: DiagStatus::Pass,
        message: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
    });

    results
}

pub fn format_diagnostics(results: &[DiagnosticResult]) -> String {
    let mut out = String::from("System Diagnostics\n\n");
    for r in results {
        let icon = match r.status {
            DiagStatus::Pass => "+",
            DiagStatus::Warn => "~",
            DiagStatus::Fail => "x",
        };
        out.push_str(&format!("  {icon} {:<18} {}\n", r.name, r.message));
    }
    out
}

pub fn generate_bug_report(
    provider_name: &str,
    model_name: &str,
    trust_mode: &str,
    session_id: &str,
) -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        r#"## Bug Report

### Environment
- nyzhi version: {version}
- OS: {} {}
- Shell: {}
- Provider: {provider_name}
- Model: {model_name}
- Trust mode: {trust_mode}
- Session: {session_id}

### Description
[Describe the issue]

### Steps to Reproduce
1. [Step 1]
2. [Step 2]

### Expected Behavior
[What you expected]

### Actual Behavior
[What actually happened]

### Additional Context
[Logs, screenshots, etc.]
"#,
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
    )
}
