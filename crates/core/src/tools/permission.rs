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
    if trust.deny_tools.iter().any(|d| {
        d.eq_ignore_ascii_case(tool_name) || d == "*"
    }) {
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
