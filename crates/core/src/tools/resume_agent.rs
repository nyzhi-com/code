use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agent_manager::AgentManager;

use super::{Tool, ToolContext, ToolResult};

pub struct ResumeAgentTool {
    manager: Arc<AgentManager>,
}

impl ResumeAgentTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for ResumeAgentTool {
    fn name(&self) -> &str {
        "resume_agent"
    }

    fn description(&self) -> &str {
        "Resume a previously completed or errored agent so it can receive \
         new send_input and wait calls."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "Agent id to resume"
                }
            },
            "required": ["id"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let id = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("resume_agent requires 'id' parameter"))?;

        let info = self.manager.get_agent_info(id).await;
        let nickname = info.as_ref().map(|(n, _)| n.as_str()).unwrap_or("unknown");

        match self.manager.resume_agent(id).await {
            Ok(status) => {
                let result = json!({
                    "status": status.to_string(),
                    "agent_nickname": nickname,
                });
                Ok(ToolResult {
                    output: serde_json::to_string(&result).unwrap_or_default(),
                    title: format!("resume_agent -> {nickname}"),
                    metadata: result,
                })
            }
            Err(e) => Ok(ToolResult {
                output: format!("Error: {e}"),
                title: "resume_agent (error)".to_string(),
                metadata: json!({ "error": e.to_string(), "agent_id": id }),
            }),
        }
    }
}
