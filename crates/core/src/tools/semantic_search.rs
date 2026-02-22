use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::index::SemanticIndex;
use crate::tools::permission::ToolPermission;

use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SemanticSearchTool {
    index: Arc<Mutex<SemanticIndex>>,
}

impl SemanticSearchTool {
    pub fn new(index: Arc<Mutex<SemanticIndex>>) -> Self {
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

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?;

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let mut index = self.index.lock().await;

        if !index.is_built() {
            index.build(&ctx.project_root)?;
        }

        let results = index.search(query, max_results);

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
                        preview(&r.content, 8),
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
        format!("{}\n  ... ({} more lines)", shown.join("\n"), lines.len() - max_lines)
    }
}
