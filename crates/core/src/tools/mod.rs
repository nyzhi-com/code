pub mod apply_patch;
pub mod ask_user;
pub mod bash;
pub mod batch;
pub mod browser;
pub mod change_tracker;
pub mod close_agent;
pub mod diff;
pub mod edit;
pub mod filesystem;
pub mod fuzzy_find;
pub mod git;
pub mod glob;
pub mod grep;
pub mod instrument;
pub mod load_skill;
pub mod lsp;
pub mod memory;
pub mod notepad;
pub mod permission;
pub mod pr;
pub mod read;
pub mod resume_agent;
pub mod semantic_search;
pub mod send_input;
pub mod spawn_agent;
pub mod tail_file;
pub mod task;
pub mod team;
pub mod think;
pub mod todo;
pub mod tool_search;
pub mod update_plan;
pub mod verify;
pub mod wait_tool;
pub mod web;
pub mod write;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use permission::ToolPermission;
use serde_json::Value;
use tokio::sync::broadcast;

pub type IndexHandle = Arc<nyzhi_index::CodebaseIndex>;

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
    /// Team name if this agent is part of an agent team.
    pub team_name: Option<String>,
    /// Agent's own name within the team (used for inbox addressing).
    pub agent_name: Option<String>,
    /// Whether this agent is the team lead (coordinator).
    pub is_team_lead: bool,
    /// Shared todo store for rehydration during compaction.
    pub todo_store: Option<TodoStoreHandle>,
    /// Codebase index for semantic search and auto-context.
    pub index: Option<IndexHandle>,
}

