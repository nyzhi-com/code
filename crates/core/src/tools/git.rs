use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;
use tokio::process::Command;

use super::permission::ToolPermission;
use super::{Tool, ToolContext, ToolResult};

const MAX_OUTPUT_BYTES: usize = 50 * 1024;

async fn run_git(args: &[&str], cwd: &Path) -> Result<(String, i32)> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await?;

    let exit_code = output.status.code().unwrap_or(-1);
    let mut out = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stderr.is_empty() && exit_code != 0 {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&stderr);
    }

    if out.len() > MAX_OUTPUT_BYTES {
        out.truncate(MAX_OUTPUT_BYTES);
        out.push_str("\n... (output truncated)");
    }

    Ok((out, exit_code))
}

// ---------------------------------------------------------------------------
// git_status (ReadOnly)
// ---------------------------------------------------------------------------

pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "Show the working tree status: current branch, staged/unstaged changes, and untracked files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let (out, code) = run_git(&["status", "--porcelain=v2", "--branch"], &ctx.project_root).await?;

        if code != 0 {
            return Ok(ToolResult {
                output: format!("git status failed (exit {code}):\n{out}"),
                title: "git status (error)".to_string(),
                metadata: json!({ "exit_code": code }),
            });
        }

        let mut branch = String::new();
        let mut ahead = 0i64;
        let mut behind = 0i64;
        let mut changes = Vec::new();

        for line in out.lines() {
            if let Some(rest) = line.strip_prefix("# branch.head ") {
                branch = rest.to_string();
            } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if let Some(a) = parts.first() {
                    ahead = a.trim_start_matches('+').parse().unwrap_or(0);
                }
                if let Some(b) = parts.get(1) {
                    behind = b.trim_start_matches('-').parse().unwrap_or(0);
                }
            } else if line.starts_with('1') || line.starts_with('2') || line.starts_with('?') || line.starts_with('u') {
                changes.push(line.to_string());
            }
        }

        let mut display = format!("Branch: {branch}");
        if ahead != 0 || behind != 0 {
            display.push_str(&format!(" (ahead {ahead}, behind {behind})"));
        }
        display.push('\n');

        if changes.is_empty() {
            display.push_str("Working tree clean.");
        } else {
            display.push_str(&format!("{} change(s):\n", changes.len()));
            let (short, _) = run_git(&["status", "--short"], &ctx.project_root).await?;
            display.push_str(&short);
        }

        Ok(ToolResult {
            output: display,
            title: "git status".to_string(),
            metadata: json!({ "branch": branch, "ahead": ahead, "behind": behind, "changes": changes.len() }),
        })
    }
}

// ---------------------------------------------------------------------------
// git_diff (ReadOnly)
// ---------------------------------------------------------------------------

pub struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }

    fn description(&self) -> &str {
        "Show changes between working tree and index (unstaged), or between index and HEAD (staged). \
         Optionally filter to a specific file path."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "staged": {
                    "type": "boolean",
                    "description": "If true, show staged (cached) changes. Default false."
                },
                "path": {
                    "type": "string",
                    "description": "Optional file path to restrict diff to."
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let staged = args.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);
        let path = args.get("path").and_then(|v| v.as_str());

        let mut git_args = vec!["diff"];
        if staged {
            git_args.push("--cached");
        }
        git_args.push("--stat");
        git_args.push("--patch");
        if let Some(p) = path {
            git_args.push("--");
            git_args.push(p);
        }

        let (out, code) = run_git(&git_args, &ctx.project_root).await?;

        if code != 0 {
            return Ok(ToolResult {
                output: format!("git diff failed (exit {code}):\n{out}"),
                title: "git diff (error)".to_string(),
                metadata: json!({ "exit_code": code }),
            });
        }

        let label = if staged { "staged" } else { "unstaged" };
        let title = match path {
            Some(p) => format!("git diff ({label}): {p}"),
            None => format!("git diff ({label})"),
        };

        if out.trim().is_empty() {
            return Ok(ToolResult {
                output: format!("No {label} changes."),
                title,
                metadata: json!({ "empty": true }),
            });
        }

        Ok(ToolResult {
            output: out,
            title,
            metadata: json!({ "staged": staged }),
        })
    }
}

