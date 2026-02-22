use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::tools::permission::ToolPermission;

pub struct ThinkTool;

#[async_trait]
impl Tool for ThinkTool {
    fn name(&self) -> &str {
        "think"
    }

    fn description(&self) -> &str {
        "Use this tool to think through complex problems step-by-step before acting. \
         The thought is logged but has no side effects. Use it to reason about \
         architecture decisions, debug hypotheses, or plan multi-step changes."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Your reasoning, analysis, or plan. Be thorough."
                }
            },
            "required": ["thought"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::ReadOnly
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let thought = args
            .get("thought")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(ToolResult {
            output: thought.to_string(),
            title: "think".to_string(),
            metadata: json!({ "type": "thinking", "length": thought.len() }),
        })
    }
}
