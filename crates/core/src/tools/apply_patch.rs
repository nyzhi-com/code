use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::{Tool, ToolContext, ToolResult};
use crate::tools::permission::ToolPermission;

pub struct ApplyPatchTool;

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a unified diff patch atomically. All hunks must succeed or the entire patch \
         is rolled back. Use for multi-file changes expressed as unified diffs."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "patch": {
                    "type": "string",
                    "description": "Unified diff string (output of `diff -u` or `git diff`)"
                }
            },
            "required": ["patch"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let patch_str = args
            .get("patch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: patch"))?;

        let hunks = parse_unified_diff(patch_str)?;
        if hunks.is_empty() {
            return Ok(ToolResult {
                output: "No hunks found in patch.".to_string(),
                title: "apply_patch: empty".to_string(),
                metadata: json!({}),
            });
        }

        let mut backups: HashMap<PathBuf, Option<String>> = HashMap::new();
        let mut files_changed = Vec::new();

        for file_hunks in &hunks {
            let file_path = resolve_path(&file_hunks.target_file, &ctx.cwd);
            if !backups.contains_key(&file_path) {
                let backup = if file_path.exists() {
                    Some(std::fs::read_to_string(&file_path)?)
                } else {
                    None
                };
                backups.insert(file_path.clone(), backup);
            }
        }

        let mut applied = 0;
        let mut errors = Vec::new();

        for file_hunks in &hunks {
            let file_path = resolve_path(&file_hunks.target_file, &ctx.cwd);
            let content = if file_path.exists() {
                std::fs::read_to_string(&file_path)?
            } else {
                String::new()
            };

            match apply_hunks_to_content(&content, &file_hunks.hunks) {
                Ok(new_content) => {
                    if let Some(parent) = file_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&file_path, &new_content)?;
                    files_changed.push(file_hunks.target_file.clone());
                    applied += file_hunks.hunks.len();

                    let mut tracker = ctx.change_tracker.blocking_lock();
                    tracker.record(super::change_tracker::FileChange {
                        path: file_path.clone(),
                        original: backups.get(&file_path).and_then(|b| b.clone()),
                        new_content: new_content.clone(),
                        tool_name: "apply_patch".to_string(),
                        timestamp: chrono::Utc::now(),
                    });
                }
                Err(e) => {
                    errors.push(format!("{}: {e}", file_hunks.target_file));
                    for (path, backup) in &backups {
                        match backup {
                            Some(content) => std::fs::write(path, content)?,
                            None => {
                                if path.exists() {
                                    std::fs::remove_file(path)?;
                                }
                            }
                        }
                    }
                    return Ok(ToolResult {
                        output: format!(
                            "Patch failed and was rolled back.\nErrors:\n{}",
                            errors.join("\n")
                        ),
                        title: "apply_patch: failed".to_string(),
                        metadata: json!({ "success": false, "errors": errors }),
                    });
                }
            }
        }

        Ok(ToolResult {
            output: format!(
                "Applied {} hunks across {} files: {}",
                applied,
                files_changed.len(),
                files_changed.join(", ")
            ),
            title: format!("apply_patch: {} files", files_changed.len()),
            metadata: json!({ "success": true, "files": files_changed, "hunks": applied }),
        })
    }
}

pub struct MultiEditTool;

#[async_trait]
impl Tool for MultiEditTool {
    fn name(&self) -> &str {
        "multi_edit"
    }

