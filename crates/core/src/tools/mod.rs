pub mod permission;
pub mod bash;
pub mod change_tracker;
pub mod close_agent;
pub mod diff;
pub mod read;
pub mod write;
pub mod edit;
pub mod glob;
pub mod grep;
pub mod git;
pub mod task;
pub mod todo;
pub mod filesystem;
pub mod lsp;
pub mod notepad;
pub mod resume_agent;
pub mod send_input;
pub mod spawn_agent;
pub mod verify;
pub mod wait_tool;
pub mod web;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use permission::ToolPermission;
use serde_json::Value;
use tokio::sync::broadcast;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn permission(&self) -> ToolPermission {
        ToolPermission::ReadOnly
    }
    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult>;
}

#[derive(Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub cwd: PathBuf,
    pub project_root: PathBuf,
    /// 0 = main agent, 1 = first sub-agent, etc.
    pub depth: u32,
    pub event_tx: Option<broadcast::Sender<crate::agent::AgentEvent>>,
    pub change_tracker: Arc<tokio::sync::Mutex<change_tracker::ChangeTracker>>,
    /// If set, only these tools are visible to the agent (role-based filtering).
    pub allowed_tool_names: Option<Vec<String>>,
}

pub struct ToolResult {
    pub output: String,
    pub title: String,
    pub metadata: Value,
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<nyzhi_provider::ToolDefinition> {
        let mut defs: Vec<_> = self
            .tools
            .values()
            .map(|t| nyzhi_provider::ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    pub async fn execute(&self, name: &str, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        if let Some(allowed) = &ctx.allowed_tool_names {
            if !allowed.iter().any(|a| a == name) {
                anyhow::bail!("Tool `{name}` is not available for this agent role");
            }
        }
        let tool = self
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {name}"))?;
        tool.execute(args, ctx).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a new registry containing only tools allowed by the given filters.
    /// `allowed`: if Some, only tools whose name is in the set are kept.
    /// `disallowed`: if Some, tools whose name is in the set are removed.
    /// Allowed is applied first (whitelist), then disallowed (blacklist).
    pub fn filtered(
        &self,
        allowed: Option<&[String]>,
        disallowed: Option<&[String]>,
    ) -> Vec<String> {
        let mut names: Vec<String> = self.tools.keys().cloned().collect();

        if let Some(allow_list) = allowed {
            let allow_set: std::collections::HashSet<&str> =
                allow_list.iter().map(|s| s.as_str()).collect();
            names.retain(|n| allow_set.contains(n.as_str()));
        }

        if let Some(deny_list) = disallowed {
            let deny_set: std::collections::HashSet<&str> =
                deny_list.iter().map(|s| s.as_str()).collect();
            names.retain(|n| !deny_set.contains(n.as_str()));
        }

        names
    }

    pub fn names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Return tool definitions filtered to only the given tool names.
    pub fn definitions_filtered(&self, allowed_names: &[String]) -> Vec<nyzhi_provider::ToolDefinition> {
        let allow_set: std::collections::HashSet<&str> =
            allowed_names.iter().map(|s| s.as_str()).collect();
        let mut defs: Vec<_> = self
            .tools
            .values()
            .filter(|t| allow_set.contains(t.name()))
            .map(|t| nyzhi_provider::ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }
}

pub type TodoStoreHandle = Arc<tokio::sync::Mutex<std::collections::HashMap<String, Vec<todo::TodoItem>>>>;

pub struct RegistryBundle {
    pub registry: ToolRegistry,
    pub todo_store: TodoStoreHandle,
}

pub fn default_registry() -> RegistryBundle {
    let todo_store = todo::shared_store();
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(bash::BashTool));
    registry.register(Box::new(read::ReadTool));
    registry.register(Box::new(write::WriteTool));
    registry.register(Box::new(edit::EditTool));
    registry.register(Box::new(glob::GlobTool));
    registry.register(Box::new(grep::GrepTool));
    registry.register(Box::new(git::GitStatusTool));
    registry.register(Box::new(git::GitDiffTool));
    registry.register(Box::new(git::GitLogTool));
    registry.register(Box::new(git::GitShowTool));
    registry.register(Box::new(git::GitBranchTool));
    registry.register(Box::new(git::GitCommitTool));
    registry.register(Box::new(git::GitCheckoutTool));
    registry.register(Box::new(todo::TodoWriteTool::with_store(todo_store.clone())));
    registry.register(Box::new(todo::TodoReadTool::with_store(todo_store.clone())));
    registry.register(Box::new(filesystem::ListDirTool));
    registry.register(Box::new(filesystem::DirectoryTreeTool));
    registry.register(Box::new(filesystem::FileInfoTool));
    registry.register(Box::new(filesystem::DeleteFileTool));
    registry.register(Box::new(filesystem::MoveFileTool));
    registry.register(Box::new(filesystem::CopyFileTool));
    registry.register(Box::new(filesystem::CreateDirTool));
    registry.register(Box::new(verify::VerifyTool));
    registry.register(Box::new(notepad::NotepadWriteTool));
    registry.register(Box::new(notepad::NotepadReadTool));
    registry.register(Box::new(lsp::LspDiagnosticsTool));
    registry.register(Box::new(lsp::AstSearchTool));
    registry.register(Box::new(web::WebFetchTool));
    registry.register(Box::new(web::WebSearchTool));
    RegistryBundle { registry, todo_store }
}
