#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolPermission {
    ReadOnly,
    NeedsApproval,
}

/// Returns true if the tool or path is explicitly denied by the trust config.
pub fn check_deny(
    tool_name: &str,
    target_path: Option<&str>,
    trust: &nyzhi_config::TrustConfig,
) -> bool {
    if trust
        .deny_tools
        .iter()
        .any(|d| d.eq_ignore_ascii_case(tool_name) || d == "*")
    {
        return true;
    }

    if let Some(path) = target_path {
        if trust.deny_paths.iter().any(|d| {
            path.starts_with(d.as_str())
                || path.contains(d.as_str())
                || (d.starts_with("*.") && path.ends_with(&d[1..]))
        }) {
            return true;
        }
    }

    false
}

/// Check whether a tool should be auto-approved under the current trust config.
/// Returns Some(true) for auto-approve, Some(false) for always-ask, None for default behavior.
pub fn check_auto_approve(
    tool_name: &str,
    permission: ToolPermission,
    trust: &nyzhi_config::TrustConfig,
) -> Option<bool> {
    if trust
        .always_ask
        .iter()
        .any(|t| t.eq_ignore_ascii_case(tool_name))
    {
        return Some(false);
    }
    if trust
        .auto_approve
        .iter()
        .any(|t| t.eq_ignore_ascii_case(tool_name))
    {
        return Some(true);
    }

    match trust.mode {
        nyzhi_config::TrustMode::Full => Some(true),
        nyzhi_config::TrustMode::AutoEdit => match permission {
            ToolPermission::ReadOnly => Some(true),
            ToolPermission::NeedsApproval => {
                let write_tools = [
                    "write",
                    "edit",
                    "multi_edit",
                    "apply_patch",
                    "delete_file",
                    "move_file",
                    "copy_file",
                    "create_dir",
                ];
                if write_tools.contains(&tool_name) {
                    Some(true)
                } else {
                    None
                }
            }
        },
        nyzhi_config::TrustMode::Limited => match permission {
            ToolPermission::ReadOnly => Some(true),
            ToolPermission::NeedsApproval => None,
        },
        nyzhi_config::TrustMode::Off => None,
    }
}

const DANGEROUS_COMMANDS: &[&str] = &[
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
];

/// Returns true if a bash command contains dangerous patterns
/// that should always require explicit approval.
pub fn is_dangerous_bash(command: &str) -> bool {
    let lower = command.to_lowercase();
    DANGEROUS_COMMANDS
        .iter()
        .any(|pat| lower.contains(&pat.to_lowercase()))
}

/// Session-level approval memory.
#[derive(Default)]
pub struct ApprovalMemory {
    approved: std::collections::HashSet<String>,
}

impl ApprovalMemory {
    pub fn remember(&mut self, tool_name: &str, pattern: &str) {
        self.approved.insert(format!("{tool_name}:{pattern}"));
    }

    pub fn was_approved(&self, tool_name: &str, pattern: &str) -> bool {
        self.approved.contains(&format!("{tool_name}:{pattern}"))
    }
}
