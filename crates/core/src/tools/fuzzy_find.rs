use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use super::{Tool, ToolContext, ToolResult};
use crate::tools::permission::ToolPermission;

pub struct FuzzyFindTool;

#[async_trait]
impl Tool for FuzzyFindTool {
    fn name(&self) -> &str {
        "fuzzy_find"
    }

    fn description(&self) -> &str {
        "Fast fuzzy filename search across the project. Finds files whose path matches \
         a fuzzy query (e.g. 'agmod' matches 'agent/mod.rs'). Returns ranked results."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Fuzzy search query for filename/path matching"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum results to return (default: 20)",
                    "default": 20
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
            .unwrap_or(20) as usize;

        let mut all_files = Vec::new();
        collect_files(&ctx.project_root, &ctx.project_root, &mut all_files, 10_000);

        let query_lower = query.to_lowercase();
        let query_chars: Vec<char> = query_lower.chars().collect();

        let mut scored: Vec<(String, i64)> = all_files
            .into_iter()
            .filter_map(|path| {
                let score = fuzzy_score(&query_chars, &path.to_lowercase());
                if score > 0 {
                    Some((path, score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(max_results);

        let count = scored.len();
        let output = if scored.is_empty() {
            "No files matched the query.".to_string()
        } else {
            scored
                .iter()
                .map(|(path, score)| format!("{path}  (score: {score})"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        Ok(ToolResult {
            output,
            title: format!("fuzzy_find: {query}"),
            metadata: json!({ "match_count": count }),
        })
    }
}

fn fuzzy_score(query: &[char], target: &str) -> i64 {
    if query.is_empty() {
        return 0;
    }
    let target_chars: Vec<char> = target.chars().collect();
    let mut qi = 0;
    let mut score: i64 = 0;
    let mut last_match: Option<usize> = None;
    let mut consecutive = 0i64;

    for (ti, &tc) in target_chars.iter().enumerate() {
        if qi < query.len() && tc == query[qi] {
            score += 1;
            if let Some(prev) = last_match {
                if ti == prev + 1 {
                    consecutive += 1;
                    score += consecutive * 2;
                } else {
                    consecutive = 0;
                }
            }
            if ti > 0 && matches!(target_chars[ti - 1], '/' | '_' | '-' | '.') {
                score += 5;
            }
            last_match = Some(ti);
            qi += 1;
        }
    }

    if qi < query.len() {
        return 0;
    }

    let basename = target.rsplit('/').next().unwrap_or(target);
    if basename
        .to_lowercase()
        .contains(&query.iter().collect::<String>())
    {
        score += 10;
    }

    score
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit || !dir.is_dir() {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if out.len() >= limit {
            return;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.')
            || name_str == "node_modules"
            || name_str == "target"
            || name_str == "__pycache__"
            || name_str == ".git"
        {
            continue;
        }

        if path.is_dir() {
            collect_files(root, &path, out, limit);
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                out.push(rel.to_string_lossy().to_string());
            }
        }
    }
}
