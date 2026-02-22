use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use super::{Tool, ToolContext, ToolResult};

pub struct TailFileTool;

#[async_trait]
impl Tool for TailFileTool {
    fn name(&self) -> &str {
        "tail_file"
    }

    fn description(&self) -> &str {
        "Read the last N lines of a file. Useful for inspecting the end of large tool outputs, \
         logs, or any file without reading the entire contents. Defaults to 50 lines."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to read."
                },
                "lines": {
                    "type": "integer",
                    "description": "Number of lines to read from the end. Default: 50."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;
        let n = args
            .get("lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(50) as usize;

        let file = std::fs::File::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to open {path}: {e}"))?;

        let metadata = file.metadata()?;
        let file_size = metadata.len();

        if file_size == 0 {
            return Ok(ToolResult {
                output: "(empty file)".to_string(),
                title: format!("tail {path}"),
                metadata: serde_json::json!({"lines": 0}),
            });
        }

        let total_lines = BufReader::new(std::fs::File::open(path)?).lines().count();
        let output = tail_lines(path, n)?;
        let shown = output.lines().count();

        let header = if total_lines > n {
            format!("... ({total_lines} total lines, showing last {shown})\n")
        } else {
            String::new()
        };

        Ok(ToolResult {
            output: format!("{header}{output}"),
            title: format!("tail -n {n} {path}"),
            metadata: serde_json::json!({
                "total_lines": total_lines,
                "shown": shown,
            }),
        })
    }
}

fn tail_lines(path: &str, n: usize) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let file_size = file.metadata()?.len();

    if file_size == 0 {
        return Ok(String::new());
    }

    let chunk_size: u64 = 8192;
    let mut lines = Vec::new();
    let mut remaining = Vec::new();
    let mut pos = file_size;

    loop {
        let read_size = chunk_size.min(pos);
        pos -= read_size;
        file.seek(SeekFrom::Start(pos))?;
        let mut buf = vec![0u8; read_size as usize];
        file.read_exact(&mut buf)?;
        buf.extend_from_slice(&remaining);
        remaining = Vec::new();

        let text = String::from_utf8_lossy(&buf);
        let mut chunk_lines: Vec<&str> = text.lines().collect();

        if pos > 0 && !chunk_lines.is_empty() {
            remaining = chunk_lines.remove(0).as_bytes().to_vec();
        }

        for line in chunk_lines.into_iter().rev() {
            lines.push(line.to_string());
            if lines.len() >= n {
                break;
            }
        }

        if lines.len() >= n || pos == 0 {
            break;
        }
    }

    if !remaining.is_empty() && lines.len() < n {
        lines.push(String::from_utf8_lossy(&remaining).to_string());
    }

    lines.reverse();
    lines.truncate(n);
    Ok(lines.join("\n"))
}
