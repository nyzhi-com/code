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
             - gopls (Go)"
                .to_string()
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
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
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
                    let truncated = if stdout.lines().count() > 50 {
                        "\n... (truncated)"
                    } else {
                        ""
                    };
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

pub struct LspGotoDefinitionTool;

#[async_trait]
impl Tool for LspGotoDefinitionTool {
    fn name(&self) -> &str {
        "lsp_goto_definition"
    }
    fn description(&self) -> &str {
        "Find the definition of a symbol at a given file location. Returns the file and line \
         where the symbol is defined."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "Absolute path to the file." },
                "line": { "type": "integer", "description": "1-based line number." },
                "column": { "type": "integer", "description": "1-based column number." }
            },
            "required": ["file", "line", "column"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: file"))?;
        let line = args.get("line").and_then(|v| v.as_u64()).unwrap_or(1);
        let col = args.get("column").and_then(|v| v.as_u64()).unwrap_or(1);

        let content = tokio::fs::read_to_string(file).await?;
        let target_line = content.lines().nth((line - 1) as usize).unwrap_or("");
        let word = extract_word_at(target_line, (col - 1) as usize);

        if word.is_empty() {
            return Ok(ToolResult {
                output: format!("No symbol found at {file}:{line}:{col}"),
                title: "lsp_goto_definition".to_string(),
                metadata: json!({}),
            });
        }

        let rg = tokio::process::Command::new("rg")
            .args([
                "--line-number",
                "--no-heading",
                "-e",
                &format!(
                    r"(fn|struct|enum|trait|type|const|static|class|interface|def)\s+{word}\b"
                ),
            ])
            .arg(
                std::path::Path::new(file)
                    .parent()
                    .unwrap_or(std::path::Path::new(".")),
            )
            .output()
            .await;

        let output = match rg {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.is_empty() {
                    format!("Definition of '{word}' not found via structural search")
                } else {
                    let lines: Vec<&str> = stdout.lines().take(10).collect();
                    format!("Definition(s) of '{word}':\n{}", lines.join("\n"))
                }
            }
            Err(_) => "ripgrep not available".to_string(),
        };

        Ok(ToolResult {
            output,
            title: format!("goto_definition({word})"),
            metadata: json!({"symbol": word}),
        })
    }
}

pub struct LspFindReferencesTool;

#[async_trait]
impl Tool for LspFindReferencesTool {
    fn name(&self) -> &str {
        "lsp_find_references"
    }
    fn description(&self) -> &str {
        "Find all references to a symbol at a given file location."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string" },
                "line": { "type": "integer" },
                "column": { "type": "integer" }
            },
            "required": ["file", "line", "column"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: file"))?;
        let line = args.get("line").and_then(|v| v.as_u64()).unwrap_or(1);
        let col = args.get("column").and_then(|v| v.as_u64()).unwrap_or(1);

        let content = tokio::fs::read_to_string(file).await?;
        let target_line = content.lines().nth((line - 1) as usize).unwrap_or("");
        let word = extract_word_at(target_line, (col - 1) as usize);

        if word.is_empty() {
            return Ok(ToolResult {
                output: format!("No symbol found at {file}:{line}:{col}"),
                title: "lsp_find_references".to_string(),
                metadata: json!({}),
            });
        }

        let rg = tokio::process::Command::new("rg")
            .args(["--line-number", "--no-heading", "-w", &word])
            .arg(
                std::path::Path::new(file)
                    .parent()
                    .unwrap_or(std::path::Path::new(".")),
            )
            .output()
            .await;

        let output = match rg {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let count = stdout.lines().count();
                let lines: Vec<&str> = stdout.lines().take(30).collect();
                let truncated = if count > 30 {
                    format!("\n... ({count} total references)")
                } else {
                    String::new()
                };
                if lines.is_empty() {
                    format!("No references to '{word}' found")
                } else {
                    format!(
                        "References to '{word}' ({count} found):\n{}{truncated}",
                        lines.join("\n")
                    )
                }
            }
            Err(_) => "ripgrep not available".to_string(),
        };

        Ok(ToolResult {
            output,
            title: format!("find_references({word})"),
            metadata: json!({"symbol": word}),
        })
    }
}

pub struct LspHoverTool;

#[async_trait]
impl Tool for LspHoverTool {
    fn name(&self) -> &str {
        "lsp_hover"
    }
    fn description(&self) -> &str {
        "Get type information and documentation for a symbol at a given file location."
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string" },
                "line": { "type": "integer" },
                "column": { "type": "integer" }
            },
            "required": ["file", "line", "column"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing: file"))?;
        let line = args.get("line").and_then(|v| v.as_u64()).unwrap_or(1);
        let col = args.get("column").and_then(|v| v.as_u64()).unwrap_or(1);

        let content = tokio::fs::read_to_string(file).await?;
        let target_line = content.lines().nth((line - 1) as usize).unwrap_or("");
        let word = extract_word_at(target_line, (col - 1) as usize);

        if word.is_empty() {
            return Ok(ToolResult {
                output: format!("No symbol found at {file}:{line}:{col}"),
                title: "lsp_hover".to_string(),
                metadata: json!({}),
            });
        }

        let rg = tokio::process::Command::new("rg")
            .args([
                "--line-number",
                "--no-heading",
                "-B",
                "3",
                "-e",
                &format!(
                    r"(pub\s+)?(fn|struct|enum|trait|type|const|class|interface|def)\s+{word}\b"
                ),
            ])
            .arg(
                std::path::Path::new(file)
                    .parent()
                    .unwrap_or(std::path::Path::new(".")),
            )
            .output()
            .await;

        let output = match rg {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.is_empty() {
                    format!("No type information found for '{word}'")
                } else {
                    let lines: Vec<&str> = stdout.lines().take(20).collect();
                    format!("Type info for '{word}':\n{}", lines.join("\n"))
                }
            }
            Err(_) => "ripgrep not available".to_string(),
        };

        Ok(ToolResult {
            output,
            title: format!("hover({word})"),
            metadata: json!({"symbol": word}),
        })
    }
}

fn extract_word_at(line: &str, col: usize) -> String {
    let bytes = line.as_bytes();
    let col = col.min(bytes.len().saturating_sub(1));
    if col >= bytes.len() || !bytes[col].is_ascii_alphanumeric() && bytes[col] != b'_' {
        return String::new();
    }
    let mut start = col;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    line[start..end].to_string()
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
