use crossterm::event::{KeyCode, KeyEvent};
use nyzhi_core::agent::{AgentConfig, AgentEvent};
use nyzhi_core::conversation::Thread;
use nyzhi_core::tools::{ToolContext, ToolRegistry};
use nyzhi_provider::Provider;
use tokio::sync::broadcast;

use crate::app::{App, AppMode, DisplayItem};

#[allow(clippy::too_many_arguments)]
pub async fn handle_key(
    app: &mut App,
    key: KeyEvent,
    provider: &dyn Provider,
    thread: &mut Thread,
    agent_config: &AgentConfig,
    event_tx: &broadcast::Sender<AgentEvent>,
    registry: &ToolRegistry,
    tool_ctx: &ToolContext,
) {
    if matches!(app.mode, AppMode::Streaming | AppMode::AwaitingApproval) {
        return;
    }

    match key.code {
        KeyCode::Enter => {
            let input = app.input.trim().to_string();
            if input.is_empty() {
                return;
            }

            if input == "/quit" || input == "/exit" {
                app.should_quit = true;
                return;
            }

            if input == "/clear" {
                app.items.clear();
                thread.clear();
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            app.items.push(DisplayItem::Message {
                role: "user".to_string(),
                content: input.clone(),
            });

            app.input.clear();
            app.cursor_pos = 0;
            app.mode = AppMode::Streaming;
            app.status = "thinking...".to_string();

            let event_tx = event_tx.clone();
            if let Err(e) = nyzhi_core::agent::run_turn(
                provider,
                thread,
                &input,
                agent_config,
                &event_tx,
                registry,
                tool_ctx,
            )
            .await
            {
                let _ = event_tx.send(AgentEvent::Error(e.to_string()));
            }
        }
        KeyCode::Char(c) => {
            app.input.insert(app.cursor_pos, c);
            app.cursor_pos += 1;
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                app.input.remove(app.cursor_pos);
            }
        }
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < app.input.len() {
                app.cursor_pos += 1;
            }
        }
        KeyCode::Up => {
            app.scroll_offset = app.scroll_offset.saturating_add(1);
        }
        KeyCode::Down => {
            app.scroll_offset = app.scroll_offset.saturating_sub(1);
        }
        _ => {}
    }
}
