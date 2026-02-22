use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use super::{Tool, ToolContext, ToolResult};

pub struct BatchApplyTool;

#[async_trait]
impl Tool for BatchApplyTool {
    fn name(&self) -> &str {
        "batch_apply"
    }

    fn description(&self) -> &str {
        "Apply a tool operation across multiple files in parallel. Useful for mass formatting, \
         search-and-replace, or any repetitive file operation. Returns results for each file."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of file paths to operate on."
                },
                "command": {
                    "type": "string",
                    "description": "Shell command to run per file. Use {file} as placeholder."
                }
            },
            "required": ["files", "command"]
        })
    }

    fn permission(&self) -> super::permission::ToolPermission {
        super::permission::ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let files: Vec<String> = args
            .get("files")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: command"))?;

        if files.is_empty() {
            return Ok(ToolResult {
                output: "No files specified.".to_string(),
                title: "batch_apply".to_string(),
                metadata: serde_json::json!({"count": 0}),
            });
        }

        let total = files.len();
        let futs = files.iter().map(|file| {
            let cmd = command.replace("{file}", file);
            let cwd = ctx.cwd.clone();
            async move {
                let output = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .current_dir(&cwd)
                    .output()
                    .await;
                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let _stderr = String::from_utf8_lossy(&out.stderr);
                        let status = if out.status.success() { "ok" } else { "error" };
                        format!("[{status}] {file}: {}", stdout.trim().chars().take(200).collect::<String>())
                    }
                    Err(e) => format!("[error] {file}: {e}"),
                }
            }
        });

        let results = futures::future::join_all(futs).await;
        let succeeded = results.iter().filter(|r| r.starts_with("[ok]")).count();
        let failed = total - succeeded;

        let mut output = format!("Batch complete: {succeeded}/{total} succeeded");
        if failed > 0 {
            output.push_str(&format!(", {failed} failed"));
        }
        output.push_str("\n\n");
        for r in &results {
            output.push_str(r);
            output.push('\n');
        }

        Ok(ToolResult {
            output,
            title: format!("batch_apply({total} files)"),
            metadata: serde_json::json!({
                "total": total,
                "succeeded": succeeded,
                "failed": failed,
            }),
        })
    }
}