// ---------------------------------------------------------------------------
// git_log (ReadOnly)
// ---------------------------------------------------------------------------

pub struct GitLogTool;

#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &str {
        "git_log"
    }

    fn description(&self) -> &str {
        "Show recent commit history. Returns hash, date, author, and message for each commit."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "count": {
                    "type": "integer",
                    "description": "Number of commits to show (default 20, max 100)."
                },
                "path": {
                    "type": "string",
                    "description": "Optional file path to filter history."
                },
                "author": {
                    "type": "string",
                    "description": "Optional author name/email filter."
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let count = args
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .min(100);
        let path = args.get("path").and_then(|v| v.as_str());
        let author = args.get("author").and_then(|v| v.as_str());

        let count_str = count.to_string();
        let mut git_args = vec![
            "log",
            "--oneline",
            "--format=%h %ad %an | %s",
            "--date=short",
            "-n",
            &count_str,
        ];

        let author_flag;
        if let Some(a) = author {
            author_flag = format!("--author={a}");
            git_args.push(&author_flag);
        }
        if let Some(p) = path {
            git_args.push("--");
            git_args.push(p);
        }

        let (out, code) = run_git(&git_args, &ctx.project_root).await?;

        if code != 0 {
            return Ok(ToolResult {
                output: format!("git log failed (exit {code}):\n{out}"),
                title: "git log (error)".to_string(),
                metadata: json!({ "exit_code": code }),
            });
        }

        if out.trim().is_empty() {
            return Ok(ToolResult {
                output: "No commits found.".to_string(),
                title: "git log".to_string(),
                metadata: json!({ "count": 0 }),
            });
        }

        let actual_count = out.lines().count();
        Ok(ToolResult {
            output: out,
            title: format!("git log ({actual_count} commits)"),
            metadata: json!({ "count": actual_count }),
        })
    }
}

// ---------------------------------------------------------------------------
// git_show (ReadOnly)
// ---------------------------------------------------------------------------

pub struct GitShowTool;

#[async_trait]
impl Tool for GitShowTool {
    fn name(&self) -> &str {
        "git_show"
    }

    fn description(&self) -> &str {
        "Show the contents of a specific commit: message, author, date, and diff."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ref": {
                    "type": "string",
                    "description": "Commit hash, branch name, tag, or other git ref to inspect."
                }
            },
            "required": ["ref"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let git_ref = args
            .get("ref")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: ref"))?;

        let (out, code) = run_git(&["show", "--stat", "--patch", git_ref], &ctx.project_root).await?;

        if code != 0 {
            return Ok(ToolResult {
                output: format!("git show failed (exit {code}):\n{out}"),
                title: format!("git show {git_ref} (error)"),
                metadata: json!({ "exit_code": code }),
            });
        }

        Ok(ToolResult {
            output: out,
            title: format!("git show {git_ref}"),
            metadata: json!({ "ref": git_ref }),
        })
    }
}

// ---------------------------------------------------------------------------
// git_branch (ReadOnly -- list only)
// ---------------------------------------------------------------------------

pub struct GitBranchTool;

#[async_trait]
impl Tool for GitBranchTool {
    fn name(&self) -> &str {
        "git_branch"
    }

    fn description(&self) -> &str {
        "List all local and remote branches. The current branch is marked with *."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "all": {
                    "type": "boolean",
                    "description": "If true, include remote-tracking branches. Default true."
                }
            },
            "required": []
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let all = args.get("all").and_then(|v| v.as_bool()).unwrap_or(true);

        let mut git_args = vec!["branch", "-v"];
        if all {
            git_args.push("-a");
        }

        let (out, code) = run_git(&git_args, &ctx.project_root).await?;

        if code != 0 {
            return Ok(ToolResult {
                output: format!("git branch failed (exit {code}):\n{out}"),
                title: "git branch (error)".to_string(),
                metadata: json!({ "exit_code": code }),
            });
        }

        let branch_count = out.lines().count();
        Ok(ToolResult {
            output: out,
            title: format!("git branch ({branch_count})"),
            metadata: json!({ "count": branch_count }),
        })
    }
}

