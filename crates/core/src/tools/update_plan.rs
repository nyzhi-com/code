use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::tools::permission::ToolPermission;

pub struct UpdatePlanTool;

#[async_trait]
impl Tool for UpdatePlanTool {
    fn name(&self) -> &str {
        "update_plan"
    }

    fn description(&self) -> &str {
        "Update the current execution plan. Use this to track progress, add/remove steps, \
         or refine the plan during execution. The plan is saved to .nyzhi/plans/."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "plan_name": {
                    "type": "string",
                    "description": "Name for the plan (used as filename)"
                },
                "plan_content": {
                    "type": "string",
                    "description": "Full plan content in Markdown. Use checkboxes (- [ ] / - [x]) for steps."
                }
            },
            "required": ["plan_name", "plan_content"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::ReadOnly
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let name = args
            .get("plan_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: plan_name"))?;
        let content = args
            .get("plan_content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: plan_content"))?;

        let dir = ctx.project_root.join(".nyzhi").join("plans");
        std::fs::create_dir_all(&dir)?;

        let safe_name: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let path = dir.join(format!("{safe_name}.md"));
        std::fs::write(&path, content)?;

        let completed = content.matches("- [x]").count();
        let total = completed + content.matches("- [ ]").count();

        Ok(ToolResult {
            output: format!(
                "Plan '{}' saved ({}/{} steps complete)\nPath: {}",
                name,
                completed,
                total,
                path.display()
            ),
            title: format!("update_plan: {name}"),
            metadata: json!({
                "plan_name": name,
                "completed_steps": completed,
                "total_steps": total,
                "path": path.to_string_lossy(),
            }),
        })
    }
}
