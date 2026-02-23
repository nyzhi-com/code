use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allow_network: Vec<String>,
    #[serde(default)]
    pub allow_read: Vec<String>,
    #[serde(default)]
    pub allow_write: Vec<String>,
    #[serde(default)]
    pub block_dotfiles: bool,
}

pub struct SandboxedCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env_overrides: Vec<(String, String)>,
}

const BLOCKED_DOTFILES: &[&str] = &[
    ".ssh",
    ".aws",
    ".npmrc",
    ".env",
    ".netrc",
    ".docker",
    ".kube",
    ".gnupg",
    ".config/gh",
    ".gitconfig",
];

const DANGEROUS_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "sudo rm",
    "mkfs.",
    "dd if=",
    ":(){:|:&};:",
    "curl | bash",
    "curl | sh",
    "wget | bash",
    "wget | sh",
    "> /dev/sd",
    "chmod 777 /",
    "chown root",
];

pub fn is_dangerous_command(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    DANGEROUS_PATTERNS
        .iter()
        .any(|pat| lower.contains(&pat.to_lowercase()))
}

pub fn check_dotfile_access(path: &str) -> bool {
    let home = dirs::home_dir().unwrap_or_default();
    for dotfile in BLOCKED_DOTFILES {
        let blocked = home.join(dotfile);
        if path.starts_with(&blocked.to_string_lossy().to_string()) {
            return true;
        }
    }
    false
}

pub fn scan_for_secrets(text: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let patterns = [
        ("AWS Access Key", r"AKIA[0-9A-Z]{16}"),
        ("GitHub Token", r"gh[pousr]_[A-Za-z0-9_]{36,}"),
        (
            "Generic API Key",
            r#"(?i)(api[_-]?key|apikey|secret[_-]?key)\s*[:=]\s*["'][A-Za-z0-9+/=]{20,}["']"#,
        ),
        ("Bearer Token", r"Bearer\s+[A-Za-z0-9\-._~+/]+=*"),
        (
            "Private Key Header",
            r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----",
        ),
    ];

    for (name, pattern) in &patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(text) {
                findings.push(format!("Potential {name} detected in output"));
            }
        }
    }
    findings
}

#[cfg(target_os = "macos")]
pub fn wrap_command_sandboxed(
    cmd: &str,
    project_root: &Path,
    config: &SandboxConfig,
) -> Result<SandboxedCommand> {
    let profile = generate_seatbelt_profile(project_root, config);
    let profile_path = project_root.join(".nyzhi").join("sandbox.sb");
    std::fs::create_dir_all(profile_path.parent().unwrap())?;
    std::fs::write(&profile_path, &profile)?;

    Ok(SandboxedCommand {
        program: "sandbox-exec".to_string(),
        args: vec![
            "-f".to_string(),
            profile_path.to_string_lossy().to_string(),
            "/bin/sh".to_string(),
            "-c".to_string(),
            cmd.to_string(),
        ],
        env_overrides: vec![],
    })
}

#[cfg(not(target_os = "macos"))]
pub fn wrap_command_sandboxed(
    cmd: &str,
    project_root: &Path,
    config: &SandboxConfig,
) -> Result<SandboxedCommand> {
    let has_bwrap = std::process::Command::new("which")
        .arg("bwrap")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if has_bwrap {
        let mut args = vec![
            "--ro-bind".to_string(),
            "/usr".to_string(),
            "/usr".to_string(),
            "--ro-bind".to_string(),
            "/lib".to_string(),
            "/lib".to_string(),
            "--ro-bind".to_string(),
            "/lib64".to_string(),
            "/lib64".to_string(),
            "--ro-bind".to_string(),
            "/bin".to_string(),
            "/bin".to_string(),
            "--ro-bind".to_string(),
            "/sbin".to_string(),
            "/sbin".to_string(),
            "--proc".to_string(),
            "/proc".to_string(),
            "--dev".to_string(),
            "/dev".to_string(),
            "--tmpfs".to_string(),
            "/tmp".to_string(),
            "--bind".to_string(),
            project_root.to_string_lossy().to_string(),
            project_root.to_string_lossy().to_string(),
            "--chdir".to_string(),
            project_root.to_string_lossy().to_string(),
        ];

        for path in &config.allow_read {
            args.extend_from_slice(&["--ro-bind".to_string(), path.clone(), path.clone()]);
        }
        for path in &config.allow_write {
            args.extend_from_slice(&["--bind".to_string(), path.clone(), path.clone()]);
        }

        if config.allow_network.is_empty() {
            args.push("--unshare-net".to_string());
        }

        args.extend_from_slice(&["/bin/sh".to_string(), "-c".to_string(), cmd.to_string()]);

        Ok(SandboxedCommand {
            program: "bwrap".to_string(),
            args,
            env_overrides: vec![],
        })
    } else {
        Ok(SandboxedCommand {
            program: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), cmd.to_string()],
            env_overrides: vec![],
        })
    }
}

#[cfg(target_os = "macos")]
fn generate_seatbelt_profile(project_root: &Path, config: &SandboxConfig) -> String {
    let project_dir = project_root.to_string_lossy();
    let tmp_dir = std::env::temp_dir().to_string_lossy().to_string();
    let home = dirs::home_dir().unwrap_or_default();

    let mut profile = String::from("(version 1)\n(deny default)\n");
    profile.push_str("(allow process-exec)\n");
    profile.push_str("(allow process-fork)\n");
    profile.push_str("(allow sysctl-read)\n");
    profile.push_str("(allow mach-lookup)\n");

    profile.push_str(&format!("(allow file-read* (subpath \"{project_dir}\"))\n"));
    profile.push_str(&format!(
        "(allow file-write* (subpath \"{project_dir}\"))\n"
    ));
    profile.push_str(&format!("(allow file-read* (subpath \"{tmp_dir}\"))\n"));
    profile.push_str(&format!("(allow file-write* (subpath \"{tmp_dir}\"))\n"));

    profile.push_str("(allow file-read* (subpath \"/usr\"))\n");
    profile.push_str("(allow file-read* (subpath \"/bin\"))\n");
    profile.push_str("(allow file-read* (subpath \"/sbin\"))\n");
    profile.push_str("(allow file-read* (subpath \"/Library\"))\n");
    profile.push_str("(allow file-read* (subpath \"/System\"))\n");
    profile.push_str("(allow file-read* (subpath \"/dev\"))\n");
    profile.push_str("(allow file-read* (subpath \"/private/tmp\"))\n");
    profile.push_str("(allow file-read* (subpath \"/private/var\"))\n");

    for dotfile in BLOCKED_DOTFILES {
        let blocked_path = home.join(dotfile);
        profile.push_str(&format!(
            "(deny file-read* (subpath \"{}\"))\n",
            blocked_path.display()
        ));
    }

    for path in &config.allow_read {
        profile.push_str(&format!("(allow file-read* (subpath \"{path}\"))\n"));
    }
    for path in &config.allow_write {
        profile.push_str(&format!("(allow file-write* (subpath \"{path}\"))\n"));
    }

    if config.allow_network.is_empty() {
        profile.push_str("(deny network*)\n");
    } else {
        profile.push_str("(allow network*)\n");
    }

    profile
}