// ---------------------------------------------------------------------------
// git_commit (NeedsApproval)
// ---------------------------------------------------------------------------

pub struct GitCommitTool;

#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }

    fn description(&self) -> &str {
        "Stage files and create a git commit. If no paths are given, stages all changes. \
         Returns the commit hash and summary."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The commit message."
                },
                "paths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional list of file paths to stage. If omitted, stages all changes (git add -A)."
                }
            },
            "required": ["message"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: message"))?;

        if message.trim().is_empty() {
            return Ok(ToolResult {
                output: "Commit message cannot be empty.".to_string(),
                title: "git commit (rejected)".to_string(),
                metadata: json!({ "error": "empty_message" }),
            });
        }

        let paths: Vec<&str> = args
            .get("paths")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect()
            })
            .unwrap_or_default();

        if paths.is_empty() {
            let (_, code) = run_git(&["add", "-A"], &ctx.project_root).await?;
            if code != 0 {
                return Ok(ToolResult {
                    output: "git add -A failed.".to_string(),
                    title: "git commit (error)".to_string(),
                    metadata: json!({ "exit_code": code }),
                });
            }
        } else {
            let mut add_args: Vec<&str> = vec!["add", "--"];
            add_args.extend(paths.iter());
            let (out, code) = run_git(&add_args, &ctx.project_root).await?;
            if code != 0 {
                return Ok(ToolResult {
                    output: format!("git add failed (exit {code}):\n{out}"),
                    title: "git commit (error)".to_string(),
                    metadata: json!({ "exit_code": code }),
                });
            }
        }

        let (status_out, _) = run_git(&["diff", "--cached", "--stat"], &ctx.project_root).await?;
        if status_out.trim().is_empty() {
            return Ok(ToolResult {
                output: "Nothing to commit (no staged changes after add).".to_string(),
                title: "git commit (no changes)".to_string(),
                metadata: json!({ "error": "nothing_to_commit" }),
            });
        }

        let (out, code) = run_git(&["commit", "-m", message], &ctx.project_root).await?;

        if code != 0 {
            return Ok(ToolResult {
                output: format!("git commit failed (exit {code}):\n{out}"),
                title: "git commit (error)".to_string(),
                metadata: json!({ "exit_code": code }),
            });
        }

        let (hash, _) = run_git(&["rev-parse", "--short", "HEAD"], &ctx.project_root).await?;

        Ok(ToolResult {
            output: format!("{}\nCommit: {}", out.trim(), hash.trim()),
            title: format!("git commit {}", hash.trim()),
            metadata: json!({ "hash": hash.trim(), "message": message }),
        })
    }
}

// ---------------------------------------------------------------------------
// git_checkout (NeedsApproval -- create/switch branches)
// ---------------------------------------------------------------------------

pub struct GitCheckoutTool;

#[async_trait]
impl Tool for GitCheckoutTool {
    fn name(&self) -> &str {
        "git_checkout"
    }

    fn description(&self) -> &str {
        "Switch to an existing branch, or create and switch to a new branch."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "branch": {
                    "type": "string",
                    "description": "Branch name to switch to or create."
                },
                "create": {
                    "type": "boolean",
                    "description": "If true, create a new branch (git checkout -b). Default false."
                }
            },
            "required": ["branch"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let branch = args
            .get("branch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: branch"))?;

        let create = args.get("create").and_then(|v| v.as_bool()).unwrap_or(false);

        let git_args = if create {
            vec!["checkout", "-b", branch]
        } else {
            vec!["checkout", branch]
        };

        let (out, code) = run_git(&git_args, &ctx.project_root).await?;

        if code != 0 {
            return Ok(ToolResult {
                output: format!("git checkout failed (exit {code}):\n{out}"),
                title: format!("git checkout {branch} (error)"),
                metadata: json!({ "exit_code": code }),
            });
        }

        let action = if create { "Created and switched to" } else { "Switched to" };
        Ok(ToolResult {
            output: format!("{action} branch '{branch}'.\n{out}"),
            title: format!("git checkout {branch}"),
            metadata: json!({ "branch": branch, "created": create }),
        })
    }
}