pub struct ToolResult {
    pub output: String,
    pub title: String,
    pub metadata: Value,
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    /// Tools that are indexed but not sent as full definitions in ChatRequest.
    /// The agent can discover them via tool_search and they expand on first use.
    deferred: std::collections::HashSet<String>,
    /// Session-level cache of deferred tools that have been expanded (used at least once).
    expanded: std::collections::HashSet<String>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            deferred: std::collections::HashSet::new(),
            expanded: std::collections::HashSet::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Register a tool as deferred (index-only, not sent in ChatRequest until used).
    pub fn register_deferred(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name.clone(), tool);
        self.deferred.insert(name);
    }

    /// Mark a deferred tool as expanded (it will now be included in definitions).
    pub fn expand_deferred(&mut self, name: &str) {
        if self.deferred.contains(name) {
            self.expanded.insert(name.to_string());
        }
    }

    /// Check if a tool is deferred and not yet expanded.
    pub fn is_deferred(&self, name: &str) -> bool {
        self.deferred.contains(name) && !self.expanded.contains(name)
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// Return definitions for all non-deferred tools plus any expanded deferred tools.
    pub fn definitions(&self) -> Vec<nyzhi_provider::ToolDefinition> {
        let mut defs: Vec<_> = self
            .tools
            .values()
            .filter(|t| !self.deferred.contains(t.name()) || self.expanded.contains(t.name()))
            .map(|t| nyzhi_provider::ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    /// Return definitions for read-only tools only (plan mode).
    pub fn definitions_read_only(&self) -> Vec<nyzhi_provider::ToolDefinition> {
        let mut defs: Vec<_> = self
            .tools
            .values()
            .filter(|t| t.permission() == permission::ToolPermission::ReadOnly)
            .filter(|t| !self.deferred.contains(t.name()) || self.expanded.contains(t.name()))
            .map(|t| nyzhi_provider::ToolDefinition {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect();
        defs.sort_by(|a, b| a.name.cmp(&b.name));
        defs
    }

    pub fn deferred_count(&self) -> usize {
        self.deferred.len().saturating_sub(self.expanded.len())
    }

    /// Build the deferred tool index for tool_search.
    pub fn deferred_index(&self) -> Vec<tool_search::DeferredToolEntry> {
        self.deferred
            .iter()
            .filter(|name| !self.expanded.contains(name.as_str()))
            .filter_map(|name| {
                self.tools
                    .get(name)
                    .map(|t| tool_search::DeferredToolEntry {
                        name: t.name().to_string(),
                        description: t.description().to_string(),
                    })
            })
            .collect()
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
    pub fn definitions_filtered(
        &self,
        allowed_names: &[String],
    ) -> Vec<nyzhi_provider::ToolDefinition> {
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

pub type TodoStoreHandle =
    Arc<tokio::sync::Mutex<std::collections::HashMap<String, Vec<todo::TodoItem>>>>;

pub async fn todo_has_incomplete(store: &TodoStoreHandle, session_id: &str) -> bool {
    todo::has_incomplete_todos(store, session_id).await
}

pub async fn todo_incomplete_summary(store: &TodoStoreHandle, session_id: &str) -> Option<String> {
    todo::incomplete_summary(store, session_id).await
}

pub async fn todo_progress(
    store: &TodoStoreHandle,
    session_id: &str,
) -> Option<(usize, usize, usize)> {
    todo::progress_summary(store, session_id).await
}

pub struct RegistryBundle {
    pub registry: ToolRegistry,
    pub todo_store: TodoStoreHandle,
    pub deferred_index: tool_search::DeferredToolIndex,
}

pub fn default_registry(codebase_index: Option<IndexHandle>) -> RegistryBundle {
    let todo_store = todo::shared_store();
    let deferred_index = tool_search::shared_deferred_index();
    let instrument_store = instrument::shared_store();
    let mut registry = ToolRegistry::new();

    // Core tools
    registry.register(Box::new(bash::BashTool));
    registry.register(Box::new(read::ReadTool));
    registry.register(Box::new(write::WriteTool));
    registry.register(Box::new(edit::EditTool));
    registry.register(Box::new(glob::GlobTool));
    registry.register(Box::new(grep::GrepTool));

    // Git tools
    registry.register(Box::new(git::GitStatusTool));
    registry.register(Box::new(git::GitDiffTool));
    registry.register(Box::new(git::GitLogTool));
    registry.register(Box::new(git::GitShowTool));
    registry.register(Box::new(git::GitBranchTool));
    registry.register(Box::new(git::GitCommitTool));
    registry.register(Box::new(git::GitCheckoutTool));

    // Task management
    registry.register(Box::new(todo::TodoWriteTool::with_store(
        todo_store.clone(),
    )));
    registry.register(Box::new(todo::TodoReadTool::with_store(todo_store.clone())));

    // Filesystem
    registry.register(Box::new(filesystem::ListDirTool));
    registry.register(Box::new(filesystem::DirectoryTreeTool));
    registry.register(Box::new(filesystem::FileInfoTool));
    registry.register(Box::new(filesystem::DeleteFileTool));
    registry.register(Box::new(filesystem::MoveFileTool));
    registry.register(Box::new(filesystem::CopyFileTool));
    registry.register(Box::new(filesystem::CreateDirTool));

    // Code analysis
    registry.register(Box::new(verify::VerifyTool));
    registry.register(Box::new(notepad::NotepadWriteTool));
    registry.register(Box::new(notepad::NotepadReadTool));
    registry.register(Box::new(lsp::LspDiagnosticsTool));
    registry.register(Box::new(lsp::AstSearchTool));
    registry.register(Box::new(lsp::LspGotoDefinitionTool));
    registry.register(Box::new(lsp::LspFindReferencesTool));
    registry.register(Box::new(lsp::LspHoverTool));

    // Web
    registry.register(Box::new(web::WebFetchTool));
    registry.register(Box::new(web::WebSearchTool));

    // Misc
    registry.register(Box::new(tail_file::TailFileTool));
    registry.register(Box::new(load_skill::LoadSkillTool));
    registry.register(Box::new(tool_search::ToolSearchTool::new(
        deferred_index.clone(),
    )));
    registry.register(Box::new(memory::MemoryReadTool));
    registry.register(Box::new(memory::MemoryWriteTool));

    // Teams
    registry.register(Box::new(team::TeamCreateTool));
    registry.register(Box::new(team::TeamDeleteTool));
    registry.register(Box::new(team::SendMessageTool));
    registry.register(Box::new(team::TaskCreateTool));
    registry.register(Box::new(team::TaskUpdateTool));
    registry.register(Box::new(team::TaskListTool));
    registry.register(Box::new(team::TeamListTool));
    registry.register(Box::new(team::ReadInboxTool));
    registry.register(Box::new(batch::BatchApplyTool));

    // Phase 1.1: Semantic search & fuzzy find
    if let Some(idx) = codebase_index {
        registry.register(Box::new(semantic_search::SemanticSearchTool::new(idx)));
    }
    registry.register(Box::new(fuzzy_find::FuzzyFindTool));

    // Phase 1.2: Plan-execute mode
    registry.register(Box::new(update_plan::UpdatePlanTool));

    // Phase 1.3: Think tool
    registry.register(Box::new(think::ThinkTool));

    // Interactive user question
    registry.register(Box::new(ask_user::AskUserTool));

    // Phase 2.3: Structured patch application
    registry.register(Box::new(apply_patch::ApplyPatchTool));
    registry.register(Box::new(apply_patch::MultiEditTool));

    // Phase 3.3: Debug instrumentation
    registry.register(Box::new(instrument::InstrumentTool::new(
        instrument_store.clone(),
    )));
    registry.register(Box::new(instrument::RemoveInstrumentationTool::new(
        instrument_store,
    )));

    // Phase 4.2: Browser automation
    registry.register(Box::new(browser::BrowserOpenTool));
    registry.register(Box::new(browser::BrowserScreenshotTool));
    registry.register(Box::new(browser::BrowserEvaluateTool));

    // Phase 4.3: PR workflow
    registry.register(Box::new(pr::CreatePrTool));
    registry.register(Box::new(pr::ReviewPrTool));

    // NOTE: SpawnTeammateTool requires Arc<AgentManager> and is registered
    // separately in the TUI/CLI after the manager is created.
    RegistryBundle {
        registry,
        todo_store,
        deferred_index,
    }
}
