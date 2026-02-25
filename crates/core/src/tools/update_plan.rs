use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::planning::{self, PlanFile, PlanFrontmatter, PlanTodo, TodoStatus};
use crate::tools::permission::ToolPermission;

pub struct CreatePlanTool;

#[async_trait]
impl Tool for CreatePlanTool {
    fn name(&self) -> &str {
        "create_plan"
    }

    fn description(&self) -> &str {
        "Create or update the execution plan for this session. The plan is saved as a \
         .plan.md file with YAML frontmatter containing todos. On subsequent calls, \
         todos are merged by id and the body is replaced."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Short title for the plan"
                },
                "overview": {
                    "type": "string",
                    "description": "1-2 sentence summary of what the plan achieves"
                },
                "plan": {
                    "type": "string",
                    "description": "Full plan content in Markdown"
                },
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "content": { "type": "string" },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed", "cancelled"]
                            }
                        },
                        "required": ["id", "content"]
                    },
                    "description": "Task checklist items for the plan"
                }
            },
            "required": ["name", "plan", "todos"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::ReadOnly
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: name"))?;
        let overview = args
            .get("overview")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let body = args
            .get("plan")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: plan"))?;

        let new_todos: Vec<PlanTodo> = if let Some(arr) = args.get("todos").and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|item| {
                    let id = item.get("id")?.as_str()?;
                    let content = item.get("content")?.as_str()?;
                    let status = match item.get("status").and_then(|v| v.as_str()) {
                        Some("in_progress") => TodoStatus::InProgress,
                        Some("completed") => TodoStatus::Completed,
                        Some("cancelled") => TodoStatus::Cancelled,
                        _ => TodoStatus::Pending,
                    };
                    Some(PlanTodo {
                        id: id.to_string(),
                        content: content.to_string(),
                        status,
                    })
                })
                .collect()
        } else {
            vec![]
        };

        let existing = planning::load_session_plan(&ctx.project_root, &ctx.session_id)?;

        let todos = if let Some(ref existing) = existing {
            let mut merged = existing.frontmatter.todos.clone();
            for new_todo in &new_todos {
                if let Some(pos) = merged.iter().position(|t| t.id == new_todo.id) {
                    merged[pos] = new_todo.clone();
                } else {
                    merged.push(new_todo.clone());
                }
            }
            merged
        } else {
            new_todos
        };

        let plan = PlanFile {
            frontmatter: PlanFrontmatter {
                name: name.to_string(),
                overview: overview.to_string(),
                todos,
            },
            body: body.to_string(),
        };

        let (done, total) = plan.progress();
        let path = planning::save_session_plan(&ctx.project_root, &ctx.session_id, &plan)?;

        Ok(ToolResult {
            output: format!(
                "Plan '{}' saved ({}/{} steps complete)\nPath: {}",
                name, done, total, path.display()
            ),
            title: format!("create_plan: {name}"),
            metadata: json!({
                "plan_name": name,
                "completed_steps": done,
                "total_steps": total,
                "path": path.to_string_lossy(),
            }),
        })
    }
}
