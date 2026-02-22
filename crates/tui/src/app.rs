use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use nyzhi_core::agent::{AgentConfig, AgentEvent, SessionUsage};
use nyzhi_core::conversation::Thread;
use nyzhi_core::tools::{ToolContext, ToolRegistry};
use nyzhi_core::workspace::WorkspaceContext;
use nyzhi_provider::{MessageContent, Provider};
use ratatui::prelude::*;
use tokio::sync::broadcast;

use crate::input::handle_key;
use crate::spinner::SpinnerState;
use crate::theme::Theme;
use crate::ui::draw;

#[derive(PartialEq)]
pub enum AppMode {
    Input,
    Streaming,
    AwaitingApproval,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Running,
    WaitingApproval,
    Completed,
    Denied,
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
    pub scroll_offset: u16,
    pub theme: Theme,
    pub spinner: SpinnerState,
    pub session_usage: SessionUsage,
    pub workspace: WorkspaceContext,
    pub mcp_manager: Option<std::sync::Arc<nyzhi_core::mcp::McpManager>>,
    pub pending_approval:
        Option<std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>>,
    pub pending_images: Vec<PendingImage>,
    pub trust_mode: nyzhi_config::TrustMode,
    pub selector: Option<crate::components::selector::SelectorState>,
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
    pub turn_request: Option<TurnRequest>,
    pub foreground_task: Option<ForegroundTask>,
    pub background_tasks: Vec<BackgroundTask>,
    pub background_next_id: usize,
    pub ctrl_f_pending: bool,
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
            scroll_offset: 0,
            theme: Theme::from_config(config),
            spinner: SpinnerState::new(),
            session_usage: SessionUsage::default(),
            workspace,
            mcp_manager: None,
            pending_approval: None,
            pending_images: Vec::new(),
            trust_mode: nyzhi_config::TrustMode::Off,
            selector: None,
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
            turn_request: None,
            foreground_task: None,
            background_tasks: Vec::new(),
            background_next_id: 1,
            ctrl_f_pending: false,
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

