use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};

pub struct NotepadWriteTool;
pub struct NotepadReadTool;

#[async_trait]
impl Tool for NotepadWriteTool {
    fn name(&self) -> &str {
        "notepad_write"
    }

    fn description(&self) -> &str {
        "Record a learning, decision, or issue in the project notepad. \
         Entries are timestamped and organized by plan name and category."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "plan": {
                    "type": "string",
                    "description": "Plan or feature name (used as directory)"
                },
                "category": {
                    "type": "string",
                    "enum": ["learnings", "decisions", "issues"],
                    "description": "Category of the entry"
                },
                "content": {
                    "type": "string",
                    "description": "The entry content to record"
                }
            },
            "required": ["plan", "category", "content"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let plan = args["plan"].as_str().unwrap_or("default");
        let category = args["category"].as_str().unwrap_or("learnings");
        let content = args["content"].as_str().unwrap_or("");

        let result = crate::notepad::append_entry(&ctx.project_root, plan, category, content)?;

        Ok(ToolResult {
            output: result,
            title: "notepad_write".to_string(),
            metadata: json!({"plan": plan, "category": category}),
        })
    }
}

#[async_trait]
impl Tool for NotepadReadTool {
    fn name(&self) -> &str {
        "notepad_read"
    }

    fn description(&self) -> &str {
        "Read the project notepad for a given plan. Returns learnings, decisions, and issues."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "plan": {
                    "type": "string",
                    "description": "Plan or feature name to read (or omit to list all)"
                }
            }
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let plan = args.get("plan").and_then(|v| v.as_str());

        let output = if let Some(plan_name) = plan {
            crate::notepad::read_notepad(&ctx.project_root, plan_name)?
        } else {
            let plans = crate::notepad::list_notepads(&ctx.project_root)?;
            if plans.is_empty() {
                "No notepads found. Use notepad_write to create one.".to_string()
            } else {
                format!(
                    "Available notepads:\n{}",
                    plans
                        .iter()
                        .map(|p| format!("  - {p}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        };

        Ok(ToolResult {
            output,
            title: "notepad_read".to_string(),
            metadata: json!({}),
        })
    }
}
