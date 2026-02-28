use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyModifiers,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use nyzhi_core::agent::{AgentConfig, AgentEvent, SessionUsage};
use nyzhi_core::conversation::Thread;
use nyzhi_core::tools::{ToolContext, ToolRegistry};
use nyzhi_core::workspace::WorkspaceContext;
use nyzhi_provider::{MessageContent, Provider};
use ratatui::prelude::*;
use tokio::sync::broadcast;

use crate::components::selector::SelectorKind;
use crate::input::handle_key;
use crate::spinner::SpinnerState;
use crate::theme::Theme;
use crate::ui::draw;

#[derive(PartialEq)]
pub enum AppMode {
    Input,
    Streaming,
    AwaitingApproval,
    AwaitingUserQuestion,
}

#[derive(Debug, Clone)]
pub enum DisplayItem {
    Message {
        role: String,
        content: String,
    },
    Thinking(String),
    ToolCall {
        name: String,
        args_summary: String,
        output: Option<String>,
        status: ToolStatus,
        elapsed_ms: Option<u64>,
    },
    Diff {
        file: String,
        hunks: Vec<DiffHunk>,
        is_new_file: bool,
    },
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Running,
    WaitingApproval,
    Completed,
    Denied,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Checking,
    Available {
        new_version: String,
        current_version: String,
        changelog: Option<String>,
    },
    Downloading {
        progress: Option<u8>,
    },
    None,
}

pub struct PendingImage {
    pub filename: String,
    pub media_type: String,
    pub data: String,
    pub size_bytes: usize,
}

pub struct TurnRequest {
    pub input: String,
    pub content: Option<MessageContent>,
    pub is_background: bool,
    pub label: String,
}

pub struct TurnResult {
    pub thread: Thread,
    pub session_usage: SessionUsage,
    pub result: anyhow::Result<()>,
}

pub struct ForegroundTask {
    pub join_handle: tokio::task::JoinHandle<TurnResult>,
    pub thread_snapshot: Thread,
    pub label: String,
}

pub struct BackgroundTask {
    pub id: usize,
    pub label: String,
    pub join_handle: tokio::task::JoinHandle<TurnResult>,
    pub started: std::time::Instant,
}

pub struct App {
    pub mode: AppMode,
    pub input: String,
    pub cursor_pos: usize,
    pub items: Vec<DisplayItem>,
    pub current_stream: String,
    pub thinking_stream: String,
    pub should_quit: bool,
    pub provider_name: String,
    pub model_name: String,
    pub model_profile: Option<String>,
    pub scroll_offset: u16,
    pub theme: Theme,
    pub spinner: SpinnerState,
    pub session_usage: SessionUsage,
    pub session_start: std::time::Instant,
    pub workspace: WorkspaceContext,
    pub mcp_manager: Option<std::sync::Arc<nyzhi_core::mcp::McpManager>>,
    pub pending_approval:
        Option<std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>>,
    pub pending_approval_context: Option<(String, String)>,
    pub approval_cursor: usize,
    pub session_approved_tools: std::collections::HashSet<String>,
    pub pending_images: Vec<PendingImage>,
    pub trust_mode: nyzhi_config::TrustMode,
    pub selector: Option<crate::components::selector::SelectorState>,
    pub text_prompt: Option<crate::components::text_prompt::TextPromptState>,
    pub wants_editor: bool,
    pub history: crate::history::InputHistory,
    pub history_search: Option<crate::history::HistorySearch>,
    pub highlighter: crate::highlight::SyntaxHighlighter,
    pub completion: Option<crate::completion::CompletionState>,
    pub stream_start: Option<std::time::Instant>,
    pub stream_token_count: usize,
    pub turn_start: Option<std::time::Instant>,
    pub last_prompt: Option<String>,
    pub initial_session: Option<(Thread, nyzhi_core::session::SessionMeta)>,
    pub hooks_config: Vec<nyzhi_config::HookConfig>,
    pub hook_rx: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    hook_tx: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    pub custom_commands: Vec<nyzhi_core::commands::CustomCommand>,
    pub search_query: Option<String>,
    pub search_matches: Vec<usize>,
    pub search_match_idx: usize,
    pub notify: nyzhi_config::NotifyConfig,
    pub output_style: nyzhi_config::OutputStyle,
    pub show_thinking: bool,
    pub turn_request: Option<TurnRequest>,
    pub foreground_task: Option<ForegroundTask>,
    pub background_tasks: Vec<BackgroundTask>,
    pub background_next_id: usize,
    pub ctrl_f_pending: bool,
    pub context_used_tokens: usize,
    pub context_window: u32,
    pub update_status: UpdateStatus,
    update_info: Option<nyzhi_core::updater::UpdateInfo>,
    update_done_rx:
        Option<tokio::sync::mpsc::Receiver<anyhow::Result<nyzhi_core::updater::UpdateResult>>>,
    pub thinking_level: Option<String>,
    pub pending_command_dispatch: bool,
    pub pending_oauth: Option<(String, String)>,
    oauth_rx: Option<
        tokio::sync::oneshot::Receiver<(String, Result<nyzhi_auth::token_store::StoredToken>)>,
    >,
    oauth_msg_rx: Option<tokio::sync::mpsc::UnboundedReceiver<String>>,
    pub pending_provider_reload: Option<String>,
    pub pending_user_question:
        Option<std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<String>>>>>,
    pub ephemeral: bool,
    pub plan_mode: bool,
    pub last_plan_name: Option<String>,
    pub show_plan_panel: bool,
    pub plan_panel: crate::components::plan_panel::PlanPanelState,
    pub current_session_id: Option<String>,
    pub todo_store: Option<nyzhi_core::tools::TodoStoreHandle>,
    pub autopilot: Option<nyzhi_core::autopilot::AutopilotState>,
    pub todo_enforcement_paused: bool,
    pub todo_enforce_count: u32,
    pub todo_progress: Option<(usize, usize, usize)>,
    pub todo_panel: Option<crate::components::todo_panel::TodoPanelState>,
    pub settings_panel: Option<crate::components::settings_panel::SettingsPanel>,
    pub message_queue: VecDeque<TurnRequest>,
    pub model_cache: nyzhi_provider::ModelCacheHandle,
    pub codebase_index: Option<nyzhi_core::tools::IndexHandle>,
    pub checkpoint_manager: Option<nyzhi_core::checkpoint::CheckpointManager>,
    pub librarian: nyzhi_core::librarian::Librarian,
    pub index_progress: Option<(usize, usize, bool)>,
    pub index_error: Option<String>,
    pub session_title: String,
    pub last_turn_duration: Option<f64>,
    pub subagent_model_overrides: nyzhi_core::agent_roles::SubagentModelOverrides,
    pub shared_context: std::sync::Arc<tokio::sync::Mutex<nyzhi_core::context_briefing::SharedContext>>,
    pub config: nyzhi_config::Config,
    pub logo_anim: crate::logo_anim::LogoAnimation,
}

impl App {
    pub fn new(
        provider_name: &str,
        model_name: &str,
        config: &nyzhi_config::TuiConfig,
        workspace: WorkspaceContext,
    ) -> Self {
        Self {
            mode: AppMode::Input,
            input: String::new(),
            cursor_pos: 0,
            items: Vec::new(),
            current_stream: String::new(),
            thinking_stream: String::new(),
            should_quit: false,
            provider_name: provider_name.to_string(),
            model_name: model_name.to_string(),
            model_profile: None,
            scroll_offset: 0,
            theme: Theme::from_config(config),
            spinner: SpinnerState::new(),
            session_usage: SessionUsage::default(),
            session_start: std::time::Instant::now(),
            workspace,
            mcp_manager: None,
            pending_approval: None,
            pending_approval_context: None,
            approval_cursor: 0,
            session_approved_tools: std::collections::HashSet::new(),
            pending_images: Vec::new(),
            trust_mode: nyzhi_config::TrustMode::Off,
            selector: None,
            text_prompt: None,
            wants_editor: false,
            history: crate::history::InputHistory::new(
                nyzhi_config::Config::data_dir().join("history"),
            ),
            history_search: None,
            highlighter: crate::highlight::SyntaxHighlighter::new(),
            completion: None,
            stream_start: None,
            stream_token_count: 0,
            turn_start: None,
            last_prompt: None,
            initial_session: None,
            hooks_config: Vec::new(),
            hook_rx: None,
            hook_tx: None,
            custom_commands: Vec::new(),
            search_query: None,
            search_matches: Vec::new(),
            search_match_idx: 0,
            notify: config.notify.clone(),
            output_style: config.output_style,
            show_thinking: config.show_thinking,
            turn_request: None,
            foreground_task: None,
            background_tasks: Vec::new(),
            background_next_id: 1,
            ctrl_f_pending: false,
            context_used_tokens: 0,
            context_window: 0,
            update_status: UpdateStatus::None,
            update_info: None,
            update_done_rx: None,
            thinking_level: None,
            pending_command_dispatch: false,
            pending_oauth: None,
            oauth_rx: None,
            oauth_msg_rx: None,
            pending_provider_reload: None,
            pending_user_question: None,
            ephemeral: false,
            plan_mode: false,
            last_plan_name: None,
            show_plan_panel: false,
            plan_panel: crate::components::plan_panel::PlanPanelState::default(),
            current_session_id: None,
            todo_store: None,
            autopilot: None,
            todo_enforcement_paused: false,
            todo_enforce_count: 0,
            todo_progress: None,
            todo_panel: None,
            settings_panel: None,
            message_queue: VecDeque::new(),
            model_cache: nyzhi_provider::ModelCache::handle(),
            codebase_index: None,
            checkpoint_manager: None,
            librarian: nyzhi_core::librarian::Librarian::new(
                &nyzhi_config::Config::data_dir(),
            ),
            index_progress: None,
            index_error: None,
            session_title: String::from("nyzhi code"),
            last_turn_duration: None,
            subagent_model_overrides: nyzhi_core::agent_roles::SubagentModelOverrides::new(),
            shared_context: std::sync::Arc::new(tokio::sync::Mutex::new(nyzhi_core::context_briefing::SharedContext::default())),
            config: nyzhi_config::Config::default(),
            logo_anim: crate::logo_anim::LogoAnimation::new(),
        }
    }

    pub fn run_search(&mut self, query: &str) {
        let q = query.to_lowercase();
        self.search_matches.clear();
        self.search_match_idx = 0;

        for (i, item) in self.items.iter().enumerate() {
            let text = match item {
                DisplayItem::Message { content, .. } => content.to_lowercase(),
                DisplayItem::Thinking(content) => content.to_lowercase(),
                DisplayItem::ToolCall {
                    args_summary,
                    output,
                    ..
                } => {
                    let mut t = args_summary.to_lowercase();
                    if let Some(o) = output {
                        t.push(' ');
                        t.push_str(&o.to_lowercase());
                    }
                    t
                }
                DisplayItem::Diff { file, .. } => file.to_lowercase(),
            };
            if text.contains(&q) {
                self.search_matches.push(i);
            }
        }

        self.search_query = Some(query.to_string());
    }

    pub fn search_next(&mut self) {
        if !self.search_matches.is_empty() {
            self.search_match_idx = (self.search_match_idx + 1) % self.search_matches.len();
        }
    }

