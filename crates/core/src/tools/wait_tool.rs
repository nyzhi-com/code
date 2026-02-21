use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::agent_manager::AgentManager;

use super::{Tool, ToolContext, ToolResult};

pub const DEFAULT_WAIT_TIMEOUT_MS: i64 = 30_000;
pub const MIN_WAIT_TIMEOUT_MS: i64 = 10_000;
pub const MAX_WAIT_TIMEOUT_MS: i64 = 300_000;

pub struct WaitTool {
    manager: Arc<AgentManager>,
}

impl WaitTool {
    pub fn new(manager: Arc<AgentManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for WaitTool {
    fn name(&self) -> &str {
        "wait"
    }

    fn description(&self) -> &str {
        "Wait for agents to reach a final status. Returns the status of the \
         first agent to complete. Completed statuses include the agent's final \
         message. Returns empty status when timed out. Prefer longer waits \
         (minutes) to avoid busy polling."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Agent ids to wait on. Pass multiple to wait for whichever finishes first."
                },
                "timeout_ms": {
                    "type": "number",
                    "description": format!(
                        "Optional timeout in milliseconds. Default {DEFAULT_WAIT_TIMEOUT_MS}, \
                         min {MIN_WAIT_TIMEOUT_MS}, max {MAX_WAIT_TIMEOUT_MS}."
                    )
                }
            },
            "required": ["ids"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let ids: Vec<String> = args
            .get("ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if ids.is_empty() {
            return Ok(ToolResult {
                output: "Error: ids must be non-empty".to_string(),
                title: "wait (error)".to_string(),
                metadata: json!({ "error": "empty_ids" }),
            });
        }

        let timeout_ms = args
            .get("timeout_ms")
            .and_then(|v| v.as_i64())
            .unwrap_or(DEFAULT_WAIT_TIMEOUT_MS);

        match self.manager.wait_any(&ids, timeout_ms).await {
            Ok((statuses, timed_out)) => {
                let status_map: serde_json::Map<String, Value> = statuses
                    .iter()
                    .map(|(id, status)| (id.clone(), json!(status.to_string())))
                    .collect();

                let result = json!({
                    "status": status_map,
                    "timed_out": timed_out,
                });

                let title = if timed_out {
                    "wait (timed out)".to_string()
                } else {
                    let completed: Vec<&String> = statuses.keys().collect();
                    format!("wait -> {} completed", completed.len())
                };

                Ok(ToolResult {
                    output: serde_json::to_string(&result).unwrap_or_default(),
                    title,
                    metadata: result,
                })
            }
            Err(e) => Ok(ToolResult {
                output: format!("Error: {e}"),
                title: "wait (error)".to_string(),
                metadata: json!({ "error": e.to_string() }),
            }),
        }
    }
}
