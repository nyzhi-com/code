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

pub struct ToolContext {
    pub session_id: String,
    pub cwd: PathBuf,
    pub project_root: PathBuf,
    /// 0 = main agent, 1 = first sub-agent, etc.
    pub depth: u32,
    pub event_tx: Option<broadcast::Sender<crate::agent::AgentEvent>>,
    pub change_tracker: Arc<tokio::sync::Mutex<change_tracker::ChangeTracker>>,
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
    RegistryBundle { registry, todo_store }
}