    pub fn search_prev(&mut self) {
        if !self.search_matches.is_empty() {
            self.search_match_idx = if self.search_match_idx == 0 {
                self.search_matches.len() - 1
            } else {
                self.search_match_idx - 1
            };
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_matches.clear();
        self.search_match_idx = 0;
    }

    fn try_save_session(&self, thread: Option<&nyzhi_core::conversation::Thread>) {
        if self.ephemeral {
            return;
        }
        if let Some(t) = thread {
            if t.message_count() > 0 {
                let _ = nyzhi_core::session::save_session(
                    t,
                    &self.provider_name,
                    &self.model_name,
                );
            }
        }
    }

    pub async fn run(
        &mut self,
        mut provider: Option<std::sync::Arc<dyn Provider>>,
        mut registry: ToolRegistry,
        config: &nyzhi_config::Config,
    ) -> Result<()> {
        self.config = config.clone();

        // Post-update health check — detect if a recent update broke anything
        let health_warnings = nyzhi_core::updater::startup_health_check();
        for w in &health_warnings {
            self.items.push(DisplayItem::Message {
                role: "system".to_string(),
                content: format!("Post-update warning: {w}"),
            });
        }

        if self.checkpoint_manager.is_none()
            && nyzhi_core::git_undo::is_git_repo(&self.workspace.project_root)
        {
            self.checkpoint_manager = Some(nyzhi_core::checkpoint::CheckpointManager::new(
                &self.workspace.project_root,
                "default",
            ));
        }

        let nyzhi_dir = self.workspace.project_root.join(".nyzhi");
        if self.trust_mode == nyzhi_config::TrustMode::Off && !nyzhi_dir.exists() {
            self.items.push(DisplayItem::Message {
                role: "system".to_string(),
                content: "Welcome! Choose a permission mode to get started.".to_string(),
            });
            self.open_trust_selector();
        }

        self.history.load();
        self.custom_commands = nyzhi_core::commands::load_all_commands(
            &self.workspace.project_root,
            &config.agent.commands,
        );

        {
            let cache = self.model_cache.clone();
            tokio::spawn(async move {
                let registry = nyzhi_provider::ModelRegistry::new();
                for pid in registry.providers() {
                    let _ = nyzhi_provider::refresh_provider_models(pid, &cache).await;
                }
            });
        }

        if config.index.enabled && self.codebase_index.is_none() {
            let mut api_keys = std::collections::HashMap::new();
            for provider_id in &["openai", "voyage", "perplexity"] {
                if let Ok(cred) = nyzhi_auth::resolve_credential(provider_id, None) {
                    api_keys.insert(provider_id.to_string(), cred.header_value());
                }
            }
            let index_options = nyzhi_index::IndexOptions {
                embedding_mode: config.index.embedding.clone(),
                embedding_model: config.index.embedding_model.clone(),
                exclude: config.index.exclude.clone(),
                api_keys,
            };
            match nyzhi_index::CodebaseIndex::open_sync_with_options(
                &self.workspace.project_root,
                index_options,
            ) {
                Ok(index) => {
                    let handle = std::sync::Arc::new(index);
                    self.codebase_index = Some(handle.clone());
                    self.index_progress = Some((0, 0, false));
                    tokio::spawn(async move {
                        match handle.build().await {
                            Ok(stats) => {
                                tracing::debug!(
                                    "Index built: {} files, {} chunks, {} vectors",
                                    stats.file_count, stats.chunk_count, stats.vector_count
                                );
                            }
                            Err(e) => tracing::debug!("Index build failed: {e}"),
                        }
                    });
                }
                Err(e) => tracing::debug!("Failed to open index: {e}"),
            }
        }

        terminal::enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        io::stdout().execute(EnableBracketedPaste)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        let (event_tx, mut event_rx) = broadcast::channel::<AgentEvent>(256);
        let mut thread: Option<Thread> = Some(
            if let Some((loaded_thread, loaded_meta)) = self.initial_session.take() {
                for msg in loaded_thread.messages() {
                    let role = match msg.role {
                        nyzhi_provider::Role::User => "user",
                        nyzhi_provider::Role::Assistant => "assistant",
                        _ => "system",
                    };
                    let mut text = msg.content.as_text().to_string();
                    if msg.content.has_images() {
                        text.push_str("\n[image attached]");
                    }
                    if !text.is_empty() {
                        self.items.push(DisplayItem::Message {
                            role: role.to_string(),
                            content: text,
                        });
                    }
                }
                self.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: format!(
                        "Resumed session: {} ({} messages)",
                        loaded_meta.title, loaded_meta.message_count,
                    ),
                });
                loaded_thread
            } else {
                Thread::new()
            },
        );

        let mcp_tool_summaries = if let Some(mgr) = &self.mcp_manager {
            let mut summaries = Vec::new();
            for (server, tool_def) in mgr.all_tools().await {
                summaries.push(nyzhi_core::prompt::McpToolSummary {
                    server_name: server,
                    tool_name: tool_def.name.to_string(),
                    description: tool_def
                        .description
                        .as_deref()
                        .unwrap_or("MCP tool")
                        .to_string(),
                });
            }
            summaries
        } else {
            Vec::new()
        };

        let mut model_info_idx = provider.as_ref().and_then(|p| {
            p.supported_models()
                .iter()
                .position(|m| m.id == self.model_name)
                .or(if p.supported_models().is_empty() {
                    None
                } else {
                    Some(0)
                })
        });

        let supports_vision = provider.as_ref().map_or(false, |p| {
            model_info_idx
                .map(|i| p.supported_models()[i].supports_vision)
                .unwrap_or(false)
        });

        let skills =
            nyzhi_core::skills::load_skills(&self.workspace.project_root).unwrap_or_default();
        let skills_text = nyzhi_core::skills::format_skills_for_prompt(&skills);

