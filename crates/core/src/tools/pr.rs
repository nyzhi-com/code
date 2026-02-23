use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::tools::permission::ToolPermission;

pub struct CreatePrTool;

#[async_trait]
impl Tool for CreatePrTool {
    fn name(&self) -> &str {
        "create_pr"
    }

    fn description(&self) -> &str {
        "Create a GitHub or GitLab pull request from the current branch. \
         Uses the `gh` CLI for GitHub or `glab` for GitLab. \
         Generates a title and description from the commit history if not provided."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "PR title (auto-generated from commits if not provided)"
                },
                "body": {
                    "type": "string",
                    "description": "PR description/body (auto-generated if not provided)"
                },
                "base": {
                    "type": "string",
                    "description": "Base branch (default: main or master)"
                },
                "draft": {
                    "type": "boolean",
                    "description": "Create as draft PR (default: false)",
                    "default": false
                }
            }
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let title = args.get("title").and_then(|v| v.as_str());
        let body = args.get("body").and_then(|v| v.as_str());
        let base = args.get("base").and_then(|v| v.as_str());
        let draft = args.get("draft").and_then(|v| v.as_bool()).unwrap_or(false);

        let branch = get_current_branch(&ctx.project_root).await?;

        let auto_title = if title.is_none() {
            Some(generate_pr_title(&ctx.project_root).await?)
        } else {
            None
        };
        let pr_title = title
            .map(|s| s.to_string())
            .or(auto_title)
            .unwrap_or_else(|| branch.clone());

        let auto_body = if body.is_none() {
            Some(generate_pr_body(&ctx.project_root).await?)
        } else {
            None
        };
        let pr_body = body
            .map(|s| s.to_string())
            .or(auto_body)
            .unwrap_or_default();

        let push_output = tokio::process::Command::new("git")
            .args(["push", "-u", "origin", "HEAD"])
            .current_dir(&ctx.project_root)
            .output()
            .await?;

        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            return Ok(ToolResult {
                output: format!("Failed to push branch: {stderr}"),
                title: "create_pr: push failed".to_string(),
                metadata: json!({ "success": false }),
            });
        }

        let mut cmd_args = vec![
            "pr".to_string(),
            "create".to_string(),
            "--title".to_string(),
            pr_title.clone(),
            "--body".to_string(),
            pr_body,
        ];
        if let Some(b) = base {
            cmd_args.extend(["--base".to_string(), b.to_string()]);
        }
        if draft {
            cmd_args.push("--draft".to_string());
        }

        let output = tokio::process::Command::new("gh")
            .args(&cmd_args)
            .current_dir(&ctx.project_root)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to run gh CLI. Is it installed? Error: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(ToolResult {
                output: format!("PR created: {}", stdout.trim()),
                title: format!("create_pr: {pr_title}"),
                metadata: json!({ "success": true, "url": stdout.trim() }),
            })
        } else {
            Ok(ToolResult {
                output: format!("Failed to create PR:\n{stderr}\n{stdout}"),
                title: "create_pr: failed".to_string(),
                metadata: json!({ "success": false }),
            })
        }
    }
}

pub struct ReviewPrTool;

#[async_trait]
impl Tool for ReviewPrTool {
    fn name(&self) -> &str {
        "review_pr"
    }

    fn description(&self) -> &str {
        "Review a PR diff and provide structured feedback. Fetches the diff \
         from a PR number or URL and returns it for analysis."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pr": {
                    "type": "string",
                    "description": "PR number or URL (e.g. '42' or 'https://github.com/owner/repo/pull/42')"
                }
            },
            "required": ["pr"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::ReadOnly
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pr = args
            .get("pr")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: pr"))?;

        let output = tokio::process::Command::new("gh")
            .args(["pr", "diff", pr])
            .current_dir(&ctx.project_root)
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to run gh CLI: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Ok(ToolResult {
                output: format!("Failed to fetch PR diff: {stderr}"),
                title: format!("review_pr: #{pr} failed"),
                metadata: json!({ "success": false }),
            });
        }

        let info_output = tokio::process::Command::new("gh")
            .args([
                "pr",
                "view",
                pr,
                "--json",
                "title,body,author,state,additions,deletions",
            ])
            .current_dir(&ctx.project_root)
            .output()
            .await?;

        let pr_info = String::from_utf8_lossy(&info_output.stdout).to_string();

        let diff_lines = stdout.lines().count();
        let additions = stdout.lines().filter(|l| l.starts_with('+')).count();
        let deletions = stdout.lines().filter(|l| l.starts_with('-')).count();

        Ok(ToolResult {
            output: format!(
                "PR Info:\n{pr_info}\n\nDiff ({diff_lines} lines, +{additions}/-{deletions}):\n{stdout}"
            ),
            title: format!("review_pr: #{pr}"),
            metadata: json!({
                "success": true,
                "diff_lines": diff_lines,
                "additions": additions,
                "deletions": deletions,
            }),
        })
    }
}

async fn get_current_branch(project_root: &std::path::Path) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(project_root)
        .output()
        .await?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn generate_pr_title(project_root: &std::path::Path) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(project_root)
        .output()
        .await?;
    let msg = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let title = msg.split_once(' ').map(|(_, t)| t).unwrap_or(&msg);
    Ok(title.to_string())
}

async fn generate_pr_body(project_root: &std::path::Path) -> Result<String> {
    let default_branch = detect_default_branch(project_root).await?;
    let output = tokio::process::Command::new("git")
        .args(["log", "--oneline", &format!("{default_branch}..HEAD")])
        .current_dir(project_root)
        .output()
        .await?;
    let commits = String::from_utf8_lossy(&output.stdout).to_string();

    let diff_stat = tokio::process::Command::new("git")
        .args(["diff", "--stat", &format!("{default_branch}..HEAD")])
        .current_dir(project_root)
        .output()
        .await?;
    let stats = String::from_utf8_lossy(&diff_stat.stdout).to_string();

    Ok(format!(
        "## Changes\n\n{commits}\n## Stats\n\n```\n{stats}```\n"
    ))
}

async fn detect_default_branch(project_root: &std::path::Path) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .current_dir(project_root)
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() => {
            let branch = String::from_utf8_lossy(&o.stdout).trim().to_string();
            Ok(branch
                .strip_prefix("origin/")
                .unwrap_or(&branch)
                .to_string())
        }
        _ => Ok("main".to_string()),
    }
}
