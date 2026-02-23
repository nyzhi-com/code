use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::{Tool, ToolContext, ToolResult};

pub struct MemoryReadTool;

#[async_trait]
impl Tool for MemoryReadTool {
    fn name(&self) -> &str {
        "memory_read"
    }

    fn description(&self) -> &str {
        "Read from persistent project memory. Without a topic, returns the MEMORY.md index. \
         With a topic name, returns that topic file's content. Memory persists across sessions."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "topic": {
                    "type": "string",
                    "description": "Optional topic name (e.g. 'api-conventions', 'debugging'). Omit to read the index."
                }
            }
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let topic = args.get("topic").and_then(|v| v.as_str());

        match topic {
            Some(t) => {
                let content = crate::memory::read_topic(&ctx.project_root, t)?;
                Ok(ToolResult {
                    output: content,
                    title: format!("memory_read({t})"),
                    metadata: serde_json::json!({"topic": t}),
                })
            }
            None => {
                let index = crate::memory::read_index(&ctx.project_root)?;
                let topics = crate::memory::list_topics(&ctx.project_root);
                Ok(ToolResult {
                    output: format!(
                        "{index}\n\nAvailable topics: {}",
                        if topics.is_empty() {
                            "(none)".to_string()
                        } else {
                            topics.join(", ")
                        }
                    ),
                    title: "memory_read (index)".to_string(),
                    metadata: serde_json::json!({"topics": topics}),
                })
            }
        }
    }
}

pub struct MemoryWriteTool;

#[async_trait]
impl Tool for MemoryWriteTool {
    fn name(&self) -> &str {
        "memory_write"
    }

    fn description(&self) -> &str {
        "Write to persistent project memory. Creates or appends to a topic file and updates \
         the MEMORY.md index. Use 'replace' mode to overwrite instead of append. \
         Memory persists across sessions."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "topic": {
                    "type": "string",
                    "description": "Topic name (e.g. 'api-conventions', 'debugging', 'project-structure')."
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the topic."
                },
                "mode": {
                    "type": "string",
                    "enum": ["append", "replace"],
                    "description": "Write mode. 'append' (default) adds to existing content. 'replace' overwrites."
                }
            },
            "required": ["topic", "content"]
        })
    }

    fn permission(&self) -> super::permission::ToolPermission {
        super::permission::ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let topic = args
            .get("topic")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: topic"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;
        let replace = args
            .get("mode")
            .and_then(|v| v.as_str())
            .map(|m| m == "replace")
            .unwrap_or(false);

        let path = crate::memory::write_topic(&ctx.project_root, topic, content, replace)?;
        let mode_str = if replace { "replaced" } else { "appended" };

        Ok(ToolResult {
            output: format!("Memory {mode_str} to topic '{topic}' at {}", path.display()),
            title: format!("memory_write({topic})"),
            metadata: serde_json::json!({
                "topic": topic,
                "mode": mode_str,
                "path": path.display().to_string(),
            }),
        })
    }
}
