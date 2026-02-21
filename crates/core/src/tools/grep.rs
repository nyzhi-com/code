use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;

use super::{Tool, ToolContext, ToolResult};

const MAX_MATCHES: usize = 500;
const MAX_LINE_LEN: usize = 500;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Search file contents using a regex pattern. Returns matching lines with file paths and line numbers."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in (default: working directory)"
                },
                "include": {
                    "type": "string",
                    "description": "Glob pattern to filter files (e.g. '*.rs', '*.ts')"
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

        let re = Regex::new(pattern)
            .map_err(|e| anyhow::anyhow!("Invalid regex pattern: {e}"))?;

        let base = args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|p| resolve_path(p, &ctx.cwd))
            .unwrap_or_else(|| ctx.cwd.clone());

        let include = args.get("include").and_then(|v| v.as_str());

        let include_glob = include
            .map(|pat| {
                let full = base.join("**").join(pat);
                ::glob::Pattern::new(&full.to_string_lossy())
            })
            .transpose()
            .map_err(|e| anyhow::anyhow!("Invalid include pattern: {e}"))?;

        let mut results = Vec::new();
        search_dir(&base, &re, &include_glob, &mut results, MAX_MATCHES)?;

        let count = results.len();
        let output = if results.is_empty() {
            "No matches found".to_string()
        } else {
            results.join("\n")
        };

        Ok(ToolResult {
            output,
            title: format!("grep: {pattern}"),
            metadata: json!({ "match_count": count }),
        })
    }
}

fn search_dir(
    dir: &Path,
    re: &Regex,
    include: &Option<::glob::Pattern>,
    results: &mut Vec<String>,
    max: usize,
) -> Result<()> {
    if results.len() >= max || !dir.exists() {
        return Ok(());
    }

    if dir.is_file() {
        search_file(dir, re, include, results, max)?;
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        if results.len() >= max {
            break;
        }
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') || name_str == "node_modules" || name_str == "target" {
            continue;
        }

        if path.is_dir() {
            search_dir(&path, re, include, results, max)?;
        } else if path.is_file() {
            search_file(&path, re, include, results, max)?;
        }
    }
    Ok(())
}

fn search_file(
    path: &Path,
    re: &Regex,
    include: &Option<::glob::Pattern>,
    results: &mut Vec<String>,
    max: usize,
) -> Result<()> {
    if let Some(pat) = include {
        if !pat.matches_path(path) {
            return Ok(());
        }
    }

    let content = match std::fs::read(path) {
        Ok(data) => data,
        Err(_) => return Ok(()),
    };

    if content.len() > 512 && content[..512].contains(&0) {
        return Ok(());
    }

    let text = String::from_utf8_lossy(&content);
    for (line_num, line) in text.lines().enumerate() {
        if results.len() >= max {
            break;
        }
        if re.is_match(line) {
            let display_line = if line.len() > MAX_LINE_LEN {
                format!("{}...", &line[..MAX_LINE_LEN])
            } else {
                line.to_string()
            };
            results.push(format!("{}:{}:{}", path.display(), line_num + 1, display_line));
        }
    }
    Ok(())
}

fn resolve_path(file_path: &str, cwd: &Path) -> std::path::PathBuf {
    let p = Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}
