use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspLocation {
    pub uri: String,
    pub range: LspRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    pub range: LspRange,
    pub severity: Option<u32>,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspSymbol {
    pub name: String,
    pub kind: u32,
    pub range: LspRange,
}

pub struct LspClient {
    child: Child,
    next_id: i64,
}

fn detect_server_command(file_ext: &str) -> Option<(&'static str, Vec<&'static str>)> {
    match file_ext {
        "rs" => Some(("rust-analyzer", vec![])),
        "ts" | "tsx" | "js" | "jsx" => Some(("typescript-language-server", vec!["--stdio"])),
        "py" => Some(("pylsp", vec![])),
        "go" => Some(("gopls", vec!["serve"])),
        _ => None,
    }
}

impl LspClient {
    pub async fn start(server_cmd: &str, args: &[&str], cwd: &Path) -> Result<Self> {
        let child = Command::new(server_cmd)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context(format!("Failed to start LSP server: {server_cmd}"))?;

        Ok(Self { child, next_id: 1 })
    }

    pub async fn start_for_file(file_path: &Path, cwd: &Path) -> Result<Self> {
        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let (cmd, args) = detect_server_command(ext)
            .ok_or_else(|| anyhow::anyhow!("No LSP server known for .{ext} files"))?;

        Self::start(cmd, &args, cwd).await
    }

    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let content = serde_json::to_string(&msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        let stdin = self.child.stdin.as_mut().context("No stdin")?;
        stdin.write_all(header.as_bytes()).await?;
        stdin.write_all(content.as_bytes()).await?;
        stdin.flush().await?;

        let stdout = self.child.stdout.as_mut().context("No stdout")?;
        let mut reader = BufReader::new(stdout);

        let mut header_line = String::new();
        reader.read_line(&mut header_line).await?;
        let content_length: usize = header_line
            .trim()
            .strip_prefix("Content-Length: ")
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);

        let mut empty_line = String::new();
        reader.read_line(&mut empty_line).await?;

        let mut body = vec![0u8; content_length];
        tokio::io::AsyncReadExt::read_exact(&mut reader, &mut body).await?;

        let response: Value = serde_json::from_slice(&body)?;
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    pub async fn initialize(&mut self, root_uri: &str) -> Result<Value> {
        self.send_request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {},
            }),
        )
        .await
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.send_request("shutdown", Value::Null).await;
        Ok(())
    }
}

pub fn symbol_kind_name(kind: u32) -> &'static str {
    match kind {
        1 => "File",
        2 => "Module",
        3 => "Namespace",
        5 => "Class",
        6 => "Method",
        8 => "Constructor",
        9 => "Enum",
        10 => "Interface",
        12 => "Function",
        13 => "Variable",
        14 => "Constant",
        15 => "String",
        23 => "Struct",
        _ => "Other",
    }
}

pub fn detect_available_servers() -> HashMap<String, String> {
    let servers = [
        ("rust", "rust-analyzer"),
        ("typescript", "typescript-language-server"),
        ("python", "pylsp"),
        ("go", "gopls"),
    ];

    let mut available = HashMap::new();
    for (lang, cmd) in &servers {
        if std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            available.insert(lang.to_string(), cmd.to_string());
        }
    }
    available
}
