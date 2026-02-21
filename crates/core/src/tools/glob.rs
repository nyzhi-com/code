use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use super::{Tool, ToolContext, ToolResult};

const MAX_RESULTS: usize = 1000;

pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern. Returns up to 1000 matching file paths."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g. '**/*.rs', 'src/**/*.ts')"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search from (default: working directory)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: pattern"))?;

        let base = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|p| resolve_path(p, &ctx.cwd))
            .unwrap_or_else(|| ctx.cwd.clone());

        let full_pattern = base.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let mut matches: Vec<String> = ::glob::glob(&pattern_str)
            .map_err(|e| anyhow::anyhow!("Invalid glob pattern: {e}"))?
            .filter_map(|entry| entry.ok())
            .filter(|p| p.is_file())
            .take(MAX_RESULTS)
            .map(|p| p.display().to_string())
            .collect();

        matches.sort();

        let count = matches.len();
        let output = if matches.is_empty() {
            "No matching files found".to_string()
        } else {
            matches.join("\n")
        };

        Ok(ToolResult {
            output,
            title: format!("glob: {pattern}"),
            metadata: json!({ "count": count }),
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
