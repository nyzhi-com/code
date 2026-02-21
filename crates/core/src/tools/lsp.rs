use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};

use super::{Tool, ToolContext, ToolResult};

pub struct LspDiagnosticsTool;

#[async_trait]
impl Tool for LspDiagnosticsTool {
    fn name(&self) -> &str {
        "lsp_diagnostics"
    }

    fn description(&self) -> &str {
        "Detect available LSP language servers and list diagnostics capabilities. \
         Shows which servers are installed for the current project."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let servers = crate::lsp::detect_available_servers();

        let output = if servers.is_empty() {
            "No LSP servers detected.\n\nInstall one of:\n  \
             - rust-analyzer (Rust)\n  \
             - typescript-language-server (TypeScript/JavaScript)\n  \
             - pylsp (Python)\n  \
             - gopls (Go)".to_string()
        } else {
            let lines: Vec<String> = servers
                .iter()
                .map(|(lang, cmd)| format!("  {lang}: {cmd}"))
                .collect();
            format!("Available LSP servers:\n{}", lines.join("\n"))
        };

        Ok(ToolResult {
            output,
            title: "lsp_diagnostics".to_string(),
            metadata: json!({"servers": servers}),
        })
    }
}

pub struct AstSearchTool;

#[async_trait]
impl Tool for AstSearchTool {
    fn name(&self) -> &str {
        "ast_search"
    }

    fn description(&self) -> &str {
        "Structural code search using pattern matching. Searches for function signatures, \
         struct/class definitions, impl blocks, imports, and other structural patterns. \
         More targeted than text grep for code structure queries."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern_type": {
                    "type": "string",
                    "enum": ["function", "struct", "impl", "import", "class", "enum", "trait", "interface"],
                    "description": "Type of code structure to search for"
                },
                "name": {
                    "type": "string",
                    "description": "Name pattern (supports * wildcards)"
                },
                "path": {
                    "type": "string",
                    "description": "File or directory to search in (default: project root)"
                },
                "language": {
                    "type": "string",
                    "description": "Language hint: rust, typescript, python, go"
                }
            },
            "required": ["pattern_type"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        let pattern_type = args["pattern_type"].as_str().unwrap_or("function");
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("*");
        let path = args.get("path").and_then(|v| v.as_str())
            .map(|p| ctx.cwd.join(p))
            .unwrap_or_else(|| ctx.project_root.clone());

        let regex_pattern = build_structural_regex(pattern_type, name);

        let rg_output = tokio::process::Command::new("rg")
            .args(["--line-number", "--no-heading", "-e", &regex_pattern])
            .arg(&path)
            .output()
            .await;

        let output = match rg_output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.is_empty() {
                    format!("No {pattern_type} matching '{name}' found")
                } else {
                    let lines: Vec<&str> = stdout.lines().take(50).collect();
                    let truncated = if stdout.lines().count() > 50 { "\n... (truncated)" } else { "" };
                    format!("{}{truncated}", lines.join("\n"))
                }
            }
            Err(_) => "ripgrep (rg) not found - required for ast_search".to_string(),
        };

        Ok(ToolResult {
            output,
            title: "ast_search".to_string(),
            metadata: json!({"pattern_type": pattern_type, "name": name}),
        })
    }
}

fn build_structural_regex(pattern_type: &str, name: &str) -> String {
    let name_regex = if name == "*" {
        r"\w+".to_string()
    } else {
        name.replace('*', r"\w*")
    };

    match pattern_type {
        "function" | "fn" => format!(r"(pub\s+)?(async\s+)?fn\s+{name_regex}\s*[<(]"),
        "struct" => format!(r"(pub\s+)?struct\s+{name_regex}\s*[<{{(]"),
        "enum" => format!(r"(pub\s+)?enum\s+{name_regex}\s*[<{{]"),
        "trait" => format!(r"(pub\s+)?trait\s+{name_regex}\s*[<:{{]"),
        "impl" => format!(r"impl\s+(<.*>\s+)?{name_regex}"),
        "import" => format!(r"(use|import|from)\s+.*{name_regex}"),
        "class" => format!(r"(export\s+)?(abstract\s+)?class\s+{name_regex}"),
        "interface" => format!(r"(export\s+)?interface\s+{name_regex}"),
        _ => name_regex.to_string(),
    }
}