        let mut sys_prompt = nyzhi_core::prompt::build_system_prompt_with_skills(
            Some(&self.workspace),
            config.agent.custom_instructions.as_deref(),
            &mcp_tool_summaries,
            supports_vision,
            &skills_text,
        );
        if config.agent.auto_commit {
            sys_prompt.push_str(nyzhi_core::prompt::auto_commit_instructions());
        }
        if config.memory.auto_memory {
            let mem = nyzhi_core::memory::load_memory_for_prompt(&self.workspace.project_root);
            if !mem.is_empty() {
                sys_prompt.push_str(&mem);
            }
            sys_prompt.push_str(nyzhi_core::prompt::auto_memory_instructions());
        }
        let mut agent_config = AgentConfig {
            system_prompt: sys_prompt,
            max_steps: config.agent.max_steps.unwrap_or(100),
            max_tokens: config.agent.max_tokens,
            trust: config.agent.trust.clone(),
            retry: config.agent.retry.clone(),
            routing: config.agent.routing.clone(),
            auto_compact_threshold: config.agent.auto_compact_threshold,
            compact_instructions: config.agent.compact_instructions.clone(),
            auto_context: config.index.auto_context,
            auto_context_chunks: config.index.auto_context_chunks,
            ..AgentConfig::default()
        };
        self.trust_mode = agent_config.trust.mode.clone();
        self.hooks_config = config.agent.hooks.clone();
        let (hook_tx, hook_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        self.hook_tx = Some(hook_tx);
        self.hook_rx = Some(hook_rx);

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let change_tracker = std::sync::Arc::new(tokio::sync::Mutex::new(
            nyzhi_core::tools::change_tracker::ChangeTracker::new(),
        ));
        let sid = thread.as_ref().unwrap().id.clone();
        self.current_session_id = Some(sid.clone());

        if let Some(ref store) = self.todo_store {
            let store_c = store.clone();
            let pr = self.workspace.project_root.clone();
            let sid_c = sid.clone();
            tokio::spawn(async move {
                nyzhi_core::tools::todo::load_todos_into_store(&store_c, &pr, &sid_c).await;
            });
        }

        let tool_ctx = ToolContext {
            session_id: sid,
            cwd,
            project_root: self.workspace.project_root.clone(),
            depth: 0,
            event_tx: Some(event_tx.clone()),
            change_tracker: change_tracker.clone(),
            allowed_tool_names: None,
            team_name: None,
            agent_name: None,
            is_team_lead: false,
            todo_store: Some(
                self.todo_store
                    .clone()
                    .unwrap_or_else(|| nyzhi_core::tools::todo::shared_store()),
            ),
            index: self.codebase_index.clone(),
            sandbox_level: nyzhi_config::SandboxLevel::default(),
            subagent_model_overrides: Some(self.subagent_model_overrides.clone()),
            shared_context: Some(self.shared_context.clone()),
        };

        let agent_manager = if let Some(ref p) = provider {
            let agent_registry = std::sync::Arc::new(
                nyzhi_core::tools::default_registry(self.codebase_index.clone()).registry,
            );
            Some(std::sync::Arc::new(
                nyzhi_core::agent_manager::AgentManager::new(
                    p.clone(),
                    agent_registry,
                    event_tx.clone(),
                    config.agent.agents.max_threads,
                    config.agent.agents.max_depth,
                ),
            ))
        } else {
            None
        };

        if let Some(ref agent_manager) = agent_manager {
            let user_agent_roles =
                nyzhi_core::agent_roles::convert_user_roles(&config.agent.agents.roles);
            let file_agent_roles =
                nyzhi_core::agent_files::load_file_based_roles(&self.workspace.project_root);
            let mut all_user_roles = user_agent_roles;
            all_user_roles.extend(file_agent_roles);

            registry.register(Box::new(
                nyzhi_core::tools::spawn_agent::SpawnAgentTool::with_user_roles(
                    agent_manager.clone(),
                    all_user_roles,
                ),
            ));
            registry.register(Box::new(nyzhi_core::tools::send_input::SendInputTool::new(
                agent_manager.clone(),
            )));
            registry.register(Box::new(nyzhi_core::tools::wait_tool::WaitTool::new(
                agent_manager.clone(),
            )));
            registry.register(Box::new(
                nyzhi_core::tools::close_agent::CloseAgentTool::new(agent_manager.clone()),
            ));
            registry.register(Box::new(
                nyzhi_core::tools::resume_agent::ResumeAgentTool::new(agent_manager.clone()),
            ));
            registry.register(Box::new(nyzhi_core::tools::team::SpawnTeammateTool::new(
                agent_manager.clone(),
            )));
        }

        let registry = Arc::new(registry);

        // Background update check
        let update_config = config.update.clone();
        let (update_tx, mut update_rx) =
            tokio::sync::mpsc::channel::<nyzhi_core::updater::UpdateInfo>(1);
        if update_config.enabled {
            self.update_status = UpdateStatus::Checking;
            tokio::spawn(async move {
                if let Ok(Some(info)) = nyzhi_core::updater::check_for_update(&update_config).await
                {
                    let _ = update_tx.send(info).await;
                }
            });
        }

        loop {
            self.spinner.tick();
            self.logo_anim.tick();

            if let Ok(info) = update_rx.try_recv() {
                self.update_status = UpdateStatus::Available {
                    new_version: info.new_version.clone(),
                    current_version: info.current_version.clone(),
                    changelog: info.changelog.clone(),
                };
                self.update_info = Some(info);
            }

            if let Some(ref mut rx) = self.update_done_rx {
                if let Ok(result) = rx.try_recv() {
                    self.update_done_rx = None;
                    self.update_status = UpdateStatus::None;
                    match result {
                        Ok(ur) => {
                            let mut msg = format!(
                                "Updated to v{}! Restart nyzhi to use the new version.",
                                ur.new_version
                            );
                            if let Some(ref bp) = ur.backup_path {
                                msg.push_str(&format!("\n  Backup saved to: {}", bp.display()));
                            }
                            if ur.verified {
                                msg.push_str("\n  Post-flight verification: passed");
                            }
                            // Run startup health check for integrity warnings
                            let warnings = nyzhi_core::updater::startup_health_check();
                            for w in &warnings {
                                msg.push_str(&format!("\n  Warning: {w}"));
                            }
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: msg,
                            });
                        }
                        Err(e) => {
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Update failed: {e:#}"),
                            });
                        }
                    }
                }
            }

            if let Some(ref idx) = self.codebase_index {
                if let Some((_, _, complete)) = self.index_progress {
                    if !complete {
                        use futures::FutureExt;
                        if let Some(p) = idx.progress().now_or_never() {
                            self.index_progress = Some((p.indexed, p.total, p.complete));
                            if p.complete && !p.errors.is_empty() {
                                let n = p.errors.len();
                                let first = &p.errors[0];
                                let msg = if n == 1 {
                                    format!("Index: 1 file skipped ({})", first.1)
                                } else {
                                    format!("Index: {} files skipped (first: {})", n, first.1)
                                };
                                self.index_error = Some(msg);
                            }
                        }
                    }
                }
            }

            terminal.draw(|frame| draw(frame, self, &self.theme, &self.spinner))?;

            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                    Event::Paste(text) => {
                        if let Some(ref mut sel) = self.selector {
                            if matches!(
                                sel.kind,
                                SelectorKind::ApiKeyInput | SelectorKind::CustomModelInput
                            ) {
                                sel.search.push_str(&text);
                            }
                        } else if matches!(self.mode, AppMode::Input) {
                            self.input.insert_str(self.cursor_pos, &text);
                            self.cursor_pos += text.len();
                        }
                    }
                    Event::Key(key) => {
                        let update_key_handled = self.handle_update_key(key);
                        if update_key_handled {
                            // handled by update banner
                        } else if self.todo_panel.is_some() {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('q') => {
                                    self.todo_panel = None;
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if let Some(ref mut tp) = self.todo_panel {
                                        tp.scroll_up();
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if let Some(ref mut tp) = self.todo_panel {
                                        tp.scroll_down();
                                    }
                                }
                                _ => {}
                            }
                        } else if self.text_prompt.is_some() {
                            self.handle_text_prompt_key(key, config).await;
                        } else if self.settings_panel.is_some() {
                            let action = self
                                .settings_panel
                                .as_mut()
                                .unwrap()
                                .handle_key(key);
                            self.handle_settings_action(action);
                        } else if self.selector.is_some() {
                            self.handle_selector_key(key, &mut model_info_idx, &mut agent_config)
                                .await;
                        } else if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            self.should_quit = true;
                        } else if key.code == KeyCode::Char('k')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            self.open_command_selector();
                        } else if key.code == KeyCode::Char(',')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            self.open_settings_panel();
                        } else if key.code == KeyCode::Char('t')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            self.open_theme_selector();
                        } else if key.code == KeyCode::Char('l')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            self.items.clear();
                            if let Some(t) = thread.as_mut() {
                                t.clear();
                            }
                            self.input.clear();
                            self.cursor_pos = 0;
                        } else if key.code == KeyCode::Char('b')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(self.mode, AppMode::Streaming)
                        {
                            if let Some(fg) = self.foreground_task.take() {
                                thread = Some(fg.thread_snapshot);
                                let id = self.background_next_id;
                                self.background_next_id += 1;
                                let label = fg.label.clone();
                                self.background_tasks.push(BackgroundTask {
                                    id,
                                    label: fg.label,
                                    join_handle: fg.join_handle,
                                    started: std::time::Instant::now(),
                                });
                                if !self.current_stream.is_empty() {
                                    self.current_stream.clear();
                                }
                                self.thinking_stream.clear();
                                self.stream_start = None;
                                self.stream_token_count = 0;
                                self.turn_start = None;
                                self.mode = AppMode::Input;
                                self.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!("Task moved to background (#{id}: {label})"),
                                });
                            }
                        } else if key.code == KeyCode::Esc
                            && matches!(self.mode, AppMode::Streaming)
                        {
                            if let Some(fg) = self.foreground_task.take() {
                                fg.join_handle.abort();
                                thread = Some(fg.thread_snapshot);
                            }
                            if !self.current_stream.is_empty() {
                                self.items.push(DisplayItem::Message {
                                    role: "assistant".to_string(),
                                    content: std::mem::take(&mut self.current_stream),
                                });
                            }
                            self.thinking_stream.clear();
                            self.stream_start = None;
                            self.stream_token_count = 0;
                            self.turn_start = None;
                            self.mode = AppMode::Input;
                            self.try_save_session(thread.as_ref());
                            if self.autopilot.is_some() {
                                if let Some(ref mut ap) = self.autopilot {
                                    ap.cancel();
                                    let _ = nyzhi_core::autopilot::save_state(
                                        &tool_ctx.project_root,
                                        ap,
                                    );
                                }
                                self.autopilot = None;
                                self.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Autopilot cancelled.".to_string(),
                                });
                            } else {
                                self.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Cancelled.".to_string(),
                                });
                            }
                        } else if key.code == KeyCode::Char('f')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(self.mode, AppMode::Input)
                            && !self.background_tasks.is_empty()
                        {
                            if self.ctrl_f_pending {
                                let count = self.background_tasks.len();
                                for bg in self.background_tasks.drain(..) {
                                    bg.join_handle.abort();
                                }
                                self.ctrl_f_pending = false;
                                self.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!("Killed {count} background task(s)"),
                                });
                            } else {
                                self.ctrl_f_pending = true;
                                self.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!(
                                        "Press Ctrl+F again to kill {} background task(s)",
                                        self.background_tasks.len()
                                    ),
                                });
                            }
                        } else if matches!(self.mode, AppMode::AwaitingApproval) {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                                    self.respond_approval(false).await;
                                }
                                KeyCode::Char('y') | KeyCode::Char('Y') => {
                                    self.respond_approval(true).await;
                                }
                                KeyCode::Left => {
                                    if self.approval_cursor > 0 {
                                        self.approval_cursor -= 1;
                                    }
                                }
                                KeyCode::Right => {
                                    if self.approval_cursor < 2 {
                                        self.approval_cursor += 1;
                                    }
                                }
                                KeyCode::Enter => match self.approval_cursor {
                                    0 => self.respond_approval(true).await,
                                    1 => self.respond_approval(false).await,
                                    2 => {
                                        if let Some((ref tool, _)) =
                                            self.pending_approval_context
                                        {
                                            self.session_approved_tools
                                                .insert(tool.clone());
                                        }
                                        self.respond_approval(true).await;
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        } else {
                            if key.code != KeyCode::Char('f')
                                || !key.modifiers.contains(KeyModifiers::CONTROL)
                            {
                                self.ctrl_f_pending = false;
                            }
                            let mi = provider
                                .as_ref()
                                .and_then(|p| model_info_idx.map(|i| &p.supported_models()[i]));
                            handle_key(
                                self,
                                key,
                                provider.as_deref(),
                                thread.as_mut(),
                                &mut agent_config,
                                &event_tx,
                                &registry,
                                &tool_ctx,
                                mi,
                                &mut model_info_idx,
                            )
                            .await;
                        }
                    }
                    Event::Mouse(_) => {},
                    _ => {}
                }
            }

            if self.pending_command_dispatch {
                self.pending_command_dispatch = false;
                let mi = provider
                    .as_ref()
                    .and_then(|p| model_info_idx.map(|i| &p.supported_models()[i]));
                let enter = crossterm::event::KeyEvent::new(
                    KeyCode::Enter,
                    crossterm::event::KeyModifiers::NONE,
                );
                handle_key(
                    self,
                    enter,
                    provider.as_deref(),
                    thread.as_mut(),
                    &mut agent_config,
                    &event_tx,
                    &registry,
                    &tool_ctx,
                    mi,
                    &mut model_info_idx,
                )
                .await;
            }

            if let Some((provider_id, method)) = self.pending_oauth.take() {
                let (tx, rx) = tokio::sync::oneshot::channel();
                let (msg_tx, msg_rx) = tokio::sync::mpsc::unbounded_channel();
                self.oauth_rx = Some(rx);
                self.oauth_msg_rx = Some(msg_rx);
                tokio::spawn(async move {
                    let result =
                        nyzhi_auth::oauth::login_interactive(&provider_id, &method, msg_tx).await;
                    let _ = tx.send((provider_id, result));
                });
            }

            if let Some(ref mut rx) = self.oauth_msg_rx {
                while let Ok(msg) = rx.try_recv() {
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: msg,
                    });
                    self.scroll_offset = 0;
                }
            }

            if let Some(ref mut rx) = self.oauth_rx {
                if let Ok(result) = rx.try_recv() {
                    self.oauth_rx = None;
                    self.oauth_msg_rx = None;
                    let (pid, res) = result;
                    let display = nyzhi_config::find_provider_def(&pid)
                        .map(|d| d.name)
                        .unwrap_or(&pid);
                    match res {
                        Ok(_token) => {
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Logged in to {display} via OAuth."),
                            });
                            self.pending_provider_reload = Some(pid.clone());
                        }
                        Err(e) => {
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("OAuth login failed for {display}: {e:#}"),
                            });
                        }
                    }
                }
            }

            if let Some(reload_provider_id) = self.pending_provider_reload.take() {
                {
                    let cache = self.model_cache.clone();
                    let pid = reload_provider_id.clone();
                    tokio::spawn(async move {
                        let _ = nyzhi_provider::refresh_provider_models(&pid, &cache).await;
                    });
                }
                match nyzhi_provider::create_provider_async(&reload_provider_id, config).await {
                    Ok(new_prov) => {
                        let new_prov: std::sync::Arc<dyn Provider> = new_prov.into();
                        let default_model = new_prov
                            .supported_models()
                            .first()
                            .map(|m| m.id.clone())
                            .unwrap_or_default();
                        model_info_idx = new_prov
                            .supported_models()
                            .iter()
                            .position(|m| m.id == default_model)
                            .or(Some(0));
                        self.provider_name = reload_provider_id.clone();
                        self.model_name = default_model;
                        provider = Some(new_prov);
                        let display = nyzhi_config::find_provider_def(&reload_provider_id)
                            .map(|d| d.name)
                            .unwrap_or(&reload_provider_id);
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Switched to {display} ({}).", self.model_name),
                        });
                    }
                    Err(e) => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Provider reload failed: {e:#}"),
                        });
                    }
                }
            }

            if self.wants_editor {
                self.wants_editor = false;
                Self::open_external_editor(self, &mut terminal)?;
            }

            // --- Spawn turn from request set by handle_key ---
            if let Some(req) = self.turn_request.take() {
                let Some(ref provider) = provider else {
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No provider configured. Run /login or set an API key in ~/.config/nyzhi/config.toml".to_string(),
                    });
                    self.turn_request = None;
                    continue;
                };
                let mi_c = model_info_idx.map(|i| provider.supported_models()[i].clone());
                if req.is_background {
                    let bg_thread = thread.as_ref().unwrap().clone();
                    let bg_usage = self.session_usage.clone();
                    let (bg_event_tx, _) = broadcast::channel::<AgentEvent>(256);
                    let provider_c = provider.clone();
                    let registry_c = registry.clone();
                    let mut config_c = agent_config.clone();
                    config_c.plan_mode = self.plan_mode || config_c.plan_mode;
                    let tool_ctx_c = tool_ctx.clone();
                    let join_handle = tokio::spawn(async move {
                        let mut t = bg_thread;
                        let mut u = bg_usage;
                        let result = if let Some(content) = req.content {
                            nyzhi_core::agent::run_turn_with_content(
                                &*provider_c,
                                &mut t,
                                content,
                                &config_c,
                                &bg_event_tx,
                                &registry_c,
                                &tool_ctx_c,
                                mi_c.as_ref(),
                                &mut u,
                            )
                            .await
                        } else {
                            nyzhi_core::agent::run_turn(
                                &*provider_c,
                                &mut t,
                                &req.input,
                                &config_c,
                                &bg_event_tx,
                                &registry_c,
                                &tool_ctx_c,
                                mi_c.as_ref(),
                                &mut u,
                            )
                            .await
                        };
                        TurnResult {
                            thread: t,
                            session_usage: u,
                            result,
                        }
                    });
                    let id = self.background_next_id;
                    self.background_next_id += 1;
                    self.background_tasks.push(BackgroundTask {
                        id,
                        label: req.label.clone(),
                        join_handle,
                        started: std::time::Instant::now(),
                    });
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Background task #{id} started: {}", req.label),
                    });
                } else {
                    let is_deep = nyzhi_core::deep_mode::is_deep_prefix(&req.input);
                    let mut req = req;
                    if is_deep {
                        req.input = nyzhi_core::deep_mode::strip_deep_prefix(&req.input).to_string();
                    }

                    if is_deep {
                        if let Some(ref mut mgr) = self.checkpoint_manager {
                            let mc = thread.as_ref().map(|t| t.messages().len()).unwrap_or(0);
                            mgr.auto_save(mc);
                        }
                    }

                    let fg_thread = thread.take().unwrap();
                    let snapshot = fg_thread.clone();
                    let fg_usage = self.session_usage.clone();
                    let provider_c = provider.clone();
                    let registry_c = registry.clone();
                    let mut config_c = agent_config.clone();
                    config_c.plan_mode = self.plan_mode || config_c.plan_mode;
                    config_c.act_after_plan = req.label == "execute plan";
                    if is_deep {
                        config_c.system_prompt.push_str("\n\n");
                        config_c.system_prompt.push_str(nyzhi_core::deep_mode::deep_mode_system_suffix());
                        config_c.max_steps = config_c.max_steps.max(200);
                    }
                    let event_tx_c = event_tx.clone();
                    let tool_ctx_c = tool_ctx.clone();
                    let join_handle = tokio::spawn(async move {
                        let mut t = fg_thread;
                        let mut u = fg_usage;
                        let result = if let Some(content) = req.content {
                            nyzhi_core::agent::run_turn_with_content(
                                &*provider_c,
                                &mut t,
                                content,
                                &config_c,
                                &event_tx_c,
                                &registry_c,
                                &tool_ctx_c,
                                mi_c.as_ref(),
                                &mut u,
                            )
                            .await
                        } else {
                            nyzhi_core::agent::run_turn(
                                &*provider_c,
                                &mut t,
                                &req.input,
                                &config_c,
                                &event_tx_c,
                                &registry_c,
                                &tool_ctx_c,
                                mi_c.as_ref(),
                                &mut u,
                            )
                            .await
                        };
                        TurnResult {
                            thread: t,
                            session_usage: u,
                            result,
                        }
                    });
                    self.foreground_task = Some(ForegroundTask {
                        join_handle,
                        thread_snapshot: snapshot,
                        label: req.label,
                    });
                }
            }

            // --- Foreground task completion ---
            if self
                .foreground_task
                .as_ref()
                .is_some_and(|f| f.join_handle.is_finished())
            {
                let fg = self.foreground_task.take().unwrap();
                match fg.join_handle.await {
                    Ok(result) => {
                        self.session_usage = result.session_usage;
                        thread = Some(result.thread);
                        if let Err(e) = &result.result {
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Turn error: {e}"),
                            });
                        }
                    }
                    Err(e) => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Task panicked: {e}"),
                        });
                    }
                }
                if !self.current_stream.is_empty() {
                    self.items.push(DisplayItem::Message {
                        role: "assistant".to_string(),
                        content: std::mem::take(&mut self.current_stream),
                    });
                }
                self.thinking_stream.clear();
                self.stream_start = None;
                self.stream_token_count = 0;
                self.turn_start = None;
                self.mode = AppMode::Input;
                self.try_save_session(thread.as_ref());
            }

            // --- Background task completion ---
            let mut bg_completed = Vec::new();
            for (i, bg) in self.background_tasks.iter().enumerate() {
                if bg.join_handle.is_finished() {
                    bg_completed.push(i);
                }
            }
            for i in bg_completed.into_iter().rev() {
                let bg = self.background_tasks.remove(i);
                let elapsed = bg.started.elapsed();
                match bg.join_handle.await {
                    Ok(result) => {
                        let last_msg = result
                            .thread
                            .messages()
                            .last()
                            .map(|m| {
                                let text = m.content.as_text().to_string();
                                if text.len() > 500 {
                                    format!("{}...", &text[..500])
                                } else {
                                    text
                                }
                            })
                            .unwrap_or_default();
                        let status = if result.result.is_ok() {
                            "completed"
                        } else {
                            "failed"
                        };
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Background task #{} {status} ({:.1}s): {}\n{}",
                                bg.id,
                                elapsed.as_secs_f64(),
                                bg.label,
                                if last_msg.is_empty() {
                                    "(no output)".to_string()
                                } else {
                                    last_msg
                                },
                            ),
                        });
                        if let Err(e) = &result.result {
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("  Error: {e}"),
                            });
                        }
                    }
                    Err(e) => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Background task #{} panicked: {e}", bg.id),
                        });
                    }
                }
            }

            // --- Drain agent events (only display for foreground) ---
            let has_foreground = self.foreground_task.is_some();
            while let Ok(agent_event) = event_rx.try_recv() {
                if !has_foreground {
                    match &agent_event {
                        AgentEvent::Usage(usage) => {
                            self.session_usage = usage.clone();
                        }
                        _ => continue,
                    }
                    continue;
                }
                match agent_event {
                    AgentEvent::ThinkingDelta(text) => {
                        if self.turn_start.is_none() {
                            self.turn_start = Some(std::time::Instant::now());
                        }
                        self.thinking_stream.push_str(&text);
                    }
                    AgentEvent::TextDelta(text) => {
                        if self.turn_start.is_none() {
                            self.turn_start = Some(std::time::Instant::now());
                        }
                        if self.stream_start.is_none() {
                            self.stream_start = Some(std::time::Instant::now());
                        }
                        let word_count = text.split_whitespace().count();
                        self.stream_token_count += (word_count as f64 * 1.3) as usize;
                        self.current_stream.push_str(&text);
                    }
                    AgentEvent::ToolCallStart { name, .. } => {
                        if self.turn_start.is_none() {
                            self.turn_start = Some(std::time::Instant::now());
                        }
                        if !self.thinking_stream.is_empty() {
                            self.items.push(DisplayItem::Thinking(std::mem::take(
                                &mut self.thinking_stream,
                            )));
                        }
                        if !self.current_stream.is_empty() {
                            self.items.push(DisplayItem::Message {
                                role: "assistant".to_string(),
                                content: std::mem::take(&mut self.current_stream),
                            });
                        }
                        self.items.push(DisplayItem::ToolCall {
                            name,
                            args_summary: String::new(),
                            output: None,
                            status: ToolStatus::Running,
                            elapsed_ms: None,
                        });
                    }
                    AgentEvent::ToolCallDelta { args_delta, .. } => {
                        if let Some(DisplayItem::ToolCall {
                            args_summary,
                            status,
                            ..
                        }) = self.items.last_mut()
                        {
                            if *status == ToolStatus::Running {
                                args_summary.push_str(&args_delta);
                            }
                        }
                    }
                    AgentEvent::ToolCallDone {
                        name,
                        output,
                        elapsed_ms: ev_elapsed,
                        ..
                    } => {
                        if let Some(DisplayItem::ToolCall {
                            name: ref item_name,
                            output: ref mut item_output,
                            status,
                            elapsed_ms,
                            ..
                        }) = self.items.last_mut()
                        {
                            if *item_name == name
                                && (*status == ToolStatus::Running
                                    || *status == ToolStatus::WaitingApproval)
                            {
                                *item_output = Some(truncate_display(&output, 500));
                                *status = ToolStatus::Completed;
                                *elapsed_ms = Some(ev_elapsed);
                            }
                        }
                        if name == "write" || name == "edit" || name == "multi_edit" || name == "apply_patch" {
                            let tracker = change_tracker.clone();
                            let last_change = {
                                let guard = futures::executor::block_on(tracker.lock());
                                guard.last().map(|c| {
                                    (
                                        c.path.display().to_string(),
                                        c.original.clone(),
                                        c.new_content.clone(),
                                    )
                                })
                            };
                            if let Some((file_path, old_content, new_content)) = last_change {
                                let rel = file_path
                                    .strip_prefix(
                                        &self.workspace.project_root.display().to_string(),
                                    )
                                    .unwrap_or(&file_path)
                                    .trim_start_matches('/')
                                    .to_string();
                                let diff_item = generate_diff(
                                    &rel,
                                    old_content.as_deref(),
                                    &new_content,
                                );
                                if let DisplayItem::Diff { ref hunks, .. } = diff_item {
                                    if !hunks.is_empty() {
                                        self.items.push(diff_item);
                                    }
                                }

                                let fmt_path = rel.clone();
                                let fmt_root = self.workspace.project_root.clone();
                                if let Some(tx) = self.hook_tx.clone() {
                                    tokio::spawn(async move {
                                        if let Some(r) = nyzhi_core::formatter::format_file_async(
                                            fmt_path,
                                            fmt_root,
                                        ).await {
                                            if r.success {
                                                let _ = tx.send(format!("Formatted with {}", r.formatter));
                                            }
                                        }
                                    });
                                }
                            }
                        }

                        if name == "todowrite" || name == "todoread" {
                            if let Some(ref store) = self.todo_store {
                                let store_c = store.clone();
                                let sid = thread.as_ref().map(|t| t.id.clone()).unwrap_or_default();
                                self.todo_progress = futures::executor::block_on(
                                    nyzhi_core::tools::todo_progress(&store_c, &sid),
                                );
                                if self.todo_panel.is_some() {
                                    use crate::components::todo_panel::{
                                        TodoPanelItem, TodoPanelState,
                                    };
                                    let items = store_c.blocking_lock();
                                    let panel_items: Vec<TodoPanelItem> = items
                                        .values()
                                        .flat_map(|todos| {
                                            todos.iter().map(|t| TodoPanelItem {
                                                id: t.id.clone(),
                                                content: t.content.clone(),
                                                status: t.status.clone(),
                                                blocked_by: t.blocked_by.clone(),
                                            })
                                        })
                                        .collect();
                                    let old_scroll =
                                        self.todo_panel.as_ref().map(|p| p.scroll).unwrap_or(0);
                                    self.todo_panel = Some(TodoPanelState {
                                        items: panel_items,
                                        scroll: old_scroll,
                                        enforcer_active: !self.todo_enforcement_paused,
                                        enforce_count: self.todo_enforce_count,
                                    });
                                }
                            }
                        }
                        if name == "create_plan" || name == "update_plan" {
                            if let Some(plan_name) = output
                                .strip_prefix("Plan '")
                                .and_then(|s| s.split('\'').next())
                            {
                                self.last_plan_name = Some(plan_name.to_string());
                            }
                            self.reload_plan_panel();
                        }
                        const FILE_TOOLS: &[&str] =
                            &["edit", "write", "delete_file", "move_file", "copy_file"];
                        if FILE_TOOLS.contains(&name.as_str()) && !self.hooks_config.is_empty() {
                            let tracker = change_tracker.clone();
                            let hooks = self.hooks_config.clone();
                            let hook_cwd = tool_ctx.cwd.clone();
                            if let Some(tx) = self.hook_tx.clone() {
                                tokio::spawn(async move {
                                    let changed_file = {
                                        let guard = tracker.lock().await;
                                        guard.last().map(|c| c.path.display().to_string())
                                    };
                                    if let Some(file) = changed_file {
                                        let results = nyzhi_core::hooks::run_after_edit_hooks(
                                            &hooks, &file, &hook_cwd,
                                        )
                                        .await;
                                        for r in results {
                                            let _ = tx.send(r.summary());
                                        }
                                    }
                                });
                            }
                        }
                    }
                    AgentEvent::ToolOutputDelta { tool_name, delta } => {
                        if let Some(DisplayItem::ToolCall {
                            name,
                            output,
                            status,
                            ..
                        }) = self.items.last_mut()
                        {
                            if *name == tool_name && *status == ToolStatus::Running {
                                let out = output.get_or_insert_with(String::new);
                                out.push_str(&delta);
                                out.push('\n');
                                if out.len() > 4096 {
                                    let trim_point = out.len() - 3072;
                                    *out = format!(
                                        "... (earlier output trimmed)\n{}",
                                        &out[trim_point..]
                                    );
                                }
                            }
                        }
                    }
                    AgentEvent::ApprovalRequest {
                        tool_name,
                        args_summary,
                        respond,
                    } => {
                        if self.session_approved_tools.contains(&tool_name) {
                            let mut guard = respond.lock().await;
                            if let Some(sender) = guard.take() {
                                let _ = sender.send(true);
                            }
                        } else {
                            if let Some(DisplayItem::ToolCall {
                                name: ref item_name,
                                status,
                                ..
                            }) = self.items.last_mut()
                            {
                                if *item_name == tool_name {
                                    *status = ToolStatus::WaitingApproval;
                                }
                            }
                            self.pending_approval = Some(respond);
                            self.pending_approval_context =
                                Some((tool_name.clone(), args_summary));
                            self.mode = AppMode::AwaitingApproval;
                        }
                    }
                    AgentEvent::Retrying {
                        attempt,
                        max_retries,
                        wait_ms,
                        reason,
                    } => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Retrying ({attempt}/{max_retries}) in {wait_ms}ms: {reason}"
                            ),
                        });
                    }
                    AgentEvent::AutoCompacting {
                        estimated_tokens,
                        context_window,
                    } => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Auto-compacting context ({estimated_tokens} tokens / {context_window} window)"
                            ),
                        });
                    }
                    AgentEvent::SystemMessage(msg) => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: msg.clone(),
                        });
                    }
                    AgentEvent::RoutedModel { model_name, tier } => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Routed to {model_name} (tier: {tier})"),
                        });
                    }
                    AgentEvent::SubAgentSpawned { nickname, role, .. } => {
                        let role_str = role.as_deref().unwrap_or("default");
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Spawned sub-agent {nickname} (role: {role_str})"),
                        });
                    }
                    AgentEvent::SubAgentStatusChanged {
                        nickname, status, ..
                    } => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Agent {nickname}: {status}"),
                        });
                    }
                    AgentEvent::SubAgentCompleted {
                        nickname,
                        final_message,
                        ..
                    } => {
                        let preview = final_message
                            .as_deref()
                            .map(|m| {
                                if m.len() > 200 {
                                    format!("{}...", &m[..200])
                                } else {
                                    m.to_string()
                                }
                            })
                            .unwrap_or_else(|| "no output".to_string());
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Agent {nickname} completed: {preview}"),
                        });
                    }
                    AgentEvent::UserQuestion {
                        question,
                        options,
                        allow_custom,
                        respond,
                    } => {
                        use crate::components::selector::{
                            SelectorItem, SelectorKind as SK, SelectorState,
                        };

                        let mut items: Vec<SelectorItem> = options
                            .iter()
                            .map(|(val, label)| SelectorItem::entry(label, val))
                            .collect();

                        if allow_custom {
                            items.push(SelectorItem::entry("Custom...", "__custom__"));
                        }

                        let sel = SelectorState::new(SK::UserQuestion, &question, items, "");
                        self.selector = Some(sel);
                        self.pending_user_question = Some(respond);
                        self.mode = AppMode::AwaitingUserQuestion;

                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Agent asks: {}", question),
                        });
                    }
                    AgentEvent::ContextUpdate {
                        estimated_tokens,
                        context_window,
                    } => {
                        self.context_used_tokens = estimated_tokens;
                        self.context_window = context_window;
                    }
                    AgentEvent::Usage(usage) => {
                        self.session_usage = usage;
                    }
                    AgentEvent::TurnComplete => {
                        if !self.thinking_stream.is_empty() {
                            self.items.push(DisplayItem::Thinking(std::mem::take(
                                &mut self.thinking_stream,
                            )));
                        }
                        if !self.current_stream.is_empty() {
                            self.items.push(DisplayItem::Message {
                                role: "assistant".to_string(),
                                content: std::mem::take(&mut self.current_stream),
                            });
                        }
                        let turn_elapsed = self.turn_start.map(|t| t.elapsed());
                        self.last_turn_duration = turn_elapsed.map(|d| d.as_secs_f64());
                        self.stream_start = None;
                        self.stream_token_count = 0;
                        self.turn_start = None;
                        self.mode = AppMode::Input;

                        if self.context_window > 0 && self.context_used_tokens > 0 {
                            let pct = self.context_used_tokens as f64
                                / self.context_window as f64
                                * 100.0;
                            if pct >= 75.0 {
                                self.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!(
                                        "Context usage at {:.0}%. Quality may degrade. Consider `/compact` or `/handoff`.",
                                        pct
                                    ),
                                });
                            }
                        }

                        let should_notify = turn_elapsed
                            .map(|d| d.as_millis() as u64 >= self.notify.min_duration_ms)
                            .unwrap_or(false);
                        if should_notify {
                            if self.notify.bell {
                                let _ = crossterm::execute!(
                                    std::io::stdout(),
                                    crossterm::style::Print("\x07")
                                );
                            }
                            if self.notify.desktop {
                                let elapsed_str = if let Some(d) = turn_elapsed {
                                    format!("{:.1}s", d.as_secs_f64())
                                } else {
                                    "done".to_string()
                                };
                                tokio::spawn(async move {
                                    let _ = notify_rust::Notification::new()
                                        .summary("nyzhi code")
                                        .body(&format!("Turn complete ({elapsed_str})"))
                                        .show();
                                });
                            }
                        }
                        self.try_save_session(thread.as_ref());

                        {
                            let change_tracker_clone = tool_ctx.change_tracker.clone();
                            let shared_ctx = self.shared_context.clone();
                            let project_root = self.workspace.project_root.clone();
                            let display_items = self.items.clone();
                            let todo_store = self.todo_store.clone();
                            tokio::spawn(async move {
                                let changes: Vec<String> = {
                                    let ct = change_tracker_clone.lock().await;
                                    ct.changed_files()
                                        .iter()
                                        .map(|p| p.display().to_string())
                                        .collect()
                                };
                                let messages: Vec<String> = display_items
                                    .iter()
                                    .rev()
                                    .take(5)
                                    .filter_map(|item| {
                                        if let DisplayItem::Message { role, content } = item {
                                            let trunc = if content.len() > 200 {
                                                format!("{}...", &content[..200])
                                            } else {
                                                content.clone()
                                            };
                                            Some(format!("[{role}] {trunc}"))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                                let todos: Vec<(String, String)> = if let Some(ref store) = todo_store {
                                    let s = store.lock().await;
                                    s.values()
                                        .flat_map(|items| items.iter())
                                        .filter(|t| t.status != "completed")
                                        .map(|t| (t.id.clone(), t.content.clone()))
                                        .collect()
                                } else {
                                    Vec::new()
                                };
                                let mut sc = shared_ctx.lock().await;
                                sc.update_changes(changes);
                                sc.update_conversation(messages);
                                sc.update_todos(todos);
                                sc.project_root = Some(project_root);
                            });
                        }

                        if !self.hooks_config.is_empty() {
                            let hooks = self.hooks_config.clone();
                            let hook_cwd = tool_ctx.cwd.clone();
                            if let Some(tx) = self.hook_tx.clone() {
                                tokio::spawn(async move {
                                    let results =
                                        nyzhi_core::hooks::run_after_turn_hooks(&hooks, &hook_cwd)
                                            .await;
                                    for r in results {
                                        let _ = tx.send(r.summary());
                                    }
                                });
                            }
                        }

                        if let Some(ref mut ap) = self.autopilot {
                            let last_output = self
                                .items
                                .iter()
                                .rev()
                                .find_map(|item| {
                                    if let DisplayItem::Message { role, content } = item {
                                        if role == "assistant" {
                                            Some(content.clone())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or_default();

                            match ap.phase {
                                nyzhi_core::autopilot::AutopilotPhase::Expansion => {
                                    ap.requirements = Some(last_output.clone());
                                }
                                nyzhi_core::autopilot::AutopilotPhase::Planning => {
                                    ap.plan = Some(last_output.clone());
                                }
                                nyzhi_core::autopilot::AutopilotPhase::Execution => {
                                    ap.execution_log.push(last_output.clone());
                                }
                                nyzhi_core::autopilot::AutopilotPhase::Qa => {
                                    ap.qa_results.push(last_output.clone());
                                }
                                nyzhi_core::autopilot::AutopilotPhase::Validation => {
                                    ap.validation_report = Some(last_output.clone());
                                }
                                _ => {}
                            }

                            ap.advance();
                            let _ = nyzhi_core::autopilot::save_state(&tool_ctx.project_root, ap);

                            if ap.is_terminal() {
                                self.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!("Autopilot complete.\n\n{}", ap.summary()),
                                });
                                self.autopilot = None;
                            } else {
                                let next_prompt = match ap.phase {
                                    nyzhi_core::autopilot::AutopilotPhase::Planning => {
                                        let reqs = ap.requirements.as_deref().unwrap_or("");
                                        nyzhi_core::autopilot::build_planning_prompt(reqs, &ap.idea)
                                    }
                                    nyzhi_core::autopilot::AutopilotPhase::Execution => {
                                        let plan = ap.plan.as_deref().unwrap_or("");
                                        nyzhi_core::autopilot::build_execution_prompt(
                                            plan, &ap.idea,
                                        )
                                    }
                                    nyzhi_core::autopilot::AutopilotPhase::Qa => {
                                        nyzhi_core::autopilot::build_qa_prompt(&ap.idea)
                                    }
                                    nyzhi_core::autopilot::AutopilotPhase::Validation => {
                                        let qa =
                                            ap.qa_results.last().map(|s| s.as_str()).unwrap_or("");
                                        nyzhi_core::autopilot::build_validation_prompt(qa, &ap.idea)
                                    }
                                    _ => String::new(),
                                };

                                if !next_prompt.is_empty() {
                                    self.items.push(DisplayItem::Message {
                                        role: "system".to_string(),
                                        content: format!(
                                            "Autopilot advancing to phase: {}",
                                            ap.phase
                                        ),
                                    });
                                    self.turn_request = Some(TurnRequest {
                                        input: next_prompt,
                                        content: None,
                                        is_background: false,
                                        label: "autopilot".to_string(),
                                    });
                                    self.mode = AppMode::Streaming;
                                }
                            }
                        }

                        if let Some(ref store) = self.todo_store {
                            let store_c = store.clone();
                            let sid = thread.as_ref().map(|t| t.id.clone()).unwrap_or_default();

                            self.todo_progress = futures::executor::block_on(
                                nyzhi_core::tools::todo_progress(&store_c, &sid),
                            );

                            if self.autopilot.is_none()
                                && self.turn_request.is_none()
                                && !self.todo_enforcement_paused
                            {
                                let has = futures::executor::block_on(
                                    nyzhi_core::tools::todo_has_incomplete(&store_c, &sid),
                                );
                                if has && self.todo_enforce_count < 10 {
                                    self.todo_enforce_count += 1;
                                    let summary = futures::executor::block_on(
                                        nyzhi_core::tools::todo_incomplete_summary(&store_c, &sid),
                                    )
                                    .unwrap_or_default();
                                    let incomplete_count = summary.lines().count();
                                    let reminder = format!(
                                        "[SYSTEM - TODO ENFORCER (attempt {}/10)]\n\n\
                                         {incomplete_count} incomplete todo(s). You MUST complete them:\n\n\
                                         {summary}\n\n\
                                         Pick the next `pending` or `in_progress` item and execute it NOW.\n\
                                         Use `todowrite` to mark items `completed` as you finish.\n\
                                         Do NOT end your turn with incomplete todos."
                                    , self.todo_enforce_count);
                                    self.items.push(DisplayItem::Message {
                                        role: "system".to_string(),
                                        content: format!("Enforcer: {incomplete_count} incomplete todos (attempt {}/10)", self.todo_enforce_count),
                                    });
                                    self.turn_request = Some(TurnRequest {
                                        input: reminder,
                                        content: None,
                                        is_background: false,
                                        label: "todo-enforcer".to_string(),
                                    });
                                    self.mode = AppMode::Streaming;
                                } else if !has {
                                    self.todo_enforce_count = 0;
                                }
                            }
                        }

                        if self.turn_request.is_none() && !self.message_queue.is_empty() {
                            if let Some(next) = self.message_queue.pop_front() {
                                let remaining = self.message_queue.len();
                                self.items.push(DisplayItem::Message {
                                    role: "user".to_string(),
                                    content: next.input.clone(),
                                });
                                if remaining > 0 {
                                    self.items.push(DisplayItem::Message {
                                        role: "system".to_string(),
                                        content: format!(
                                            "Queued message submitted ({remaining} remaining)"
                                        ),
                                    });
                                }
                                self.turn_request = Some(next);
                                self.mode = AppMode::Streaming;
                            }
                        }
                    }
                    AgentEvent::Error(e) => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Error: {e}"),
                        });
                        self.mode = AppMode::Input;
                    }
                }
            }

            if let Some(ref mut rx) = self.hook_rx {
                while let Ok(msg) = rx.try_recv() {
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: msg,
                    });
                }
            }

            if self.should_quit {
                break;
            }
        }

        for bg in self.background_tasks.drain(..) {
            bg.join_handle.abort();
        }
        if let Some(fg) = self.foreground_task.take() {
            fg.join_handle.abort();
        }
        self.try_save_session(thread.as_ref());
        self.history.save();

        io::stdout().execute(DisableBracketedPaste)?;
        terminal::disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    fn handle_update_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        if !matches!(self.update_status, UpdateStatus::Available { .. }) {
            return false;
        }
        if !matches!(self.mode, AppMode::Input) || !self.input.is_empty() {
            return false;
        }
        if !key.modifiers.is_empty() {
            return false;
        }
        match key.code {
            KeyCode::Char('u') | KeyCode::Char('U') => {
                if let Some(info) = self.update_info.take() {
                    self.update_status = UpdateStatus::Downloading { progress: None };
                    let (done_tx, done_rx) = tokio::sync::mpsc::channel::<
                        anyhow::Result<nyzhi_core::updater::UpdateResult>,
                    >(1);
                    self.update_done_rx = Some(done_rx);
                    tokio::spawn(async move {
                        let result = nyzhi_core::updater::download_and_apply(&info).await;
                        let _ = done_tx.send(result).await;
                    });
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Backing up and downloading update...".to_string(),
                    });
                }
                true
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.update_status = UpdateStatus::None;
                self.update_info = None;
                true
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                if let UpdateStatus::Available {
                    ref new_version, ..
                } = self.update_status
                {
                    nyzhi_core::updater::skip_version(new_version);
                }
                self.update_status = UpdateStatus::None;
                self.update_info = None;
                true
            }
            _ => false,
        }
    }

    async fn handle_selector_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        model_info_idx: &mut Option<usize>,
        agent_config: &mut AgentConfig,
    ) {
        use crate::components::selector::{SelectorAction, SelectorKind};
        use crate::theme::{Accent, ThemePreset};

        let action = if let Some(sel) = &mut self.selector {
            sel.handle_key(key)
        } else {
            return;
        };

        match action {
            SelectorAction::Select(value) => {
                let kind = self.selector.as_ref().unwrap().kind;
                match kind {
                    SelectorKind::Theme => {
                        let preset = ThemePreset::from_name(&value);
                        self.theme = Theme::new(preset, self.theme.accent_type);
                        let _ = nyzhi_config::Config::save_tui_preferences(
                            preset.name(),
                            self.theme.accent_type.name(),
                        );
                    }
                    SelectorKind::Accent => {
                        let accent = Accent::from_name(&value);
                        self.theme = Theme::new(self.theme.preset, accent);
                        let _ = nyzhi_config::Config::save_tui_preferences(
                            self.theme.preset.name(),
                            accent.name(),
                        );
                    }
                    SelectorKind::Model => {
                        let is_thinking = self
                            .selector
                            .as_ref()
                            .and_then(|s| s.context_value.as_deref())
                            == Some("thinking");
                        if is_thinking {
                            let label = if value == "off" {
                                "off".to_string()
                            } else {
                                value.clone()
                            };
                            self.thinking_level = if value == "off" {
                                None
                            } else {
                                Some(value.clone())
                            };
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Thinking level set to: {}", label),
                            });
                        } else if value == "__all_providers__" {
                            self.selector = None;
                            self.open_provider_selector();
                            return;
                        } else if value.starts_with("__connect__/") {
                            let provider_id =
                                value.strip_prefix("__connect__/").unwrap().to_string();
                            self.selector = None;
                            let def = nyzhi_config::find_provider_def(&provider_id);
                            let has_oauth = def.map(|d| d.supports_oauth).unwrap_or(false);
                            if has_oauth {
                                self.open_connect_method(&provider_id);
                            } else {
                                self.open_api_key_input(&provider_id);
                            }
                            return;
                        } else if value.starts_with("__custom__/") {
                            let provider_id =
                                value.strip_prefix("__custom__/").unwrap().to_string();
                            self.selector = None;
                            self.open_custom_model_input(&provider_id);
                            return;
                        } else if let Some((prov, model_id)) = value.split_once('/') {
                            self.provider_name = prov.to_string();
                            self.model_name = model_id.to_string();
                            *model_info_idx = None;
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Switched to {}/{}", prov, model_id),
                            });
                        } else {
                            let idx = self.selector.as_ref().unwrap().cursor;
                            *model_info_idx = Some(idx);
                            self.model_name = value;
                        }
                    }
                    SelectorKind::Provider => {
                        self.selector = None;
                        let def = nyzhi_config::find_provider_def(&value);
                        let has_oauth = def.map(|d| d.supports_oauth).unwrap_or(false);
                        if has_oauth {
                            self.open_connect_method(&value);
                        } else {
                            self.open_api_key_input(&value);
                        }
                        return;
                    }
                    SelectorKind::ConnectMethod => {
                        let provider_id = self
                            .selector
                            .as_ref()
                            .and_then(|s| s.context_value.clone())
                            .unwrap_or_default();
                        self.selector = None;
                        if value == "apikey" {
                            self.open_api_key_input(&provider_id);
                        } else {
                            self.pending_oauth = Some((provider_id, value));
                        }
                        return;
                    }
                    SelectorKind::Command => {
                        self.selector = None;
                        match value.as_str() {
                            "/style" => {
                                self.open_style_selector();
                                return;
                            }
                            "/trust" => {
                                self.open_trust_selector();
                                return;
                            }
                            "/resume" | "/sessions" => {
                                self.open_session_selector();
                                return;
                            }
                            "/theme" => {
                                self.open_theme_selector();
                                return;
                            }
                            "/accent" => {
                                self.open_accent_selector();
                                return;
                            }
                            "/model" => {
                                self.open_model_selector();
                                return;
                            }
                            "/connect" => {
                                self.open_provider_selector();
                                return;
                            }
                            _ => {
                                self.input = value;
                                self.cursor_pos = self.input.len();
                                self.pending_command_dispatch = true;
                                return;
                            }
                        }
                    }
                    SelectorKind::Style => {
                        match value.as_str() {
                            "normal" => self.output_style = nyzhi_config::OutputStyle::Normal,
                            "verbose" => self.output_style = nyzhi_config::OutputStyle::Verbose,
                            "minimal" => self.output_style = nyzhi_config::OutputStyle::Minimal,
                            "structured" => {
                                self.output_style = nyzhi_config::OutputStyle::Structured
                            }
                            _ => {}
                        }
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Output style: {}", self.output_style),
                        });
                    }
                    SelectorKind::Trust => {
                        if let Ok(mode) = value.parse::<nyzhi_config::TrustMode>() {
                            agent_config.trust.mode = mode.clone();
                            self.trust_mode = mode;
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Trust mode: {}", value),
                            });
                        }
                    }
                    SelectorKind::Session => {
                        self.selector = None;
                        self.input = format!("/resume {}", value);
                        self.cursor_pos = self.input.len();
                        self.pending_command_dispatch = true;
                        return;
                    }
                    SelectorKind::CustomModelInput => {
                        let provider_id = self
                            .selector
                            .as_ref()
                            .and_then(|s| s.context_value.clone())
                            .unwrap_or_default();
                        let model_id = self
                            .selector
                            .as_ref()
                            .map(|s| s.search.trim().to_string())
                            .unwrap_or_default();
                        if !model_id.is_empty() {
                            self.provider_name = provider_id;
                            self.model_name = model_id.clone();
                            *model_info_idx = None;
                            self.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Switched to custom model: {}", model_id),
                            });
                        }
                    }
                    SelectorKind::ApiKeyInput => {
                        let provider_id = self
                            .selector
                            .as_ref()
                            .and_then(|s| s.context_value.clone())
                            .unwrap_or_default();
                        let api_key = self
                            .selector
                            .as_ref()
                            .map(|s| s.search.clone())
                            .unwrap_or_default();
                        if !api_key.is_empty() {
                            match nyzhi_auth::token_store::store_api_key(&provider_id, &api_key) {
                                Ok(()) => {
                                    self.pending_provider_reload = Some(provider_id.clone());
                                    self.items.push(DisplayItem::Message {
                                        role: "system".to_string(),
                                        content: format!("API key saved for {provider_id}."),
                                    });
                                }
                                Err(e) => {
                                    self.items.push(DisplayItem::Message {
                                        role: "system".to_string(),
                                        content: format!("Failed to save API key: {e}"),
                                    });
                                }
                            }
                        }
                    }
                    SelectorKind::UserQuestion => {
                        if value == "__custom__" {
                            let custom_text = self
                                .selector
                                .as_ref()
                                .map(|s| s.search.trim().to_string())
                                .unwrap_or_default();
                            self.selector = None;
                            if !custom_text.is_empty() {
                                self.respond_user_question(custom_text).await;
                            } else {
                                self.open_user_question_custom_input();
                            }
                            return;
                        }
                        self.selector = None;
                        self.respond_user_question(value).await;
                        return;
                    }
                    SelectorKind::PlanTransition => {
                        self.selector = None;
                        return;
                    }
                }
                self.selector = None;
            }
            SelectorAction::Cancel => {
                let was_user_question = self
                    .selector
                    .as_ref()
                    .map(|s| s.kind == SelectorKind::UserQuestion)
                    .unwrap_or(false);
                self.selector = None;
                if was_user_question {
                    self.respond_user_question("__cancelled__".to_string())
                        .await;
                }
            }
            SelectorAction::Tab => {
                let kind = self.selector.as_ref().map(|s| s.kind);
                if kind == Some(SelectorKind::Model) {
                    let is_thinking = self
                        .selector
                        .as_ref()
                        .and_then(|s| s.context_value.as_deref())
                        == Some("thinking");
                    if !is_thinking {
                        self.handle_model_tab(model_info_idx);
                    }
                }
            }
            SelectorAction::ViewAllProviders => {
                self.selector = None;
                self.open_provider_selector();
            }
            SelectorAction::None => {}
        }
        let _ = model_info_idx;
    }

    async fn handle_text_prompt_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        _config: &nyzhi_config::Config,
    ) {
        use crate::components::text_prompt::{TextPromptAction, TextPromptKind};

        let action = if let Some(prompt) = &mut self.text_prompt {
            prompt.handle_key(key)
        } else {
            return;
        };

        match action {
            TextPromptAction::Submit(value) => {
                let kind = self.text_prompt.as_ref().unwrap().kind;
                match kind {
                    TextPromptKind::ExaApiKey => {
                        self.text_prompt = None;
                        self.handle_exa_setup(value).await;
                    }
                    TextPromptKind::UserQuestionCustom => {
                        self.text_prompt = None;
                        self.respond_user_question(value).await;
                    }
                }
            }
            TextPromptAction::Cancel => {
                let kind = self.text_prompt.as_ref().map(|p| p.kind);
                self.text_prompt = None;
                if kind == Some(TextPromptKind::UserQuestionCustom) {
                    self.respond_user_question("__cancelled__".to_string())
                        .await;
                } else {
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Cancelled".to_string(),
                    });
                }
            }
            TextPromptAction::None => {}
        }
    }

    async fn handle_exa_setup(&mut self, api_key: String) {
        let mut env = std::collections::HashMap::new();
        env.insert("EXA_API_KEY".to_string(), api_key);
        let exa_config = nyzhi_config::McpServerConfig::Stdio {
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "exa-mcp-server".to_string()],
            env,
        };

        match nyzhi_config::Config::load() {
            Ok(mut global_config) => {
                global_config
                    .mcp
                    .servers
                    .insert("exa".to_string(), exa_config.clone());
                if let Err(e) = global_config.save() {
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Failed to save config: {e}"),
                    });
                    return;
                }
            }
            Err(e) => {
                self.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: format!("Failed to load config: {e}"),
                });
                return;
            }
        }

        if let Some(mcp) = &self.mcp_manager {
            match mcp.connect_server("exa", &exa_config).await {
                Ok(tool_count) => {
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!(
                            "Exa web search enabled! {tool_count} tool(s) registered.\n\
                             Restart nyzhi to fully activate Exa tools in the current session."
                        ),
                    });
                }
                Err(e) => {
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!(
                            "Exa config saved, but live connection failed: {e}\n\
                             Restart nyzhi to connect."
                        ),
                    });
                }
            }
        } else {
            self.items.push(DisplayItem::Message {
                role: "system".to_string(),
                content: "Exa config saved to ~/.config/nyzhi/config.toml.\n\
                         Restart nyzhi to enable Exa web search tools."
                    .to_string(),
            });
        }
    }

    pub fn open_theme_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};
        use crate::theme::ThemePreset;

        let items: Vec<SelectorItem> = ThemePreset::ALL
            .iter()
            .map(|p| SelectorItem::entry(p.display_name(), p.name()).with_color(p.bg_page_color()))
            .collect();
        self.selector = Some(SelectorState::new(
            SelectorKind::Theme,
            "Theme",
            items,
            self.theme.preset.name(),
        ));
    }

    pub fn open_accent_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};
        use crate::theme::Accent;

        let items: Vec<SelectorItem> = Accent::ALL
            .iter()
            .map(|a| {
                SelectorItem::entry(&capitalize(a.name()), a.name())
                    .with_color(a.color_preview(self.theme.mode))
            })
            .collect();
        self.selector = Some(SelectorState::new(
            SelectorKind::Accent,
            "Accent Color",
            items,
            self.theme.accent_type.name(),
        ));
    }

    pub fn toggle_plan_panel(&mut self) {
        self.show_plan_panel = !self.show_plan_panel;
        if self.show_plan_panel {
            self.reload_plan_panel();
        }
    }

    pub fn reload_plan_panel(&mut self) {
        let project_root = std::env::current_dir().unwrap_or_default();
        if let Some(sid) = &self.current_session_id {
            if let Ok(Some(plan)) = nyzhi_core::planning::load_session_plan(&project_root, sid) {
                self.plan_panel.load(plan);
            }
        }
    }

    pub fn exit_plan_mode_and_execute(&mut self) {
        self.plan_mode = false;
        let project_root = std::env::current_dir().unwrap_or_default();
        let plan_content = self.current_session_id.as_ref().and_then(|sid| {
            nyzhi_core::planning::load_session_plan(&project_root, sid)
                .ok()
                .flatten()
                .map(|p| nyzhi_core::planning::serialize_plan(&p))
        });

        let msg = if let Some(ref content) = plan_content {
            format!(
                "Execute the following plan step by step. Use `create_plan` to update todo statuses as you complete them.\n\n{content}"
            )
        } else {
            "Execute the plan above step by step. Use `create_plan` to update todo statuses as you go.".to_string()
        };

        self.items.push(DisplayItem::Message {
            role: "system".to_string(),
            content: "Switched to Act mode -- executing plan".to_string(),
        });
        self.items.push(DisplayItem::Message {
            role: "user".to_string(),
            content: msg.clone(),
        });
        self.turn_request = Some(TurnRequest {
            input: msg,
            content: None,
            is_background: false,
            label: "execute plan".to_string(),
        });
        self.mode = AppMode::Streaming;
    }

    pub fn open_model_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let registry = nyzhi_provider::ModelRegistry::new();
        let mut all_providers = registry.providers();
        let priority = [
            "openai",
            "anthropic",
            "gemini",
            "cursor",
            "github-copilot",
            "openrouter",
            "deepseek",
            "groq",
            "together",
            "ollama",
        ];
        all_providers.sort_by_key(|p| {
            priority
                .iter()
                .position(|&x| x == *p)
                .unwrap_or(priority.len())
        });

        let cache = &self.model_cache;

        let mut connected_providers: Vec<(&str, String, Vec<nyzhi_provider::ModelInfo>)> =
            Vec::new();
        let mut unconnected_providers: Vec<(&str, String)> = Vec::new();

        for provider_id in &all_providers {
            let status = nyzhi_auth::auth_status(provider_id);
            let display_name = nyzhi_config::find_provider_def(provider_id)
                .map(|d| d.name)
                .unwrap_or(provider_id)
                .to_string();
            if status == "not connected" {
                unconnected_providers.push((provider_id, display_name));
            } else {
                let models = nyzhi_provider::cached_or_hardcoded(provider_id, cache);
                if !models.is_empty() {
                    connected_providers.push((provider_id, display_name, models));
                }
            }
        }

        let mut items = Vec::new();

        for (provider_id, display_name, models) in &connected_providers {
            items.push(SelectorItem::header(display_name));
            for m in models {
                let thinking_badge = if m.has_thinking() {
                    if m.id == self.model_name && *provider_id == self.provider_name {
                        let level = self.thinking_level.as_deref().unwrap_or("off");
                        format!(" [{}]", level)
                    } else {
                        " [thinking]".to_string()
                    }
                } else {
                    String::new()
                };
                let ctx = m.context_display();
                let right = format!("{}{}", ctx, thinking_badge);
                let value = format!("{}/{}", provider_id, m.id);
                let mut item = SelectorItem::entry(&m.name, &value);
                item.right_badge = Some(right);
                items.push(item);
            }
            let mut custom = SelectorItem::entry("Custom model...", &format!("__custom__/{}", provider_id));
            custom.right_badge = Some("enter model ID".to_string());
            items.push(custom);
        }

        if !unconnected_providers.is_empty() {
            items.push(SelectorItem::header("Not Connected"));
            for (provider_id, display_name) in &unconnected_providers {
                let def = nyzhi_config::find_provider_def(provider_id);
                let hint = if def.map(|d| d.supports_oauth).unwrap_or(false) {
                    "OAuth or API key"
                } else {
                    "API key"
                };
                let mut item = SelectorItem::entry(display_name, &format!("__connect__/{}", provider_id));
                item.right_badge = Some(hint.to_string());
                items.push(item);
            }
        }

        let mut all_item = SelectorItem::entry("View all providers", "__all_providers__");
        all_item.right_badge = Some("ctrl+a".to_string());
        items.push(all_item);

        let current = format!("{}/{}", self.provider_name, self.model_name);
        self.selector = Some(SelectorState::new(
            SelectorKind::Model,
            "Select model",
            items,
            &current,
        ));
    }

    pub fn open_provider_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let mut items = Vec::new();
        let categories = [
            ("popular", "Popular"),
            ("agents", "Agents"),
            ("other", "Other"),
        ];
        for (cat_id, cat_name) in &categories {
            let providers: Vec<_> = nyzhi_config::BUILT_IN_PROVIDERS
                .iter()
                .filter(|d| d.category == *cat_id)
                .collect();
            if providers.is_empty() {
                continue;
            }
            items.push(SelectorItem::header(cat_name));
            for def in providers {
                let status = nyzhi_auth::auth_status(def.id);
                let auth_info = if def.supports_oauth {
                    format!(" ({}, OAuth or API key)", status)
                } else {
                    format!(" ({})", status)
                };
                let label = format!("{}{}", def.name, auth_info);
                items.push(SelectorItem::entry(&label, def.id));
            }
        }
        self.selector = Some(SelectorState::new(
            SelectorKind::Provider,
            "Connect a provider",
            items,
            &self.provider_name,
        ));
    }

    pub fn open_thinking_selector(&mut self, model_info: Option<&nyzhi_provider::ModelInfo>) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let thinking = model_info.and_then(|m| m.thinking.as_ref());
        let levels: Vec<(&str, &str)> = match thinking {
            Some(ts) => ts.user_facing_levels(),
            None => {
                self.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: "Current model does not support thinking/reasoning.".to_string(),
                });
                return;
            }
        };

        let items: Vec<SelectorItem> = levels
            .iter()
            .map(|(value, desc)| SelectorItem::entry(&format!("{:<12} {}", value, desc), value))
            .collect();

        let current = self.thinking_level.as_deref().unwrap_or("off");
        self.selector = Some(SelectorState::new(
            SelectorKind::Model,
            "Thinking Level",
            items,
            current,
        ));
        if let Some(sel) = &mut self.selector {
            sel.context_value = Some("thinking".to_string());
        }
    }

    pub fn open_connect_method(&mut self, provider_id: &str) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let def = nyzhi_config::find_provider_def(provider_id);
        let display_name = def.map(|d| d.name).unwrap_or(provider_id);
        let status = nyzhi_auth::auth_status(provider_id);

        let mut items = vec![];
        if status != "not connected" {
            items.push(SelectorItem::header(&format!("Currently: {status}")));
        }

        match provider_id {
            "openai" => {
                items.push(SelectorItem::entry(
                    "Codex subscription (device code login)",
                    "codex",
                ));
                items.push(SelectorItem::entry("Enter API key manually", "apikey"));
            }
            "gemini" => {
                items.push(SelectorItem::entry(
                    "Gemini CLI OAuth (free tier / paid plan)",
                    "gemini-cli",
                ));
                items.push(SelectorItem::entry("Enter API key manually", "apikey"));
            }
            "cursor" => {
                items.push(SelectorItem::entry(
                    "Import from Cursor IDE (auto)",
                    "cursor-auto",
                ));
                items.push(SelectorItem::entry("Paste token manually", "apikey"));
            }
            "anthropic" => {
                items.push(SelectorItem::entry(
                    "Claude Pro/Max subscription (OAuth)",
                    "oauth",
                ));
                items.push(SelectorItem::entry("Enter API key manually", "apikey"));
            }
            _ => {
                items.push(SelectorItem::entry(
                    "Login with OAuth (opens browser)",
                    "oauth",
                ));
                items.push(SelectorItem::entry("Enter API key manually", "apikey"));
            }
        }

        let mut state = SelectorState::new(
            SelectorKind::ConnectMethod,
            &format!("Connect {}", display_name),
            items,
            "",
        );
        state.context_value = Some(provider_id.to_string());
        self.selector = Some(state);
    }

    pub fn open_api_key_input(&mut self, provider_id: &str) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let display_name = nyzhi_config::find_provider_def(provider_id)
            .map(|d| d.name)
            .unwrap_or(provider_id);

        let items = vec![SelectorItem::entry(
            &format!("Paste your {} API key and press Enter", display_name),
            "submit",
        )];
        let mut state = SelectorState::new(
            SelectorKind::ApiKeyInput,
            &format!("{} API Key", display_name),
            items,
            "",
        );
        state.context_value = Some(provider_id.to_string());
        self.selector = Some(state);
    }

    pub fn open_command_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let categories: &[(&str, &[&str])] = &[
            ("Provider", &["/model", "/connect", "/login"]),
            (
                "Agent",
                &[
                    "/autopilot",
                    "/team",
                    "/qa",
                    "/persist",
                    "/think",
                    "/style",
                    "/trust",
                ],
            ),
            (
                "Session",
                &[
                    "/clear",
                    "/compact",
                    "/resume",
                    "/sessions",
                    "/export",
                    "/search",
                    "/retry",
                ],
            ),
            (
                "Project",
                &[
                    "/init",
                    "/doctor",
                    "/verify",
                    "/hooks",
                    "/mcp",
                    "/commands",
                    "/learn",
                ],
            ),
            (
                "View",
                &[
                    "/status", "/context", "/changes", "/todo", "/plan", "/notepad", "/bg",
                ],
            ),
            ("UI", &["/settings", "/theme", "/accent", "/thinking", "/notify", "/image"]),
            (
                "System",
                &["/help", "/bug", "/editor", "/enable_exa", "/undo", "/exit"],
            ),
        ];

        let cmd_defs: std::collections::HashMap<&str, &str> = crate::completion::SLASH_COMMANDS
            .iter()
            .map(|c| (c.name, c.description))
            .collect();

        let mut categorized = std::collections::HashSet::new();
        let mut items = Vec::new();
        for (cat_name, cmds) in categories {
            items.push(SelectorItem::header(cat_name));
            for &cmd in *cmds {
                categorized.insert(cmd);
                let desc = cmd_defs.get(cmd).copied().unwrap_or("");
                let label = format!("{:<18} {}", cmd, desc);
                items.push(SelectorItem::entry(&label, cmd));
            }
        }

        let mut uncategorized: Vec<&str> = crate::completion::SLASH_COMMANDS
            .iter()
            .map(|c| c.name)
            .filter(|name| !categorized.contains(name))
            .collect();
        uncategorized.sort_unstable();
        if !uncategorized.is_empty() {
            items.push(SelectorItem::header("More"));
            for cmd in uncategorized {
                let desc = cmd_defs.get(cmd).copied().unwrap_or("");
                let label = format!("{:<18} {}", cmd, desc);
                items.push(SelectorItem::entry(&label, cmd));
            }
        }

        if !self.custom_commands.is_empty() {
            items.push(SelectorItem::header("Custom"));
            let mut custom = self.custom_commands.clone();
            custom.sort_by(|a, b| a.name.cmp(&b.name));
            for cmd in custom {
                let slash_name = format!("/{}", cmd.name);
                let label = format!("{:<18} {}", slash_name, cmd.description);
                items.push(SelectorItem::entry(&label, &slash_name));
            }
        }

        self.selector = Some(SelectorState::new(
            SelectorKind::Command,
            "Commands",
            items,
            "",
        ));
    }

    pub fn open_style_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let current = self.output_style.to_string();
        let options = [
            ("normal", "Default output"),
            ("verbose", "Expand all tool args/outputs"),
            ("minimal", "Hide tool details"),
            ("structured", "JSON output"),
        ];
        let items: Vec<SelectorItem> = options
            .iter()
            .map(|(id, desc)| {
                let marker = if *id == current { " ●" } else { "" };
                SelectorItem::entry(&format!("{:<14} {}{}", id, desc, marker), id)
            })
            .collect();

        self.selector = Some(SelectorState::new(
            SelectorKind::Style,
            "Output Style",
            items,
            "",
        ));
    }

    pub fn open_settings_panel(&mut self) {
        use crate::components::settings_panel::*;

        let on_off = |b: bool| if b { "On" } else { "Off" }.to_string();

        let rows = vec![
            SettingsRow::Header("Display".into()),
            SettingsRow::Item(SettingItem {
                key: "theme".into(),
                label: "Theme".into(),
                description: "Color scheme for the entire interface".into(),
                kind: SettingKind::SubMenu,
                current_value: self.theme.preset.display_name().to_string(),
            }),
            SettingsRow::Item(SettingItem {
                key: "accent".into(),
                label: "Accent".into(),
                description: "Highlight color for active elements".into(),
                kind: SettingKind::SubMenu,
                current_value: self.theme.accent_type.name().to_string(),
            }),
            SettingsRow::Item(SettingItem {
                key: "output_style".into(),
                label: "Output style".into(),
                description: "Control verbosity of tool call output".into(),
                kind: SettingKind::Cycle {
                    options: vec![
                        "Normal".into(),
                        "Verbose".into(),
                        "Minimal".into(),
                        "Structured".into(),
                    ],
                },
                current_value: format!("{}", self.output_style),
            }),
            SettingsRow::Item(SettingItem {
                key: "show_thinking".into(),
                label: "Show thinking".into(),
                description: "Display model reasoning blocks in chat".into(),
                kind: SettingKind::Toggle,
                current_value: on_off(self.show_thinking),
            }),
            SettingsRow::Header("Agent".into()),
            SettingsRow::Item(SettingItem {
                key: "trust_mode".into(),
                label: "Trust mode".into(),
                description: "Permission level for agent actions".into(),
                kind: SettingKind::Cycle {
                    options: vec![
                        "Off".into(),
                        "Limited".into(),
                        "AutoEdit".into(),
                        "Full".into(),
                    ],
                },
                current_value: format!("{}", self.trust_mode),
            }),
            SettingsRow::Item(SettingItem {
                key: "plan_mode".into(),
                label: "Plan mode".into(),
                description: "Agent creates plans before executing".into(),
                kind: SettingKind::Toggle,
                current_value: on_off(self.plan_mode),
            }),
            SettingsRow::Header("Session".into()),
            SettingsRow::Item(SettingItem {
                key: "bell".into(),
                label: "Notifications".into(),
                description: "Terminal bell on agent completion".into(),
                kind: SettingKind::Toggle,
                current_value: on_off(self.notify.bell),
            }),
        ];

        self.settings_panel = Some(SettingsPanel::new(rows));
    }

    fn handle_settings_action(
        &mut self,
        action: crate::components::settings_panel::SettingsAction,
    ) {
        use crate::components::settings_panel::SettingsAction;
        let on_off = |b: bool| if b { "On" } else { "Off" };

        match action {
            SettingsAction::Toggle(key) => match key.as_str() {
                "show_thinking" => {
                    self.show_thinking = !self.show_thinking;
                    if let Some(p) = &mut self.settings_panel {
                        p.update_value("show_thinking", on_off(self.show_thinking));
                    }
                }
                "plan_mode" => {
                    self.plan_mode = !self.plan_mode;
                    if let Some(p) = &mut self.settings_panel {
                        p.update_value("plan_mode", on_off(self.plan_mode));
                    }
                }
                "bell" => {
                    self.notify.bell = !self.notify.bell;
                    if let Some(p) = &mut self.settings_panel {
                        p.update_value("bell", on_off(self.notify.bell));
                    }
                }
                _ => {}
            },
            SettingsAction::CycleNext(ref key) | SettingsAction::CyclePrev(ref key) => {
                let forward = matches!(action, SettingsAction::CycleNext(_));
                match key.as_str() {
                    "output_style" => {
                        self.output_style = if forward {
                            match self.output_style {
                                nyzhi_config::OutputStyle::Normal => {
                                    nyzhi_config::OutputStyle::Verbose
                                }
                                nyzhi_config::OutputStyle::Verbose => {
                                    nyzhi_config::OutputStyle::Minimal
                                }
                                nyzhi_config::OutputStyle::Minimal => {
                                    nyzhi_config::OutputStyle::Structured
                                }
                                nyzhi_config::OutputStyle::Structured => {
                                    nyzhi_config::OutputStyle::Normal
                                }
                            }
                        } else {
                            match self.output_style {
                                nyzhi_config::OutputStyle::Normal => {
                                    nyzhi_config::OutputStyle::Structured
                                }
                                nyzhi_config::OutputStyle::Verbose => {
                                    nyzhi_config::OutputStyle::Normal
                                }
                                nyzhi_config::OutputStyle::Minimal => {
                                    nyzhi_config::OutputStyle::Verbose
                                }
                                nyzhi_config::OutputStyle::Structured => {
                                    nyzhi_config::OutputStyle::Minimal
                                }
                            }
                        };
                        if let Some(p) = &mut self.settings_panel {
                            p.update_value("output_style", &format!("{}", self.output_style));
                        }
                    }
                    "trust_mode" => {
                        self.trust_mode = if forward {
                            match self.trust_mode {
                                nyzhi_config::TrustMode::Off => nyzhi_config::TrustMode::Limited,
                                nyzhi_config::TrustMode::Limited => {
                                    nyzhi_config::TrustMode::AutoEdit
                                }
                                nyzhi_config::TrustMode::AutoEdit => {
                                    nyzhi_config::TrustMode::Full
                                }
                                nyzhi_config::TrustMode::Full => nyzhi_config::TrustMode::Off,
                            }
                        } else {
                            match self.trust_mode {
                                nyzhi_config::TrustMode::Off => nyzhi_config::TrustMode::Full,
                                nyzhi_config::TrustMode::Limited => nyzhi_config::TrustMode::Off,
                                nyzhi_config::TrustMode::AutoEdit => {
                                    nyzhi_config::TrustMode::Limited
                                }
                                nyzhi_config::TrustMode::Full => {
                                    nyzhi_config::TrustMode::AutoEdit
                                }
                            }
                        };
                        if let Some(p) = &mut self.settings_panel {
                            p.update_value("trust_mode", &format!("{}", self.trust_mode));
                        }
                    }
                    _ => {}
                }
            }
            SettingsAction::OpenSub(key) => {
                self.settings_panel = None;
                match key.as_str() {
                    "theme" => self.open_theme_selector(),
                    "accent" => self.open_accent_selector(),
                    _ => {}
                }
            }
            SettingsAction::Close => {
                self.settings_panel = None;
            }
            SettingsAction::None => {}
        }
    }

    pub fn open_trust_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let current = self.trust_mode.to_string();
        let options = [
            ("off", "Confirm every action"),
            ("limited", "Auto-approve reads, confirm writes"),
            ("autoedit", "Auto-approve reads + edits"),
            ("full", "Auto-approve everything"),
        ];
        let items: Vec<SelectorItem> = options
            .iter()
            .map(|(id, desc)| {
                let marker = if *id == current { " ●" } else { "" };
                SelectorItem::entry(&format!("{:<14} {}{}", id, desc, marker), id)
            })
            .collect();

        self.selector = Some(SelectorState::new(
            SelectorKind::Trust,
            "Trust Mode",
            items,
            "",
        ));
    }

    pub fn open_custom_model_input(&mut self, provider_id: &str) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let display_name = nyzhi_config::find_provider_def(provider_id)
            .map(|d| d.name)
            .unwrap_or(provider_id);

        let items = vec![SelectorItem::entry(
            &format!("Type a model ID for {} and press Enter", display_name),
            "submit",
        )];
        let mut state = SelectorState::new(
            SelectorKind::CustomModelInput,
            &format!("{} Model ID", display_name),
            items,
            "",
        );
        state.context_value = Some(provider_id.to_string());
        self.selector = Some(state);
    }

    pub fn open_session_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        match nyzhi_core::session::list_sessions() {
            Ok(mut sessions) => {
                sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                if sessions.is_empty() {
                    self.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No saved sessions.".to_string(),
                    });
                    return;
                }
                let items: Vec<SelectorItem> = sessions
                    .iter()
                    .take(20)
                    .map(|s| {
                        let label = format!(
                            "{} ({} msgs, {})",
                            s.title,
                            s.message_count,
                            s.updated_at.format("%m/%d %H:%M"),
                        );
                        SelectorItem::entry(&label, &s.id)
                    })
                    .collect();

                self.selector = Some(SelectorState::new(
                    SelectorKind::Session,
                    "Resume Session",
                    items,
                    "",
                ));
            }
            Err(e) => {
                self.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: format!("Error listing sessions: {e}"),
                });
            }
        }
    }

    async fn respond_approval(&mut self, approved: bool) {
        self.pending_approval_context = None;
        self.approval_cursor = 0;
        if let Some(respond) = self.pending_approval.take() {
            let mut guard = respond.lock().await;
            if let Some(sender) = guard.take() {
                let _ = sender.send(approved);
            }
        }
        if !approved {
            if let Some(DisplayItem::ToolCall { status, .. }) = self.items.last_mut() {
                if *status == ToolStatus::WaitingApproval {
                    *status = ToolStatus::Denied;
                }
            }
        }
        self.mode = AppMode::Streaming;
    }

    async fn respond_user_question(&mut self, answer: String) {
        if let Some(respond) = self.pending_user_question.take() {
            let mut guard = respond.lock().await;
            if let Some(sender) = guard.take() {
                let _ = sender.send(answer.clone());
            }
        }
        if answer == "__cancelled__" {
            self.items.push(DisplayItem::Message {
                role: "system".to_string(),
                content: "Dismissed question.".to_string(),
            });
        } else {
            self.items.push(DisplayItem::Message {
                role: "system".to_string(),
                content: format!("Answered: {}", answer),
            });
        }
        self.mode = AppMode::Streaming;
    }

    fn open_user_question_custom_input(&mut self) {
        use crate::components::text_prompt::{TextPromptKind, TextPromptState};
        self.text_prompt = Some(TextPromptState::new(
            TextPromptKind::UserQuestionCustom,
            "Custom Answer",
            &["Type your response to the agent's question."],
            "Your answer...",
            false,
        ));
    }

    fn handle_model_tab(&mut self, _model_info_idx: &mut Option<usize>) {
        let cursor_model = self
            .selector
            .as_ref()
            .and_then(|s| s.items.get(s.cursor))
            .map(|item| item.value.clone())
            .unwrap_or_default();

        let registry = nyzhi_provider::ModelRegistry::new();
        let found = registry.find_any(&cursor_model);
        let model_info = found.map(|(_, m)| m);

        if model_info.map(|m| m.has_thinking()).unwrap_or(false) {
            let mi = model_info.cloned();
            self.selector = None;
            self.open_thinking_selector(mi.as_ref());
        } else {
            self.items.push(DisplayItem::Message {
                role: "system".to_string(),
                content: "This model does not support thinking levels.".to_string(),
            });
        }
    }

    fn open_external_editor(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        use std::io::Write;

        let tmp_path = std::env::temp_dir().join(format!("nyzhi_edit_{}.md", std::process::id()));
        std::fs::write(&tmp_path, &self.input)?;

        terminal::disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;

        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| "vi".to_string());

        let status = std::process::Command::new(&editor)
            .arg(&tmp_path)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status();

        terminal::enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        io::stdout().flush()?;
        // Force full redraw
        terminal.clear()?;

        match status {
            Ok(s) if s.success() => {
                let content = std::fs::read_to_string(&tmp_path).unwrap_or_default();
                let line_count = content.lines().count();
                self.input = content;
                self.cursor_pos = self.input.len();
                self.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: format!("Loaded {line_count} line(s) from editor"),
                });
            }
            Ok(s) => {
                self.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: format!("Editor exited with status: {s}"),
                });
            }
            Err(e) => {
                self.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: format!("Failed to open editor ({editor}): {e}"),
                });
            }
        }

        let _ = std::fs::remove_file(&tmp_path);
        Ok(())
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