    pub async fn run(
        &mut self,
        provider: std::sync::Arc<dyn Provider>,
        mut registry: ToolRegistry,
        config: &nyzhi_config::Config,
    ) -> Result<()> {
        self.history.load();
        self.custom_commands = nyzhi_core::commands::load_all_commands(
            &self.workspace.project_root,
            &config.agent.commands,
        );

        terminal::enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;
        io::stdout().execute(EnableBracketedPaste)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        let (event_tx, mut event_rx) = broadcast::channel::<AgentEvent>(256);
        let mut thread: Option<Thread> = Some(if let Some((loaded_thread, loaded_meta)) = self.initial_session.take() {
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
        });

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

        let mut model_info_idx = provider
            .supported_models()
            .iter()
            .position(|m| m.id == self.model_name)
            .or(if provider.supported_models().is_empty() {
                None
            } else {
                Some(0)
            });

        let supports_vision = model_info_idx
            .map(|i| provider.supported_models()[i].supports_vision)
            .unwrap_or(false);

        let skills = nyzhi_core::skills::load_skills(&self.workspace.project_root)
            .unwrap_or_default();
        let skills_text = nyzhi_core::skills::format_skills_for_prompt(&skills);

        let mut agent_config = AgentConfig {
            system_prompt: nyzhi_core::prompt::build_system_prompt_with_skills(
                Some(&self.workspace),
                config.agent.custom_instructions.as_deref(),
                &mcp_tool_summaries,
                supports_vision,
                &skills_text,
            ),
            max_steps: config.agent.max_steps.unwrap_or(100),
            max_tokens: config.agent.max_tokens,
            trust: config.agent.trust.clone(),
            retry: config.agent.retry.clone(),
            routing: config.agent.routing.clone(),
            auto_compact_threshold: config.agent.auto_compact_threshold,
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
        let tool_ctx = ToolContext {
            session_id: thread.as_ref().unwrap().id.clone(),
            cwd,
            project_root: self.workspace.project_root.clone(),
            depth: 0,
            event_tx: Some(event_tx.clone()),
            change_tracker: change_tracker.clone(),
            allowed_tool_names: None,
        };

        let agent_registry = std::sync::Arc::new(nyzhi_core::tools::default_registry().registry);
        let agent_manager = std::sync::Arc::new(nyzhi_core::agent_manager::AgentManager::new(
            provider.clone(),
            agent_registry,
            event_tx.clone(),
            config.agent.agents.max_threads,
            config.agent.agents.max_depth,
        ));

        let user_agent_roles =
            nyzhi_core::agent_roles::convert_user_roles(&config.agent.agents.roles);

        registry.register(Box::new(
            nyzhi_core::tools::spawn_agent::SpawnAgentTool::with_user_roles(
                agent_manager.clone(),
                user_agent_roles,
            ),
        ));
        registry.register(Box::new(
            nyzhi_core::tools::send_input::SendInputTool::new(agent_manager.clone()),
        ));
        registry.register(Box::new(
            nyzhi_core::tools::wait_tool::WaitTool::new(agent_manager.clone()),
        ));
        registry.register(Box::new(
            nyzhi_core::tools::close_agent::CloseAgentTool::new(agent_manager.clone()),
        ));
        registry.register(Box::new(
            nyzhi_core::tools::resume_agent::ResumeAgentTool::new(agent_manager.clone()),
        ));

        let registry = Arc::new(registry);

        loop {
            self.spinner.tick();

            terminal.draw(|frame| draw(frame, self, &self.theme, &self.spinner))?;

            if event::poll(std::time::Duration::from_millis(16))? {
                match event::read()? {
                Event::Paste(text) => {
                    if matches!(self.mode, AppMode::Input) {
                        self.input.insert_str(self.cursor_pos, &text);
                        self.cursor_pos += text.len();
                    }
                }
                Event::Key(key) => {
                    if self.selector.is_some() {
                        self.handle_selector_key(key, &mut model_info_idx);
                    } else if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.should_quit = true;
                    } else if key.code == KeyCode::Char('t')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.open_theme_selector();
                    } else if key.code == KeyCode::Char('a')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.open_accent_selector();
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
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                self.respond_approval(true).await;
                            }
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                self.respond_approval(false).await;
                            }
                            _ => {}
                        }
                    } else if let Some(t) = thread.as_mut() {
                        if key.code != KeyCode::Char('f')
                            || !key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            self.ctrl_f_pending = false;
                        }
                        let mi = model_info_idx.map(|i| &provider.supported_models()[i]);
                        handle_key(
                            self,
                            key,
                            &*provider,
                            t,
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
                _ => {}
                }
            }

            if self.wants_editor {
                self.wants_editor = false;
                Self::open_external_editor(self, &mut terminal)?;
            }

            // --- Spawn turn from request set by handle_key ---
            if let Some(req) = self.turn_request.take() {
                let mi_c = model_info_idx.map(|i| provider.supported_models()[i].clone());
                if req.is_background {
                    let bg_thread = thread.as_ref().unwrap().clone();
                    let bg_usage = self.session_usage.clone();
                    let (bg_event_tx, _) = broadcast::channel::<AgentEvent>(256);
                    let provider_c = provider.clone();
                    let registry_c = registry.clone();
                    let config_c = agent_config.clone();
                    let tool_ctx_c = tool_ctx.clone();
                    let join_handle = tokio::spawn(async move {
                        let mut t = bg_thread;
                        let mut u = bg_usage;
                        let result = if let Some(content) = req.content {
                            nyzhi_core::agent::run_turn_with_content(
                                &*provider_c, &mut t, content, &config_c,
                                &bg_event_tx, &registry_c, &tool_ctx_c,
                                mi_c.as_ref(), &mut u,
                            ).await
                        } else {
                            nyzhi_core::agent::run_turn(
                                &*provider_c, &mut t, &req.input, &config_c,
                                &bg_event_tx, &registry_c, &tool_ctx_c,
                                mi_c.as_ref(), &mut u,
                            ).await
                        };
                        TurnResult { thread: t, session_usage: u, result }
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
                    let fg_thread = thread.take().unwrap();
                    let snapshot = fg_thread.clone();
                    let fg_usage = self.session_usage.clone();
                    let provider_c = provider.clone();
                    let registry_c = registry.clone();
                    let config_c = agent_config.clone();
                    let event_tx_c = event_tx.clone();
                    let tool_ctx_c = tool_ctx.clone();
                    let join_handle = tokio::spawn(async move {
                        let mut t = fg_thread;
                        let mut u = fg_usage;
                        let result = if let Some(content) = req.content {
                            nyzhi_core::agent::run_turn_with_content(
                                &*provider_c, &mut t, content, &config_c,
                                &event_tx_c, &registry_c, &tool_ctx_c,
                                mi_c.as_ref(), &mut u,
                            ).await
                        } else {
                            nyzhi_core::agent::run_turn(
                                &*provider_c, &mut t, &req.input, &config_c,
                                &event_tx_c, &registry_c, &tool_ctx_c,
                                mi_c.as_ref(), &mut u,
                            ).await
                        };
                        TurnResult { thread: t, session_usage: u, result }
                    });
                    self.foreground_task = Some(ForegroundTask {
                        join_handle,
                        thread_snapshot: snapshot,
                        label: req.label,
                    });
                }
            }

            // --- Foreground task completion ---
            if self.foreground_task.as_ref().is_some_and(|f| f.join_handle.is_finished()) {
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
                        let last_msg = result.thread.messages().last().map(|m| {
                            let text = m.content.as_text().to_string();
                            if text.len() > 500 {
                                format!("{}...", &text[..500])
                            } else {
                                text
                            }
                        }).unwrap_or_default();
                        let status = if result.result.is_ok() { "completed" } else { "failed" };
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Background task #{} {status} ({:.1}s): {}\n{}",
                                bg.id, elapsed.as_secs_f64(), bg.label,
                                if last_msg.is_empty() { "(no output)".to_string() } else { last_msg },
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
                            self.items.push(DisplayItem::Thinking(
                                std::mem::take(&mut self.thinking_stream),
                            ));
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
                        const FILE_TOOLS: &[&str] = &[
                            "edit", "write", "delete_file", "move_file", "copy_file",
                        ];
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
                                        ).await;
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
                        self.mode = AppMode::AwaitingApproval;
                        let _ = args_summary;
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
                    AgentEvent::AutoCompacting { estimated_tokens, context_window } => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Auto-compacting context ({estimated_tokens} tokens / {context_window} window)"
                            ),
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
                    AgentEvent::SubAgentStatusChanged { nickname, status, .. } => {
                        self.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Agent {nickname}: {status}"),
                        });
                    }
                    AgentEvent::SubAgentCompleted { nickname, final_message, .. } => {
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
                    AgentEvent::Usage(usage) => {
                        self.session_usage = usage;
                    }
                    AgentEvent::TurnComplete => {
                        if !self.thinking_stream.is_empty() {
                            self.items.push(DisplayItem::Thinking(
                                std::mem::take(&mut self.thinking_stream),
                            ));
                        }
                        if !self.current_stream.is_empty() {
                            self.items.push(DisplayItem::Message {
                                role: "assistant".to_string(),
                                content: std::mem::take(&mut self.current_stream),
                            });
                        }
                        let turn_elapsed = self.turn_start.map(|t| t.elapsed());
                        self.stream_start = None;
                        self.stream_token_count = 0;
                        self.turn_start = None;
                        self.mode = AppMode::Input;

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
                        if let Some(t) = thread.as_ref() {
                            if t.message_count() > 0 {
                                let _ = nyzhi_core::session::save_session(
                                    t,
                                    &self.provider_name,
                                    &self.model_name,
                                );
                            }
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
        self.history.save();

        io::stdout().execute(DisableBracketedPaste)?;
        terminal::disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }

    fn handle_selector_key(&mut self, key: crossterm::event::KeyEvent, model_info_idx: &mut Option<usize>) {
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
                        let idx = self.selector.as_ref().unwrap().cursor;
                        *model_info_idx = Some(idx);
                        self.model_name = value;
                    }
                }
                self.selector = None;
            }
            SelectorAction::Cancel => {
                self.selector = None;
            }
            SelectorAction::None => {}
        }
        let _ = model_info_idx;
    }

    pub fn open_theme_selector(&mut self) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};
        use crate::theme::ThemePreset;

        let items: Vec<SelectorItem> = ThemePreset::ALL
            .iter()
            .map(|p| SelectorItem {
                label: p.display_name().to_string(),
                value: p.name().to_string(),
                preview_color: Some(p.bg_page_color()),
            })
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
            .map(|a| SelectorItem {
                label: capitalize(a.name()),
                value: a.name().to_string(),
                preview_color: Some(a.color_preview(self.theme.mode)),
            })
            .collect();
        self.selector = Some(SelectorState::new(
            SelectorKind::Accent,
            "Accent Color",
            items,
            self.theme.accent_type.name(),
        ));
    }

    pub fn open_model_selector(&mut self, models: &[nyzhi_provider::ModelInfo]) {
        use crate::components::selector::{SelectorItem, SelectorKind, SelectorState};

        let items: Vec<SelectorItem> = models
            .iter()
            .map(|m| SelectorItem {
                label: m.id.to_string(),
                value: m.id.to_string(),
                preview_color: None,
            })
            .collect();
        self.selector = Some(SelectorState::new(
            SelectorKind::Model,
            "Model",
            items,
            &self.model_name,
        ));
    }

    async fn respond_approval(&mut self, approved: bool) {
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

    fn open_external_editor(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        use std::io::Write;

        let tmp_path = std::env::temp_dir().join(format!(
            "nyzhi_edit_{}.md",
            std::process::id()
        ));
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
