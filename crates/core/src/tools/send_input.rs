use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agent_manager::AgentManager;

use super::{Tool, ToolContext, ToolResult};

pub struct SendInputTool {
    manager: Arc<AgentManager>,
}

impl SendInputTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for SendInputTool {
    fn name(&self) -> &str {
        "send_input"
    }

    fn description(&self) -> &str {
        "Send a message to an existing agent. Use to provide follow-up \
         instructions or additional context to a running agent."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Agent id (from spawn_agent)"
                },
                "message": {
                    "type": "string",
                    "description": "Message to send to the agent"
                }
            },
            "required": ["id", "message"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("send_input requires 'id' parameter"))?;

        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("send_input requires 'message' parameter"))?;

        if message.trim().is_empty() {
            return Ok(ToolResult {
                output: "Error: empty message can't be sent to an agent".to_string(),
                title: "send_input (error)".to_string(),
                metadata: json!({ "error": "empty_message" }),
            });
        }

        match self.manager.send_input(id, message.to_string()).await {
            Ok(()) => {
                let info = self.manager.get_agent_info(id).await;
                let nickname = info.as_ref().map(|(n, _)| n.as_str()).unwrap_or("unknown");
                Ok(ToolResult {
                    output: json!({ "status": "sent", "agent_nickname": nickname }).to_string(),
                    title: format!("send_input -> {nickname}"),
                    metadata: json!({ "agent_id": id }),
                })
            }
            Err(e) => Ok(ToolResult {
                output: format!("Error: {e}"),
                title: "send_input (error)".to_string(),
                metadata: json!({ "error": e.to_string(), "agent_id": id }),
            }),
        }
    }
}
