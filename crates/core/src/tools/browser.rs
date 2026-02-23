use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::Path;

use super::{Tool, ToolContext, ToolResult};
use crate::tools::permission::ToolPermission;

pub struct BrowserOpenTool;

#[async_trait]
impl Tool for BrowserOpenTool {
    fn name(&self) -> &str {
        "browser_open"
    }

    fn description(&self) -> &str {
        "Open a URL in a headless browser and return the page content. \
         Useful for testing web applications or fetching rendered content."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to open"
                },
                "wait_ms": {
                    "type": "integer",
                    "description": "Milliseconds to wait after page load (default: 2000)",
                    "default": 2000
                }
            },
            "required": ["url"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;
        let wait_ms = args.get("wait_ms").and_then(|v| v.as_u64()).unwrap_or(2000);

        let output = run_playwright_command(&format!(
            "const page = await browser.newPage();\n\
             await page.goto('{url}');\n\
             await page.waitForTimeout({wait_ms});\n\
             const content = await page.content();\n\
             console.log(content.substring(0, 50000));\n\
             await page.close();",
        ))
        .await?;

        Ok(ToolResult {
            output,
            title: format!("browser_open: {url}"),
            metadata: json!({ "url": url }),
        })
    }
}

pub struct BrowserScreenshotTool;

#[async_trait]
impl Tool for BrowserScreenshotTool {
    fn name(&self) -> &str {
        "browser_screenshot"
    }

    fn description(&self) -> &str {
        "Take a screenshot of a URL. Returns the path to the saved screenshot image."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to screenshot"
                },
                "output_path": {
                    "type": "string",
                    "description": "Path to save the screenshot (default: .nyzhi/screenshots/<timestamp>.png)"
                },
                "full_page": {
                    "type": "boolean",
                    "description": "Capture full page (default: false)",
                    "default": false
                }
            },
            "required": ["url"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;
        let full_page = args
            .get("full_page")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let output_path = args
            .get("output_path")
            .and_then(|v| v.as_str())
            .map(|p| resolve_path(p, &ctx.cwd))
            .unwrap_or_else(|| {
                let dir = ctx.project_root.join(".nyzhi").join("screenshots");
                std::fs::create_dir_all(&dir).ok();
                let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
                dir.join(format!("{ts}.png"))
            });

        let output_str = output_path.to_string_lossy().to_string();
        let full_page_str = if full_page { "true" } else { "false" };

        let result = run_playwright_command(&format!(
            "const page = await browser.newPage();\n\
             await page.goto('{url}');\n\
             await page.waitForTimeout(2000);\n\
             await page.screenshot({{ path: '{output_str}', fullPage: {full_page_str} }});\n\
             console.log('Screenshot saved to {output_str}');\n\
             await page.close();",
        ))
        .await?;

        Ok(ToolResult {
            output: result,
            title: format!("browser_screenshot: {url}"),
            metadata: json!({ "url": url, "path": output_str }),
        })
    }
}

pub struct BrowserEvaluateTool;

#[async_trait]
impl Tool for BrowserEvaluateTool {
    fn name(&self) -> &str {
        "browser_evaluate"
    }

    fn description(&self) -> &str {
        "Open a URL in a headless browser and evaluate JavaScript in the page context. \
         Useful for testing web apps, checking DOM state, or running browser-side assertions."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "URL to open"
                },
                "script": {
                    "type": "string",
                    "description": "JavaScript to evaluate in the page context"
                }
            },
            "required": ["url", "script"]
        })
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;
        let script = args
            .get("script")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: script"))?;

        let escaped_script = script.replace('\'', "\\'").replace('\n', "\\n");
        let result = run_playwright_command(&format!(
            "const page = await browser.newPage();\n\
             await page.goto('{url}');\n\
             await page.waitForTimeout(1000);\n\
             const result = await page.evaluate(() => {{ {escaped_script} }});\n\
             console.log(JSON.stringify(result, null, 2));\n\
             await page.close();",
        ))
        .await?;

        Ok(ToolResult {
            output: result,
            title: format!("browser_evaluate: {url}"),
            metadata: json!({ "url": url }),
        })
    }
}

async fn run_playwright_command(script: &str) -> Result<String> {
    let wrapper = format!(
        "const {{ chromium }} = require('playwright');\n\
         (async () => {{\n\
           const browser = await chromium.launch({{ headless: true }});\n\
           try {{\n\
             {script}\n\
           }} finally {{\n\
             await browser.close();\n\
           }}\n\
         }})();",
    );

    let output = tokio::process::Command::new("node")
        .arg("-e")
        .arg(&wrapper)
        .output()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to run browser command. Is Node.js and playwright installed? Error: {e}"
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        Ok(format!("Browser command failed:\n{stderr}\n{stdout}"))
    } else {
        Ok(stdout)
    }
}

fn resolve_path(file_path: &str, cwd: &Path) -> std::path::PathBuf {
    let p = Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}
