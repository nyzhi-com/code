use std::collections::HashMap;
use std::path::Path;

use crate::agent_roles::AgentRoleConfig;

fn scan_agents_dir(dir: &Path) -> HashMap<String, AgentRoleConfig> {
    let mut roles = HashMap::new();

    if !dir.exists() {
        return roles;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return roles,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let (fm, body) = parse_frontmatter(&content);

        let name = fm.get("name").cloned().unwrap_or_else(|| file_name.clone());
        let description = fm.get("description").cloned();
        let model_override = fm.get("model").cloned();
        let max_steps_override = fm.get("max_steps").and_then(|v| v.parse::<u32>().ok());
        let read_only = fm.get("read_only").map(|v| v == "true").unwrap_or(false);
        let allowed_tools = fm.get("allowed_tools").map(|v| parse_yaml_list(v));
        let disallowed_tools = fm.get("disallowed_tools").map(|v| parse_yaml_list(v));

        let system_prompt = if body.trim().is_empty() {
            None
        } else {
            Some(body.trim().to_string())
        };

        roles.insert(
            file_name,
            AgentRoleConfig {
                name,
                description,
                system_prompt_override: system_prompt,
                model_override,
                max_steps_override,
                read_only,
                allowed_tools,
                disallowed_tools,
                config_file: Some(path.display().to_string()),
            },
        );
    }

    roles
}

/// Parse agent role files from `.nyzhi/agents/` and `.claude/agents/`.
/// `.nyzhi/agents/` takes priority on name collisions.
pub fn load_file_based_roles(project_root: &Path) -> HashMap<String, AgentRoleConfig> {
    let mut roles = scan_agents_dir(&project_root.join(".claude").join("agents"));
    let primary = scan_agents_dir(&project_root.join(".nyzhi").join("agents"));
    roles.extend(primary);
    roles
}

/// Simple YAML-like frontmatter parser. Returns key-value pairs from `---` delimited block.
fn parse_frontmatter(content: &str) -> (HashMap<String, String>, String) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (HashMap::new(), content.to_string());
    }

    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let yaml_str = &after_first[..end];
        let body = &after_first[end + 4..];

        let mut map = HashMap::new();
        for line in yaml_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, val)) = line.split_once(':') {
                let key = key.trim().to_string();
                let val = val.trim().trim_matches('"').trim_matches('\'').to_string();
                if !key.is_empty() {
                    map.insert(key, val);
                }
            }
        }
        (map, body.to_string())
    } else {
        (HashMap::new(), content.to_string())
    }
}

fn parse_yaml_list(value: &str) -> Vec<String> {
    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

pub fn format_role_list(
    built_in: &HashMap<String, AgentRoleConfig>,
    config_roles: &HashMap<String, AgentRoleConfig>,
    file_roles: &HashMap<String, AgentRoleConfig>,
) -> String {
    let mut out = String::new();

    out.push_str("Built-in roles:\n");
    let mut names: Vec<_> = built_in.keys().collect();
    names.sort();
    for name in names {
        let r = &built_in[name];
        let desc = r.description.as_deref().unwrap_or("");
        out.push_str(&format!("  {name:<20} {desc}\n"));
    }

    if !config_roles.is_empty() {
        out.push_str("\nConfig roles (config.toml):\n");
        let mut names: Vec<_> = config_roles.keys().collect();
        names.sort();
        for name in names {
            let r = &config_roles[name];
            let desc = r.description.as_deref().unwrap_or("");
            out.push_str(&format!("  {name:<20} {desc}\n"));
        }
    }

    if !file_roles.is_empty() {
        out.push_str("\nFile roles (.nyzhi/agents/):\n");
        let mut names: Vec<_> = file_roles.keys().collect();
        names.sort();
        for name in names {
            let r = &file_roles[name];
            let desc = r.description.as_deref().unwrap_or("");
            let src = r.config_file.as_deref().unwrap_or("");
            out.push_str(&format!("  {name:<20} {desc} [{src}]\n"));
        }
    }

    out
}
