use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use super::{Tool, ToolContext, ToolResult};

const DEFAULT_LIMIT: usize = 2000;
const MAX_LINE_LEN: usize = 2000;

pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read a file's contents. Returns line-numbered output. \
         Use `offset` (1-indexed line number) and `limit` (max lines, default 2000) \
         for large files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file"
                },
                "offset": {
                    "type": "integer",
                    "description": "Start reading from this line number (1-indexed)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to return (default 2000)"
                }
            },
            "required": ["file_path"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let file_path = args
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: file_path"))?;

        let path = resolve_path(file_path, &ctx.cwd);

        if !path.exists() {
            return Ok(ToolResult {
                output: format!("File not found: {}", path.display()),
                title: format!("read: {file_path}"),
                metadata: json!({ "error": "not_found" }),
            });
        }

        let raw = tokio::fs::read(&path).await?;

        if is_binary(&raw) {
            return Ok(ToolResult {
                output: format!("Binary file detected: {}", path.display()),
                title: format!("read: {file_path}"),
                metadata: json!({ "binary": true, "size": raw.len() }),
            });
        }

        let content = String::from_utf8_lossy(&raw);
        let all_lines: Vec<&str> = content.lines().collect();
        let total = all_lines.len();

        let offset = args
            .get("offset")
            .and_then(|v| v.as_u64())
            .map(|v| v.saturating_sub(1) as usize)
            .unwrap_or(0);

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_LIMIT);

        let end = (offset + limit).min(total);
        let lines = &all_lines[offset.min(total)..end];

        let mut output = String::new();
        for (i, line) in lines.iter().enumerate() {
            let line_num = offset + i + 1;
            let truncated = if line.len() > MAX_LINE_LEN {
                format!("{}... (line truncated)", &line[..MAX_LINE_LEN])
            } else {
                line.to_string()
            };
            output.push_str(&format!("{line_num:6}|{truncated}\n"));
        }

        Ok(ToolResult {
            output,
            title: format!("read: {file_path}"),
            metadata: json!({ "total_lines": total, "shown": lines.len() }),
        })
    }
}

fn is_binary(data: &[u8]) -> bool {
    let check_len = data.len().min(512);
    data[..check_len].contains(&0)
}

fn resolve_path(file_path: &str, cwd: &Path) -> std::path::PathBuf {
    let p = Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}