    fn description(&self) -> &str {
        "Apply multiple string-replacement edits across multiple files transactionally. \
         All edits succeed or all are rolled back."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "file": { "type": "string", "description": "File path" },
                            "replacements": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "old": { "type": "string", "description": "Text to find" },
                                        "new": { "type": "string", "description": "Replacement text" }
                                    },
                                    "required": ["old", "new"]
                                }
                            }
                        },
                        "required": ["file", "replacements"]
                    },
                    "description": "Array of per-file edit specs"
                }
            },
            "required": ["edits"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let edits = args
            .get("edits")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: edits"))?;

        let mut backups: Vec<(PathBuf, String)> = Vec::new();
        let mut results = Vec::new();

        for edit in edits {
            let file = edit
                .get("file")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Each edit must have a 'file' field"))?;
            let replacements = edit
                .get("replacements")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow::anyhow!("Each edit must have 'replacements'"))?;

            let file_path = resolve_path(file, &ctx.cwd);
            let original = std::fs::read_to_string(&file_path)
                .map_err(|e| anyhow::anyhow!("Cannot read {file}: {e}"))?;
            backups.push((file_path.clone(), original.clone()));

            let mut content = original;
            let mut count = 0;
            for replacement in replacements {
                let old = replacement
                    .get("old")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let new = replacement
                    .get("new")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if old.is_empty() {
                    continue;
                }
                if !content.contains(old) {
                    for (path, backup) in &backups {
                        std::fs::write(path, backup).ok();
                    }
                    return Ok(ToolResult {
                        output: format!("Rolled back: old_string not found in {file}:\n{old}"),
                        title: "multi_edit: failed".to_string(),
                        metadata: json!({ "success": false }),
                    });
                }
                content = content.replacen(old, new, 1);
                count += 1;
            }

            std::fs::write(&file_path, &content)?;
            let mut tracker = ctx.change_tracker.blocking_lock();
            tracker.record(super::change_tracker::FileChange {
                path: file_path.clone(),
                original: Some(backups.last().unwrap().1.clone()),
                new_content: content.clone(),
                tool_name: "multi_edit".to_string(),
                timestamp: chrono::Utc::now(),
            });
            results.push(format!("{file}: {count} replacements"));
        }

        Ok(ToolResult {
            output: format!(
                "Applied edits to {} files:\n{}",
                results.len(),
                results.join("\n")
            ),
            title: format!("multi_edit: {} files", results.len()),
            metadata: json!({ "success": true, "files": results.len() }),
        })
    }
}

struct FileHunks {
    target_file: String,
    hunks: Vec<Hunk>,
}

struct Hunk {
    old_start: usize,
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

fn parse_unified_diff(patch: &str) -> Result<Vec<FileHunks>> {
    let mut result: Vec<FileHunks> = Vec::new();
    let mut current_file: Option<FileHunks> = None;
    let mut current_hunk: Option<Hunk> = None;

    for line in patch.lines() {
        if line.starts_with("+++ ") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(f) = current_file.as_mut() {
                    f.hunks.push(hunk);
                }
            }
            if let Some(f) = current_file.take() {
                if !f.hunks.is_empty() {
                    result.push(f);
                }
            }
            let target = line[4..].trim();
            let target = target.strip_prefix("b/").unwrap_or(target);
            current_file = Some(FileHunks {
                target_file: target.to_string(),
                hunks: Vec::new(),
            });
        } else if line.starts_with("--- ") {
            continue;
        } else if line.starts_with("@@ ") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(f) = current_file.as_mut() {
                    f.hunks.push(hunk);
                }
            }
            let old_start = parse_hunk_header(line).unwrap_or(1);
            current_hunk = Some(Hunk {
                old_start,
                old_lines: Vec::new(),
                new_lines: Vec::new(),
            });
        } else if let Some(ref mut hunk) = current_hunk {
            if let Some(stripped) = line.strip_prefix('-') {
                hunk.old_lines.push(stripped.to_string());
            } else if let Some(stripped) = line.strip_prefix('+') {
                hunk.new_lines.push(stripped.to_string());
            } else {
                let context = line.strip_prefix(' ').unwrap_or(line);
                hunk.old_lines.push(context.to_string());
                hunk.new_lines.push(context.to_string());
            }
        }
    }

    if let Some(hunk) = current_hunk {
        if let Some(f) = current_file.as_mut() {
            f.hunks.push(hunk);
        }
    }
    if let Some(f) = current_file {
        if !f.hunks.is_empty() {
            result.push(f);
        }
    }

    Ok(result)
}

fn parse_hunk_header(header: &str) -> Option<usize> {
    let after_at = header.strip_prefix("@@ -")?;
    let num = after_at.split(&[',', ' '][..]).next()?;
    num.parse().ok()
}

fn apply_hunks_to_content(content: &str, hunks: &[Hunk]) -> Result<String> {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    let mut offset: isize = 0;
    for hunk in hunks {
        let start = (hunk.old_start as isize - 1 + offset) as usize;
        let end = (start + hunk.old_lines.len()).min(lines.len());

        if start > lines.len() {
            anyhow::bail!(
                "Hunk start {} exceeds file length {}",
                hunk.old_start,
                lines.len()
            );
        }

        lines.splice(start..end, hunk.new_lines.iter().cloned());
        offset += hunk.new_lines.len() as isize - hunk.old_lines.len() as isize;
    }

    Ok(lines.join("\n"))
}

fn resolve_path(file_path: &str, cwd: &Path) -> PathBuf {
    let p = Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}
