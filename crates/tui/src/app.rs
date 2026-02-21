use std::io;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use nyzhi_core::agent::{AgentConfig, AgentEvent};
use nyzhi_core::conversation::Thread;
use nyzhi_provider::Provider;
use ratatui::prelude::*;
use tokio::sync::broadcast;

use crate::input::handle_key;
use crate::ui::draw;

pub enum AppMode {
    Input,
    Streaming,
}

pub struct App {
    pub mode: AppMode,
    pub input: String,
    pub cursor_pos: usize,
    pub messages: Vec<DisplayMessage>,
    pub current_stream: String,
    pub status: String,
    pub should_quit: bool,
    pub provider_name: String,
    pub model_name: String,
    pub scroll_offset: u16,
}

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: String,
    pub content: String,
}

impl App {
    pub fn new(provider_name: &str, model_name: &str) -> Self {
        Self {
            mode: AppMode::Input,
            input: String::new(),
            cursor_pos: 0,
            messages: Vec::new(),
            current_stream: String::new(),
            status: format!("{provider_name}/{model_name}"),
            should_quit: false,
            provider_name: provider_name.to_string(),
            model_name: model_name.to_string(),
            scroll_offset: 0,
        }
    }

    pub async fn run(
        &mut self,
        provider: &dyn Provider,
    ) -> Result<()> {
        terminal::enable_raw_mode()?;
        io::stdout().execute(EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(io::stdout());
        let mut terminal = Terminal::new(backend)?;

        let (event_tx, mut event_rx) = broadcast::channel::<AgentEvent>(256);
        let mut thread = Thread::new();
        let agent_config = AgentConfig::default();

        loop {
            terminal.draw(|frame| draw(frame, self))?;

            if event::poll(std::time::Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        self.should_quit = true;
                    } else {
                        handle_key(self, key, provider, &mut thread, &agent_config, &event_tx)
                            .await;
                    }
                }
            }

            while let Ok(agent_event) = event_rx.try_recv() {
                match agent_event {
                    AgentEvent::TextDelta(text) => {
                        self.current_stream.push_str(&text);
                    }
                    AgentEvent::TurnComplete => {
                        if !self.current_stream.is_empty() {
                            self.messages.push(DisplayMessage {
                                role: "assistant".to_string(),
                                content: std::mem::take(&mut self.current_stream),
                            });
                        }
                        self.mode = AppMode::Input;
                        self.status = format!("{}/{}", self.provider_name, self.model_name);
                    }
                    AgentEvent::Error(e) => {
                        self.status = format!("Error: {e}");
                        self.mode = AppMode::Input;
                    }
                    _ => {}
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
}
