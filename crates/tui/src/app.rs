use std::io;
use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use nyzhi_core::agent::{AgentConfig, AgentEvent, SessionUsage};
use nyzhi_core::conversation::Thread;
use nyzhi_core::tools::{ToolContext, ToolRegistry};
use nyzhi_core::workspace::WorkspaceContext;
use nyzhi_provider::Provider;
use ratatui::prelude::*;
use tokio::sync::broadcast;

use crate::input::handle_key;
use crate::spinner::SpinnerState;
use crate::theme::Theme;
use crate::ui::draw;

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
    ToolCall {
        name: String,
        args_summary: String,
        output: Option<String>,
        status: ToolStatus,
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

pub struct App {
    pub mode: AppMode,
    pub input: String,
    pub cursor_pos: usize,
    pub items: Vec<DisplayItem>,
    pub current_stream: String,
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
        }
    }

    pub async fn run(
        &mut self,
        provider: &dyn Provider,
        registry: &ToolRegistry,
        config: &nyzhi_config::Config,
    ) -> Result<()> {
        terminal::enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        let (event_tx, mut event_rx) = broadcast::channel::<AgentEvent>(256);
        let mut thread = Thread::new();

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

        let agent_config = AgentConfig {
            system_prompt: nyzhi_core::prompt::build_system_prompt_with_vision(
                Some(&self.workspace),
                config.agent.custom_instructions.as_deref(),
                &mcp_tool_summaries,
                supports_vision,
            ),
            max_steps: config.agent.max_steps.unwrap_or(100),
            max_tokens: config.agent.max_tokens,
            ..AgentConfig::default()
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let change_tracker = std::sync::Arc::new(tokio::sync::Mutex::new(
            nyzhi_core::tools::change_tracker::ChangeTracker::new(),
        ));
        let tool_ctx = ToolContext {
            session_id: thread.id.clone(),
            cwd,
            project_root: self.workspace.project_root.clone(),
            depth: 0,
            event_tx: Some(event_tx.clone()),
            change_tracker: change_tracker.clone(),
        };

        loop {
            self.spinner.tick();

            terminal.draw(|frame| draw(frame, self, &self.theme, &self.spinner))?;

            if event::poll(std::time::Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.should_quit = true;
                    } else if key.code == KeyCode::Char('t')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.theme.toggle_mode();
                    } else if key.code == KeyCode::Char('a')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.theme.next_accent();
                    } else if key.code == KeyCode::Char('l')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.items.clear();
                        thread.clear();
                        self.input.clear();
                        self.cursor_pos = 0;
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
                    } else {
                        let mi = model_info_idx.map(|i| &provider.supported_models()[i]);
                        handle_key(
                            self,
                            key,
                            provider,
                            &mut thread,
                            &agent_config,
                            &event_tx,
                            registry,
                            &tool_ctx,
                            mi,
                            &mut model_info_idx,
                        )
                        .await;
                    }
                }
            }

            while let Ok(agent_event) = event_rx.try_recv() {
                match agent_event {
                    AgentEvent::TextDelta(text) => {
                        self.current_stream.push_str(&text);
                    }
                    AgentEvent::ToolCallStart { name, .. } => {
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
                    AgentEvent::ToolCallDone { name, output, .. } => {
                        if let Some(DisplayItem::ToolCall {
                            name: ref item_name,
                            output: ref mut item_output,
                            status,
                            ..
                        }) = self.items.last_mut()
                        {
                            if *item_name == name
                                && (*status == ToolStatus::Running
                                    || *status == ToolStatus::WaitingApproval)
                            {
                                *item_output = Some(truncate_display(&output, 500));
                                *status = ToolStatus::Completed;
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
                    AgentEvent::Usage(usage) => {
                        self.session_usage = usage;
                    }
                    AgentEvent::TurnComplete => {
                        if !self.current_stream.is_empty() {
                            self.items.push(DisplayItem::Message {
                                role: "assistant".to_string(),
                                content: std::mem::take(&mut self.current_stream),
                            });
                        }
                        self.mode = AppMode::Input;
                        if thread.message_count() > 0 {
                            let _ = nyzhi_core::session::save_session(
                                &thread,
                                &self.provider_name,
                                &self.model_name,
                            );
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

            if self.should_quit {
                break;
            }
        }

        terminal::disable_raw_mode()?;
        io::stdout().execute(LeaveAlternateScreen)?;
        Ok(())
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
}

fn truncate_display(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}
