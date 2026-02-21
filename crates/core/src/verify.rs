use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckKind {
    Build,
    Test,
    Lint,
    Custom,
}

impl std::fmt::Display for CheckKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckKind::Build => write!(f, "build"),
            CheckKind::Test => write!(f, "test"),
            CheckKind::Lint => write!(f, "lint"),
            CheckKind::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyCheck {
    pub kind: CheckKind,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: CheckKind,
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timestamp: u64,
    pub elapsed_ms: u64,
}

impl Evidence {
    pub fn passed(&self) -> bool {
        self.exit_code == 0
    }

    pub fn is_fresh(&self, max_age: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.timestamp) < max_age.as_secs()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub checks: Vec<Evidence>,
}

impl VerifyReport {
    pub fn all_passed(&self) -> bool {
        self.checks.iter().all(|e| e.passed())
    }

    pub fn summary(&self) -> String {
        let mut lines = vec![];
        for e in &self.checks {
            let icon = if e.passed() { "PASS" } else { "FAIL" };
            lines.push(format!(
                "[{icon}] {} ({}) - {:.1}s",
                e.kind,
                e.command,
                e.elapsed_ms as f64 / 1000.0,
            ));
            if !e.passed() {
                let output = if e.stderr.len() > 500 {
                    format!("...{}", &e.stderr[e.stderr.len() - 500..])
                } else if !e.stderr.is_empty() {
                    e.stderr.clone()
                } else if e.stdout.len() > 500 {
                    format!("...{}", &e.stdout[e.stdout.len() - 500..])
                } else {
                    e.stdout.clone()
                };
                if !output.is_empty() {
                    lines.push(format!("    {output}"));
                }
            }
        }
        lines.join("\n")
    }
}

pub fn detect_checks(project_root: &Path) -> Vec<VerifyCheck> {
    let mut checks = vec![];

    if project_root.join("Cargo.toml").exists() {
        checks.push(VerifyCheck { kind: CheckKind::Build, command: "cargo check".to_string() });
        checks.push(VerifyCheck { kind: CheckKind::Test, command: "cargo test".to_string() });
        checks.push(VerifyCheck { kind: CheckKind::Lint, command: "cargo clippy -- -D warnings".to_string() });
    } else if project_root.join("package.json").exists() {
        checks.push(VerifyCheck { kind: CheckKind::Build, command: "npm run build".to_string() });
        checks.push(VerifyCheck { kind: CheckKind::Test, command: "npm test".to_string() });
        if project_root.join("node_modules/.bin/eslint").exists() {
            checks.push(VerifyCheck { kind: CheckKind::Lint, command: "npx eslint .".to_string() });
        }
    } else if project_root.join("go.mod").exists() {
        checks.push(VerifyCheck { kind: CheckKind::Build, command: "go build ./...".to_string() });
        checks.push(VerifyCheck { kind: CheckKind::Test, command: "go test ./...".to_string() });
        checks.push(VerifyCheck { kind: CheckKind::Lint, command: "go vet ./...".to_string() });
    } else if project_root.join("pyproject.toml").exists() || project_root.join("setup.py").exists() {
        checks.push(VerifyCheck { kind: CheckKind::Test, command: "python -m pytest".to_string() });
        checks.push(VerifyCheck { kind: CheckKind::Lint, command: "python -m ruff check .".to_string() });
    }

    checks
}

pub async fn run_check(check: &VerifyCheck, cwd: &Path) -> Evidence {
    let start = std::time::Instant::now();
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let result = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&check.command)
        .current_dir(cwd)
        .output()
        .await;

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(output) => Evidence {
            kind: check.kind.clone(),
            command: check.command.clone(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            timestamp: ts,
            elapsed_ms: elapsed,
        },
        Err(e) => Evidence {
            kind: check.kind.clone(),
            command: check.command.clone(),
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Failed to execute: {e}"),
            timestamp: ts,
            elapsed_ms: elapsed,
        },
    }
}

pub async fn run_all_checks(checks: &[VerifyCheck], cwd: &Path) -> VerifyReport {
    let mut evidence = vec![];
    for check in checks {
        evidence.push(run_check(check, cwd).await);
    }
    VerifyReport { checks: evidence }
}
