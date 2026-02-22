use std::collections::HashMap;
use std::path::Path;

use crate::agent_roles::AgentRoleConfig;

/// Parse `.nyzhi/agents/*.md` files into AgentRoleConfig entries.
/// Each file has optional YAML frontmatter (delimited by `---`) with fields:
///   name, description, model, allowed_tools, disallowed_tools, max_steps, read_only
/// The markdown body becomes the system prompt.
pub fn load_file_based_roles(project_root: &Path) -> HashMap<String, AgentRoleConfig> {
    let agents_dir = project_root.join(".nyzhi").join("agents");
    let mut roles = HashMap::new();

    if !agents_dir.exists() {
        return roles;
    }

    let entries = match std::fs::read_dir(&agents_dir) {
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

        let (frontmatter, body) = parse_frontmatter(&content);

        let name = frontmatter
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| file_name.clone());

        let description = frontmatter
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let model_override = frontmatter
            .get("model")
            .and_then(|v| v.as_str())
            .map(String::from);

        let max_steps_override = frontmatter
            .get("max_steps")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let read_only = frontmatter
            .get("read_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let allowed_tools = frontmatter
            .get("allowed_tools")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        let disallowed_tools = frontmatter
            .get("disallowed_tools")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

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

fn parse_frontmatter(content: &str) -> (serde_yaml::Value, String) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (serde_yaml::Value::Mapping(Default::default()), content.to_string());
    }

    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let yaml_str = &after_first[..end];
        let body = &after_first[end + 4..];
        let fm: serde_yaml::Value = serde_yaml::from_str(yaml_str)
            .unwrap_or(serde_yaml::Value::Mapping(Default::default()));
        (fm, body.to_string())
    } else {
        (serde_yaml::Value::Mapping(Default::default()), content.to_string())
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
