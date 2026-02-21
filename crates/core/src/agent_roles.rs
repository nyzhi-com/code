use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent::AgentConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoleConfig {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub system_prompt_override: Option<String>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub max_steps_override: Option<u32>,
    #[serde(default)]
    pub read_only: bool,
}

pub fn built_in_roles() -> HashMap<String, AgentRoleConfig> {
    let mut roles = HashMap::new();

    roles.insert(
        "default".to_string(),
        AgentRoleConfig {
            name: "default".to_string(),
            description: Some("Default agent. Inherits parent configuration.".to_string()),
            system_prompt_override: None,
            model_override: None,
            max_steps_override: None,
            read_only: false,
        },
    );

    roles.insert(
        "explorer".to_string(),
        AgentRoleConfig {
            name: "explorer".to_string(),
            description: Some(
                "Fast, read-only agent for codebase exploration. Use for specific, \
                 well-scoped questions about the codebase. Trust explorer results \
                 without re-verifying. Run explorers in parallel when useful."
                    .to_string(),
            ),
            system_prompt_override: Some(
                "You are an explorer sub-agent. Your job is to answer questions about \
                 the codebase quickly and accurately. You have read-only access: use \
                 `read`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, \
                 `git_status`, `git_diff`, `git_log`, `git_show`, and `git_branch`. \
                 Do NOT modify any files. Be concise and authoritative in your answers."
                    .to_string(),
            ),
            model_override: None,
            max_steps_override: Some(30),
            read_only: true,
        },
    );

    roles.insert(
        "worker".to_string(),
        AgentRoleConfig {
            name: "worker".to_string(),
            description: Some(
                "Execution agent for implementation tasks. Use for implementing features, \
                 fixing bugs, writing code, or making changes. Has full tool access. \
                 Tell workers they are not alone in the codebase and should ignore \
                 edits made by other agents."
                    .to_string(),
            ),
            system_prompt_override: Some(
                "You are a worker sub-agent. Implement the assigned task thoroughly. \
                 You have full tool access. Note: other agents may be working on \
                 the same codebase concurrently -- do not touch files outside your \
                 assigned scope."
                    .to_string(),
            ),
            model_override: None,
            max_steps_override: Some(50),
            read_only: false,
        },
    );

    roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            name: "reviewer".to_string(),
            description: Some(
                "Code review agent. Analyzes code for bugs, security issues, and \
                 improvements. Has read-only access. Returns structured findings."
                    .to_string(),
            ),
            system_prompt_override: Some(
                "You are a code reviewer sub-agent. Analyze the given code for bugs, \
                 security issues, performance problems, and possible improvements. \
                 You have read-only access. Structure your findings by severity: \
                 critical, warning, suggestion. Be specific with file and line references."
                    .to_string(),
            ),
            model_override: None,
            max_steps_override: Some(30),
            read_only: true,
        },
    );

    roles
}

pub fn resolve_role(
    name: Option<&str>,
    user_roles: &HashMap<String, AgentRoleConfig>,
) -> AgentRoleConfig {
    let name = name.unwrap_or("default");
    if let Some(role) = user_roles.get(name) {
        return role.clone();
    }
    let builtins = built_in_roles();
    if let Some(role) = builtins.get(name) {
        return role.clone();
    }
    AgentRoleConfig {
        name: name.to_string(),
        description: None,
        system_prompt_override: None,
        model_override: None,
        max_steps_override: None,
        read_only: false,
    }
}

pub fn apply_role(config: &mut AgentConfig, role: &AgentRoleConfig) {
    if let Some(prompt) = &role.system_prompt_override {
        config.system_prompt = prompt.clone();
    }
    if let Some(max_steps) = role.max_steps_override {
        config.max_steps = max_steps;
    }
    config.name = format!("sub-agent/{}", role.name);
}

pub fn build_spawn_tool_description(
    user_roles: &HashMap<String, AgentRoleConfig>,
) -> String {
    let built_in = built_in_roles();
    let mut seen = std::collections::HashSet::new();
    let mut lines = Vec::new();

    for (name, role) in user_roles {
        seen.insert(name.clone());
        let desc = role
            .description
            .as_deref()
            .unwrap_or("no description");
        lines.push(format!("- `{name}`: {desc}"));
    }

    for (name, role) in &built_in {
        if seen.insert(name.clone()) {
            let desc = role
                .description
                .as_deref()
                .unwrap_or("no description");
            lines.push(format!("- `{name}`: {desc}"));
        }
    }

    format!(
        "Optional role for the new agent. If omitted, `default` is used.\n\
         Available roles:\n{}",
        lines.join("\n")
    )
}
