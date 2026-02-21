use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use super::permission::ToolPermission;
use super::{Tool, ToolContext, ToolResult};

pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Perform a string replacement in a file. The old_string must appear exactly once in the file. \
         Preserves original indentation and formatting."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file"
                },
                "old_string": {
                    "type": "string",
                    "description": "The exact string to find (must be unique in the file)"
                },
                "new_string": {
                    "type": "string",
                    "description": "The replacement string"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
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

        let old_string = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: old_string"))?;

        let new_string = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: new_string"))?;

        let path = resolve_path(file_path, &ctx.cwd);

        if !path.exists() {
            return Ok(ToolResult {
                output: format!("File not found: {}", path.display()),
                title: format!("edit: {file_path}"),
                metadata: json!({ "error": "not_found" }),
            });
        }

        let content = tokio::fs::read_to_string(&path).await?;
        let count = content.matches(old_string).count();

        if count == 0 {
            return Ok(ToolResult {
                output: "old_string not found in file".to_string(),
                title: format!("edit: {file_path}"),
                metadata: json!({ "error": "no_match" }),
            });
        }

        if count > 1 {
            return Ok(ToolResult {
                output: format!(
                    "old_string found {count} times -- it must appear exactly once. \
                     Include more surrounding context to make it unique."
                ),
                title: format!("edit: {file_path}"),
                metadata: json!({ "error": "multiple_matches", "count": count }),
            });
        }

        let new_content = content.replacen(old_string, new_string, 1);
        tokio::fs::write(&path, &new_content).await?;

        Ok(ToolResult {
            output: format!("Applied edit to {}", path.display()),
            title: format!("edit: {file_path}"),
            metadata: json!({ "applied": true }),
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