fn truncate_display(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}

pub fn generate_diff(file: &str, old: Option<&str>, new: &str) -> DisplayItem {
    let old = old.unwrap_or("");
    let is_new_file = old.is_empty();

    let old_lines: Vec<&str> = if old.is_empty() {
        vec![]
    } else {
        old.lines().collect()
    };
    let new_lines: Vec<&str> = new.lines().collect();

    let mut hunks = Vec::new();
    let max_ctx = 3;

    let lcs = simple_diff_ranges(&old_lines, &new_lines);
    let edits = edits_from_lcs(&old_lines, &new_lines, &lcs);

    if edits.is_empty() {
        return DisplayItem::Diff {
            file: file.to_string(),
            hunks: vec![],
            is_new_file,
        };
    }

    let mut current_hunk_lines: Vec<DiffLine> = Vec::new();
    let mut hunk_old_start = 1usize;
    let mut hunk_new_start = 1usize;
    let mut hunk_old_count = 0usize;
    let mut hunk_new_count = 0usize;
    let mut last_change_idx: Option<usize> = None;

    for (i, edit) in edits.iter().enumerate() {
        let is_change = !matches!(edit, Edit::Equal(_));
        if is_change {
            if let Some(last) = last_change_idx {
                if i - last > max_ctx * 2 {
                    let header = format!(
                        "@@ -{},{} +{},{} @@",
                        hunk_old_start, hunk_old_count, hunk_new_start, hunk_new_count
                    );
                    hunks.push(DiffHunk {
                        header,
                        lines: std::mem::take(&mut current_hunk_lines),
                    });
                    hunk_old_start =
                        edits[..i].iter().filter(|e| matches!(e, Edit::Equal(_) | Edit::Delete(_))).count() + 1 - max_ctx.min(i);
                    hunk_new_start =
                        edits[..i].iter().filter(|e| matches!(e, Edit::Equal(_) | Edit::Insert(_))).count() + 1 - max_ctx.min(i);
                    hunk_old_count = 0;
                    hunk_new_count = 0;
                    let ctx_start = i.saturating_sub(max_ctx);
                    for e in &edits[ctx_start..i] {
                        if let Edit::Equal(line) = e {
                            current_hunk_lines.push(DiffLine {
                                kind: DiffLineKind::Context,
                                content: line.to_string(),
                            });
                            hunk_old_count += 1;
                            hunk_new_count += 1;
                        }
                    }
                }
            } else {
                let ctx_start = i.saturating_sub(max_ctx);
                let mut old_pos = edits[..ctx_start]
                    .iter()
                    .filter(|e| matches!(e, Edit::Equal(_) | Edit::Delete(_)))
                    .count();
                let mut new_pos = edits[..ctx_start]
                    .iter()
                    .filter(|e| matches!(e, Edit::Equal(_) | Edit::Insert(_)))
                    .count();
                hunk_old_start = old_pos + 1;
                hunk_new_start = new_pos + 1;
                for e in &edits[ctx_start..i] {
                    if let Edit::Equal(line) = e {
                        current_hunk_lines.push(DiffLine {
                            kind: DiffLineKind::Context,
                            content: line.to_string(),
                        });
                        hunk_old_count += 1;
                        hunk_new_count += 1;
                        old_pos += 1;
                        new_pos += 1;
                    }
                }
            }
            last_change_idx = Some(i);
        }

        if last_change_idx.is_some() {
            match edit {
                Edit::Equal(line) => {
                    current_hunk_lines.push(DiffLine {
                        kind: DiffLineKind::Context,
                        content: line.to_string(),
                    });
                    hunk_old_count += 1;
                    hunk_new_count += 1;
                }
                Edit::Delete(line) => {
                    current_hunk_lines.push(DiffLine {
                        kind: DiffLineKind::Removed,
                        content: line.to_string(),
                    });
                    hunk_old_count += 1;
                }
                Edit::Insert(line) => {
                    current_hunk_lines.push(DiffLine {
                        kind: DiffLineKind::Added,
                        content: line.to_string(),
                    });
                    hunk_new_count += 1;
                }
            }
        }
    }

    if !current_hunk_lines.is_empty() {
        while current_hunk_lines.len() > 1 {
            if matches!(
                current_hunk_lines.last(),
                Some(DiffLine { kind: DiffLineKind::Context, .. })
            ) && current_hunk_lines
                .iter()
                .rev()
                .take_while(|l| l.kind == DiffLineKind::Context)
                .count()
                > max_ctx
            {
                current_hunk_lines.pop();
                hunk_old_count -= 1;
                hunk_new_count -= 1;
            } else {
                break;
            }
        }
        let header = format!(
            "@@ -{},{} +{},{} @@",
            hunk_old_start, hunk_old_count, hunk_new_start, hunk_new_count
        );
        hunks.push(DiffHunk {
            header,
            lines: current_hunk_lines,
        });
    }

    DisplayItem::Diff {
        file: file.to_string(),
        hunks,
        is_new_file,
    }
}

