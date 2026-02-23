use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{Tool, ToolContext, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocked_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<String>,
}

type TodoStore = Arc<Mutex<HashMap<String, Vec<TodoItem>>>>;

pub struct TodoWriteTool {
    store: TodoStore,
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoWriteTool {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_store(store: TodoStore) -> Self {
        Self { store }
    }
}

pub struct TodoReadTool {
    store: TodoStore,
}

impl Default for TodoReadTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoReadTool {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_store(store: TodoStore) -> Self {
        Self { store }
    }
}

pub fn shared_store() -> TodoStore {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn has_incomplete_todos(store: &TodoStore, session_id: &str) -> bool {
    let store = store.lock().await;
    store
        .get(session_id)
        .map(|items| {
            items.iter().any(|t| t.status == "pending" || t.status == "in_progress")
        })
        .unwrap_or(false)
}

pub async fn incomplete_summary(store: &TodoStore, session_id: &str) -> Option<String> {
    let store = store.lock().await;
    let items = store.get(session_id)?;
    let completed_ids: std::collections::HashSet<&str> = items
        .iter()
        .filter(|t| t.status == "completed")
        .map(|t| t.id.as_str())
        .collect();

    let incomplete: Vec<&TodoItem> = items
        .iter()
        .filter(|t| t.status == "pending" || t.status == "in_progress")
        .collect();
    if incomplete.is_empty() {
        return None;
    }
    let lines: Vec<String> = incomplete
        .iter()
        .map(|t| {
            let is_blocked = !t.blocked_by.is_empty()
                && !t.blocked_by.iter().all(|dep| completed_ids.contains(dep.as_str()));
            let mut line = format!("[{}] {}: {}", t.status, t.id, t.content);
            if is_blocked {
                let pending_deps: Vec<&str> = t.blocked_by.iter()
                    .filter(|dep| !completed_ids.contains(dep.as_str()))
                    .map(|s| s.as_str())
                    .collect();
                line.push_str(&format!(" [BLOCKED by: {}]", pending_deps.join(", ")));
            } else if !t.blocked_by.is_empty() {
                line.push_str(" [READY]");
            }
            line
        })
        .collect();
    Some(lines.join("\n"))
}

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "todowrite"
    }

    fn description(&self) -> &str {
        "Create or update a structured todo list for tracking tasks. \
         Provide an array of todo items with id, content, and status (pending/in_progress/completed/cancelled)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
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
                            },
                            "blocked_by": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "IDs of todos that must complete before this one can start"
                            },
                            "blocks": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "IDs of todos that this todo blocks"
                            }
                        },
                        "required": ["id", "content", "status"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let todos: Vec<TodoItem> = serde_json::from_value(
            args.get("todos")
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Missing required parameter: todos"))?,
        )?;

        let mut store = self.store.lock().await;
        let session_todos = store.entry(ctx.session_id.clone()).or_default();

        for item in &todos {
            if let Some(existing) = session_todos.iter_mut().find(|t| t.id == item.id) {
                existing.content = item.content.clone();
                existing.status = item.status.clone();
            } else {
                session_todos.push(item.clone());
            }
        }

        let summary: Vec<String> = session_todos
            .iter()
            .map(|t| {
                let mut line = format!("[{}] {}: {}", t.status, t.id, t.content);
                if !t.blocked_by.is_empty() {
                    line.push_str(&format!(" (blocked by: {})", t.blocked_by.join(", ")));
                }
                if !t.blocks.is_empty() {
                    line.push_str(&format!(" (blocks: {})", t.blocks.join(", ")));
                }
                line
            })
            .collect();

        Ok(ToolResult {
            output: summary.join("\n"),
            title: "todowrite".to_string(),
            metadata: json!({ "count": session_todos.len() }),
        })
    }
}

#[async_trait]
impl Tool for TodoReadTool {
    fn name(&self) -> &str {
        "todoread"
    }

    fn description(&self) -> &str {
        "Read the current todo list for this session."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let store = self.store.lock().await;
        let todos = store.get(&ctx.session_id);

        let output = match todos {
            Some(items) if !items.is_empty() => {
                items
                    .iter()
                    .map(|t| {
                        let mut line = format!("[{}] {}: {}", t.status, t.id, t.content);
                        if !t.blocked_by.is_empty() {
                            line.push_str(&format!(" (blocked by: {})", t.blocked_by.join(", ")));
                        }
                        if !t.blocks.is_empty() {
                            line.push_str(&format!(" (blocks: {})", t.blocks.join(", ")));
                        }
                        line
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            _ => "No todos found".to_string(),
        };

        Ok(ToolResult {
            output,
            title: "todoread".to_string(),
            metadata: json!({}),
        })
    }
}
