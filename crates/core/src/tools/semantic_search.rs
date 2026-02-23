use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::tools::permission::ToolPermission;

use nyzhi_index::CodebaseIndex;
use std::sync::Arc;

pub struct SemanticSearchTool {
    index: Arc<CodebaseIndex>,
}

impl SemanticSearchTool {
    pub fn new(index: Arc<CodebaseIndex>) -> Self {
        Self { index }
    }
}

#[async_trait]
impl Tool for SemanticSearchTool {
    fn name(&self) -> &str {
        "semantic_search"
    }

    fn description(&self) -> &str {
        "Search the codebase using natural language. Finds code chunks that are \
         semantically relevant to the query, even if they don't contain the exact words. \
         Use for questions like 'where is authentication handled?' or 'database connection setup'."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language search query describing what you're looking for"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum results to return (default: 10)",
                    "default": 10
                }
            },
            "required": ["query"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::ReadOnly
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?;

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        if !self.index.is_ready() {
            return Ok(ToolResult {
                output:
                    "Index is still building. Try again in a moment, or use grep for exact matches."
                        .to_string(),
                title: format!("semantic_search: {query}"),
                metadata: json!({ "result_count": 0, "status": "building" }),
            });
        }

        let results = self.index.search(query, max_results).await?;

        let count = results.len();
        let output = if results.is_empty() {
            "No relevant results found. Try rephrasing or use grep for exact matches.".to_string()
        } else {
            results
                .into_iter()
                .enumerate()
                .map(|(i, r)| {
                    format!(
                        "{}. {} (lines {}-{}, score: {:.3})\n{}",
                        i + 1,
                        r.file,
                        r.start_line,
                        r.end_line,
                        r.score,
                        preview(&r.content, 12),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n---\n")
        };

        Ok(ToolResult {
            output,
            title: format!("semantic_search: {query}"),
            metadata: json!({ "result_count": count }),
        })
    }
}

fn preview(content: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() <= max_lines {
        content.to_string()
    } else {
        let shown: Vec<&str> = lines[..max_lines].to_vec();
        format!(
            "{}\n  ... ({} more lines)",
            shown.join("\n"),
            lines.len() - max_lines
        )
    }
}
