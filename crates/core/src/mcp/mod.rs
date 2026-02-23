pub mod tool_adapter;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use nyzhi_config::McpServerConfig;
use rmcp::model::{CallToolRequestParams, Tool as McpToolDef};
use rmcp::service::{RoleClient, RunningService, ServiceExt};
use rmcp::transport::ConfigureCommandExt;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::TokioChildProcess;
use tokio::process::Command;
use tokio::sync::RwLock;

struct McpConnection {
    name: String,
    service: RunningService<RoleClient, ()>,
    tools: Vec<McpToolDef>,
}

pub struct McpManager {
    connections: RwLock<Vec<McpConnection>>,
}

/// Summary of a connected MCP server, safe to share.
#[derive(Debug, Clone)]
pub struct McpServerInfo {
    pub name: String,
    pub tool_count: usize,
    pub tool_names: Vec<String>,
}

impl McpManager {
    pub async fn start_all(configs: &HashMap<String, McpServerConfig>) -> Result<Arc<Self>> {
        let mut connections = Vec::new();

        for (name, cfg) in configs {
            match Self::connect(name, cfg).await {
                Ok(conn) => {
                    tracing::info!(
                        server = %name,
                        tools = conn.tools.len(),
                        "MCP server connected"
                    );
                    connections.push(conn);
                }
                Err(e) => {
                    tracing::warn!(server = %name, error = %e, "Failed to connect MCP server");
                }
            }
        }

        Ok(Arc::new(Self {
            connections: RwLock::new(connections),
        }))
    }

    async fn connect(name: &str, config: &McpServerConfig) -> Result<McpConnection> {
        match config {
            McpServerConfig::Stdio { command, args, env } => {
                let env_clone = env.clone();
                let args_clone = args.clone();
                let service = ()
                    .serve(TokioChildProcess::new(Command::new(command).configure(
                        move |cmd| {
                            for arg in &args_clone {
                                cmd.arg(arg);
                            }
                            for (k, v) in &env_clone {
                                cmd.env(k, v);
                            }
                        },
                    ))?)
                    .await
                    .with_context(|| format!("MCP stdio init failed for '{name}'"))?;

                let tools_result = service
                    .list_tools(Default::default())
                    .await
                    .with_context(|| format!("MCP tools/list failed for '{name}'"))?;

                Ok(McpConnection {
                    name: name.to_string(),
                    service,
                    tools: tools_result.tools,
                })
            }
            McpServerConfig::Http {
                url,
                headers: _headers,
            } => {
                let transport = StreamableHttpClientTransport::from_uri(url.as_str());

                let service: RunningService<RoleClient, ()> = ()
                    .serve(transport)
                    .await
                    .with_context(|| format!("MCP HTTP init failed for '{name}'"))?;

                let tools_result = service
                    .list_tools(Default::default())
                    .await
                    .with_context(|| format!("MCP tools/list failed for '{name}'"))?;

                Ok(McpConnection {
                    name: name.to_string(),
                    service,
                    tools: tools_result.tools,
                })
            }
        }
    }

    pub async fn stop_all(&self) {
        let mut conns = self.connections.write().await;
        for conn in conns.drain(..) {
            if let Err(e) = conn.service.cancel().await {
                tracing::warn!(server = %conn.name, error = %e, "Error stopping MCP server");
            }
        }
    }

    /// Returns (server_name, mcp_tool_def) pairs for all connected servers.
    pub async fn all_tools(&self) -> Vec<(String, McpToolDef)> {
        let conns = self.connections.read().await;
        let mut result = Vec::new();
        for conn in conns.iter() {
            for tool in &conn.tools {
                result.push((conn.name.clone(), tool.clone()));
            }
        }
        result
    }

    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<String> {
        let conns = self.connections.read().await;
        let conn = conns
            .iter()
            .find(|c| c.name == server_name)
            .ok_or_else(|| anyhow::anyhow!("MCP server '{server_name}' not found"))?;

        let result = conn
            .service
            .call_tool(CallToolRequestParams {
                name: tool_name.to_string().into(),
                arguments,
                meta: None,
                task: None,
            })
            .await
            .with_context(|| {
                format!("MCP tools/call failed for '{tool_name}' on '{server_name}'")
            })?;

        let mut output = String::new();
        for content in &result.content {
            if let Some(text) = content.as_text() {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(&text.text);
            }
        }

        if result.is_error == Some(true) && output.is_empty() {
            output = "MCP tool returned an error with no content".to_string();
        }

        Ok(output)
    }

    /// Hot-add a single MCP server at runtime. Returns the number of tools discovered.
    pub async fn connect_server(&self, name: &str, config: &McpServerConfig) -> Result<usize> {
        let conn = Self::connect(name, config).await?;
        let tool_count = conn.tools.len();
        tracing::info!(
            server = %name,
            tools = tool_count,
            "MCP server hot-connected"
        );
        self.connections.write().await.push(conn);
        Ok(tool_count)
    }

    /// Returns tool summaries suitable for prompt injection.
    pub fn tool_summaries(&self) -> Vec<crate::prompt::McpToolSummary> {
        let conns = self.connections.try_read();
        match conns {
            Ok(conns) => conns
                .iter()
                .flat_map(|c| {
                    c.tools.iter().map(|t| crate::prompt::McpToolSummary {
                        server_name: c.name.clone(),
                        tool_name: t.name.to_string(),
                        description: t.description.as_deref().unwrap_or("").to_string(),
                    })
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub async fn server_info_list(&self) -> Vec<McpServerInfo> {
        let conns = self.connections.read().await;
        conns
            .iter()
            .map(|c| McpServerInfo {
                name: c.name.clone(),
                tool_count: c.tools.len(),
                tool_names: c.tools.iter().map(|t| t.name.to_string()).collect(),
            })
            .collect()
    }
}

/// Load `.mcp.json` from a directory (Claude Code / Codex compatibility format).
pub fn load_mcp_json(root: &Path) -> HashMap<String, McpServerConfig> {
    let path = root.join(".mcp.json");
    if !path.exists() {
        return HashMap::new();
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "Failed to read .mcp.json");
            return HashMap::new();
        }
    };

    #[derive(serde::Deserialize)]
    struct McpJson {
        #[serde(default, alias = "mcpServers")]
        mcp_servers: HashMap<String, McpJsonServer>,
    }

    #[derive(serde::Deserialize)]
    struct McpJsonServer {
        command: Option<String>,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        url: Option<String>,
        #[serde(default)]
        headers: HashMap<String, String>,
    }

    let parsed: McpJson = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "Failed to parse .mcp.json");
            return HashMap::new();
        }
    };

    parsed
        .mcp_servers
        .into_iter()
        .filter_map(|(name, server)| {
            if let Some(url) = server.url {
                Some((
                    name,
                    McpServerConfig::Http {
                        url,
                        headers: server.headers,
                    },
                ))
            } else if let Some(command) = server.command {
                Some((
                    name,
                    McpServerConfig::Stdio {
                        command,
                        args: server.args,
                        env: server.env,
                    },
                ))
            } else {
                tracing::warn!(server = %name, "MCP server in .mcp.json has neither command nor url");
                None
            }
        })
        .collect()
}
