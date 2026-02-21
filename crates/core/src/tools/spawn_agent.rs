use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agent::AgentConfig;
use crate::agent_manager::AgentManager;
use crate::agent_roles::{apply_role, resolve_role};

use super::{Tool, ToolContext, ToolResult};

pub struct SpawnAgentTool {
    manager: Arc<AgentManager>,
}

impl SpawnAgentTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        Self { manager }
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
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The initial prompt/task for the new agent"
                },
                "agent_type": {
                    "type": "string",
                    "description": "Optional role for the agent (default, explorer, worker, reviewer)"
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

        let role = resolve_role(role_name, &std::collections::HashMap::new());

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

        match self
            .manager
            .spawn_agent(
                message.to_string(),
                role_name.map(String::from),
                ctx.depth,
                ctx,
                agent_config,
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