enum Edit<'a> {
    Equal(&'a str),
    Delete(&'a str),
    Insert(&'a str),
}

fn simple_diff_ranges(old: &[&str], new: &[&str]) -> Vec<(usize, usize)> {
    let n = old.len();
    let m = new.len();
    if n == 0 || m == 0 {
        return vec![];
    }
    let mut dp = vec![vec![0u16; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            dp[i][j] = if old[i - 1] == new[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }
    let mut result = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 && j > 0 {
        if old[i - 1] == new[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

fn edits_from_lcs<'a>(old: &[&'a str], new: &[&'a str], lcs: &[(usize, usize)]) -> Vec<Edit<'a>> {
    let mut edits = Vec::new();
    let mut oi = 0usize;
    let mut ni = 0usize;

    for &(lo, ln) in lcs {
        while oi < lo {
            edits.push(Edit::Delete(old[oi]));
            oi += 1;
        }
        while ni < ln {
            edits.push(Edit::Insert(new[ni]));
            ni += 1;
        }
        edits.push(Edit::Equal(old[oi]));
        oi += 1;
        ni += 1;
    }
    while oi < old.len() {
        edits.push(Edit::Delete(old[oi]));
        oi += 1;
    }
    while ni < new.len() {
        edits.push(Edit::Insert(new[ni]));
        ni += 1;
    }
    edits
}
