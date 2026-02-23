use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use super::change_tracker::FileChange;
use super::diff::{truncate_diff, unified_diff};
use super::permission::ToolPermission;
use super::{Tool, ToolContext, ToolResult};

pub struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates the file and parent directories if they don't exist. \
         Overwrites existing content."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write"
                }
            },
            "required": ["file_path", "content"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: file_path"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;

        let path = resolve_path(file_path, &ctx.cwd);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let original = if path.exists() {
            tokio::fs::read_to_string(&path).await.ok()
        } else {
            None
        };

        let bytes = content.len();
        tokio::fs::write(&path, content).await?;

        let diff = match &original {
            Some(old) => {
                let d = unified_diff(file_path, old, content, 3);
                truncate_diff(&d, 50)
            }
            None => {
                let lines: Vec<&str> = content.lines().take(20).collect();
                let preview = lines.join("\n");
                if content.lines().count() > 20 {
                    format!(
                        "{preview}\n... ({} more lines)",
                        content.lines().count() - 20
                    )
                } else {
                    preview
                }
            }
        };

        {
            let mut tracker = ctx.change_tracker.lock().await;
            tracker.record(FileChange {
                path: path.clone(),
                original,
                new_content: content.to_string(),
                tool_name: "write".to_string(),
                timestamp: chrono::Utc::now(),
            });
        }

        let output = if diff.is_empty() {
            format!("Wrote {bytes} bytes to {}", path.display())
        } else {
            format!("Wrote {bytes} bytes to {}\n\n{diff}", path.display())
        };

        Ok(ToolResult {
            output,
            title: format!("write: {file_path}"),
            metadata: json!({ "bytes_written": bytes }),
        })
    }
}

fn resolve_path(file_path: &str, cwd: &Path) -> std::path::PathBuf {
    let p = Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}
