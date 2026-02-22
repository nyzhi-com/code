use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::{Tool, ToolContext, ToolResult};

/// Lightweight index entry for deferred tools.
#[derive(Debug, Clone)]
pub struct DeferredToolEntry {
    pub name: String,
    pub description: String,
}

/// Shared handle to the deferred tool index.
pub type DeferredToolIndex = std::sync::Arc<std::sync::RwLock<Vec<DeferredToolEntry>>>;

pub fn shared_deferred_index() -> DeferredToolIndex {
    std::sync::Arc::new(std::sync::RwLock::new(Vec::new()))
}

pub struct ToolSearchTool {
    index: DeferredToolIndex,
}

impl ToolSearchTool {
    pub fn new(index: DeferredToolIndex) -> Self {
        Self { index }
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "tool_search"
    }

    fn description(&self) -> &str {
        "Search for available tools by name or capability description. Use this to discover \
         MCP tools and other deferred tools that are not loaded by default. Returns matching \
         tool names and descriptions."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query: keywords to match against tool names and descriptions."
                },
                "search_type": {
                    "type": "string",
                    "enum": ["keyword", "regex"],
                    "description": "Search type. 'keyword' (default): case-insensitive keyword match. 'regex': regex pattern match."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?;
        let search_type = args
            .get("search_type")
            .and_then(|v| v.as_str())
            .unwrap_or("keyword");

        let index = self.index.read().map_err(|e| anyhow::anyhow!("Lock error: {e}"))?;

        if index.is_empty() {
            return Ok(ToolResult {
                output: "No deferred tools available. All tools are already loaded.".to_string(),
                title: "tool_search".to_string(),
                metadata: serde_json::json!({"matches": 0}),
            });
        }

        let matches: Vec<&DeferredToolEntry> = match search_type {
            "regex" => {
                let re = regex::Regex::new(query)
                    .map_err(|e| anyhow::anyhow!("Invalid regex: {e}"))?;
                index
                    .iter()
                    .filter(|e| re.is_match(&e.name) || re.is_match(&e.description))
                    .collect()
            }
            _ => {
                let q = query.to_lowercase();
                let keywords: Vec<&str> = q.split_whitespace().collect();
                index
                    .iter()
                    .filter(|e| {
                        let haystack = format!("{} {}", e.name, e.description).to_lowercase();
                        keywords.iter().all(|kw| haystack.contains(kw))
                    })
                    .collect()
            }
        };

        if matches.is_empty() {
            return Ok(ToolResult {
                output: format!("No tools matching '{query}'. Try broader keywords."),
                title: "tool_search".to_string(),
                metadata: serde_json::json!({"matches": 0}),
            });
        }

        let mut output = format!("Found {} matching tool(s):\n\n", matches.len());
        for entry in &matches {
            output.push_str(&format!("- **{}**: {}\n", entry.name, entry.description));
        }
        output.push_str("\nUse these tools by calling them directly. They will be loaded on first use.");

        Ok(ToolResult {
            output,
            title: format!("tool_search({query})"),
            metadata: serde_json::json!({"matches": matches.len()}),
        })
    }
}
