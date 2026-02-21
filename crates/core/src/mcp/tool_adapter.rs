use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::tools::permission::ToolPermission;
use crate::tools::{Tool, ToolContext, ToolResult};
use super::McpManager;

/// Bridges an MCP server tool into the local `ToolRegistry`.
pub struct McpTool {
    server_name: String,
    tool_name: String,
    full_name: String,
    description: String,
    schema: Value,
    manager: Arc<McpManager>,
}

impl McpTool {
    pub fn new(
        server_name: &str,
        tool_name: &str,
        description: &str,
        schema: Value,
        manager: Arc<McpManager>,
    ) -> Self {
        Self {
            server_name: server_name.to_string(),
            tool_name: tool_name.to_string(),
            full_name: format!("mcp__{server_name}__{tool_name}"),
            description: description.to_string(),
            schema,
            manager,
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.schema.clone()
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission::NeedsApproval
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult> {
        let arguments = args.as_object().cloned();

        let output = self
            .manager
            .call_tool(&self.server_name, &self.tool_name, arguments)
            .await?;

        Ok(ToolResult {
            output,
            title: format!("mcp:{}/{}", self.server_name, self.tool_name),
            metadata: serde_json::json!({
                "mcp_server": self.server_name,
                "mcp_tool": self.tool_name,
            }),
        })
    }
}
