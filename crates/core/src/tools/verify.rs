use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};
use crate::verify;

pub struct VerifyTool;

#[async_trait]
impl Tool for VerifyTool {
    fn name(&self) -> &str {
        "verify"
    }

    fn description(&self) -> &str {
        "Run project verification checks (build, test, lint). Auto-detects project type \
         or accepts custom commands. Returns structured pass/fail results with output."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "checks": {
                    "type": "array",
                    "description": "Optional custom check commands. If omitted, auto-detects from project type.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "kind": { "type": "string", "enum": ["build", "test", "lint", "custom"] },
                            "command": { "type": "string" }
                        },
                        "required": ["kind", "command"]
                    }
                }
            }
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let checks = if let Some(custom) = args.get("checks") {
            serde_json::from_value::<Vec<verify::VerifyCheck>>(custom.clone())?
        } else {
            verify::detect_checks(&ctx.project_root)
        };

        if checks.is_empty() {
            return Ok(ToolResult {
                output: "No verification checks detected for this project.".to_string(),
                title: "verify".to_string(),
                metadata: json!({"passed": true, "checks": 0}),
            });
        }

        let report = verify::run_all_checks(&checks, &ctx.cwd).await;
        let passed = report.all_passed();

        Ok(ToolResult {
            output: report.summary(),
            title: "verify".to_string(),
            metadata: json!({
                "passed": passed,
                "checks": report.checks.len(),
                "results": report.checks.iter().map(|e| json!({
                    "kind": e.kind.to_string(),
                    "command": e.command,
                    "passed": e.passed(),
                    "exit_code": e.exit_code,
                    "elapsed_ms": e.elapsed_ms,
                })).collect::<Vec<_>>(),
            }),
        })
    }
}
