use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agent::AgentConfig;
use crate::agent_manager::AgentManager;
use crate::agent_roles::{apply_role, resolve_role, AgentRoleConfig};

use super::{Tool, ToolContext, ToolResult};

pub struct SpawnAgentTool {
    manager: Arc<AgentManager>,
    user_roles: HashMap<String, AgentRoleConfig>,
}

impl SpawnAgentTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        Self {
            manager,
            user_roles: HashMap::new(),
        }
    }

    pub fn with_user_roles(
        manager: Arc<AgentManager>,
        user_roles: HashMap<String, AgentRoleConfig>,
    ) -> Self {
        Self {
            manager,
            user_roles,
        }
    }
}

#[async_trait]
impl Tool for SpawnAgentTool {
    fn name(&self) -> &str {
        "spawn_agent"
    }

    fn description(&self) -> &str {
        "Spawn a sub-agent for a well-scoped task. Returns the agent id to use \
         to communicate with this agent. Use for research, analysis, or \
         implementation tasks that benefit from focused attention."
    }

    fn parameters_schema(&self) -> Value {
        let role_desc = crate::agent_roles::build_spawn_tool_description(&self.user_roles);
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The initial prompt/task for the new agent"
                },
                "agent_type": {
                    "type": "string",
                    "description": role_desc
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("spawn_agent requires 'message' parameter"))?;

        if message.trim().is_empty() {
            return Ok(ToolResult {
                output: "Error: empty message can't be sent to an agent".to_string(),
                title: "spawn_agent (error)".to_string(),
                metadata: json!({ "error": "empty_message" }),
            });
        }

        let role_name = args
            .get("agent_type")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let role = resolve_role(role_name, &self.user_roles);

        let mut agent_config = AgentConfig {
            name: format!("sub-agent/{}", role.name),
            system_prompt: role
                .system_prompt_override
                .clone()
                .unwrap_or_else(|| {
                    "You are a focused sub-agent. Complete the assigned task \
                     thoroughly and return your findings. Be concise but complete. \
                     You have access to all standard tools."
                        .to_string()
                }),
            max_steps: 50,
            max_tokens: None,
            trust: nyzhi_config::TrustConfig {
                mode: nyzhi_config::TrustMode::Full,
                ..Default::default()
            },
            retry: nyzhi_config::RetrySettings::default(),
            routing: nyzhi_config::RoutingConfig::default(),
            auto_compact_threshold: None,
        };

        apply_role(&mut agent_config, &role);

        let tool_filter = compute_tool_filter(&role);

        match self
            .manager
            .spawn_agent(
                message.to_string(),
                role_name.map(String::from),
                ctx.depth,
                ctx,
                agent_config,
                tool_filter,
            )
            .await
        {
            Ok((agent_id, nickname)) => {
                let result = json!({
                    "agent_id": agent_id,
                    "agent_nickname": nickname,
                    "role": role_name.unwrap_or("default"),
                });
                Ok(ToolResult {
                    output: serde_json::to_string(&result).unwrap_or_default(),
                    title: format!("spawn_agent -> {nickname}"),
                    metadata: result,
                })
            }
            Err(e) => Ok(ToolResult {
                output: format!("Error spawning agent: {e}"),
                title: "spawn_agent (error)".to_string(),
                metadata: json!({ "error": e.to_string() }),
            }),
        }
    }
}

/// Compute the effective tool filter from a role's allowed/disallowed lists.
/// Returns None (no filtering) if neither list is set.
fn compute_tool_filter(role: &AgentRoleConfig) -> Option<Vec<String>> {
    let all_tools: Vec<String> = vec![
        "bash", "read", "write", "edit", "glob", "grep",
        "git_status", "git_diff", "git_log", "git_show", "git_branch",
        "git_commit", "git_checkout",
        "list_dir", "directory_tree", "file_info",
        "delete_file", "move_file", "copy_file", "create_dir",
        "todowrite", "todoread",
        "verify", "notepad_write", "notepad_read",
        "lsp_diagnostics", "ast_search",
    ].into_iter().map(String::from).collect();

    match (&role.allowed_tools, &role.disallowed_tools) {
        (Some(allowed), _) => {
            let mut names = allowed.clone();
            if let Some(denied) = &role.disallowed_tools {
                let deny_set: std::collections::HashSet<&str> =
                    denied.iter().map(|s| s.as_str()).collect();
                names.retain(|n| !deny_set.contains(n.as_str()));
            }
            Some(names)
        }
        (None, Some(denied)) => {
            let deny_set: std::collections::HashSet<&str> =
                denied.iter().map(|s| s.as_str()).collect();
            let filtered: Vec<String> = all_tools
                .into_iter()
                .filter(|n| !deny_set.contains(n.as_str()))
                .collect();
            Some(filtered)
        }
        (None, None) => None,
    }
}
