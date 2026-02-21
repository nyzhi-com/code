use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nyzhi_core::agent::{AgentConfig, AgentEvent};
use nyzhi_core::conversation::Thread;
use nyzhi_core::tools::{ToolContext, ToolRegistry};
use nyzhi_provider::{ModelInfo, Provider};
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
    model_info: Option<&ModelInfo>,
) {
    if matches!(app.mode, AppMode::Streaming | AppMode::AwaitingApproval) {
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('u') {
        app.input.clear();
        app.cursor_pos = 0;
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

            if input == "/theme" {
                app.theme.toggle_mode();
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/accent" {
                app.theme.next_accent();
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/compact" {
                if model_info.is_some() {
                    let est = thread.estimated_tokens(&agent_config.system_prompt);
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!(
                            "Compacting... (~{est} tokens, {} messages)",
                            thread.message_count()
                        ),
                    });
                    app.mode = AppMode::Streaming;

                    let summary_prompt =
                        nyzhi_core::context::build_compaction_prompt(thread.messages());
                    let summary_request = nyzhi_provider::ChatRequest {
                        model: String::new(),
                        messages: vec![nyzhi_provider::Message {
                            role: nyzhi_provider::Role::User,
                            content: nyzhi_provider::MessageContent::Text(summary_prompt),
                        }],
                        tools: vec![],
                        max_tokens: Some(2048),
                        temperature: Some(0.0),
                        system: None,
                        stream: false,
                    };
                    match provider.chat(&summary_request).await {
                        Ok(resp) => {
                            let summary = resp.message.content.as_text().to_string();
                            thread.compact(&summary, 4);
                            let new_est =
                                thread.estimated_tokens(&agent_config.system_prompt);
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!(
                                    "Compacted to ~{new_est} tokens ({} messages)",
                                    thread.message_count()
                                ),
                            });
                        }
                        Err(e) => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Compaction failed: {e}"),
                            });
                        }
                    }
                    app.mode = AppMode::Input;
                } else {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No model info available for compaction".to_string(),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/sessions" {
                match nyzhi_core::session::list_sessions() {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: "No saved sessions.".to_string(),
                            });
                        } else {
                            let mut lines = vec!["Saved sessions:".to_string()];
                            for (i, s) in sessions.iter().take(20).enumerate() {
                                lines.push(format!(
                                    "  {}. [{}] {} ({} msgs, {})",
                                    i + 1,
                                    &s.id[..8],
                                    s.title,
                                    s.message_count,
                                    s.updated_at.format("%Y-%m-%d %H:%M"),
                                ));
                            }
                            lines.push(String::new());
                            lines.push("Use /resume <id-prefix> to restore a session.".to_string());
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: lines.join("\n"),
                            });
                        }
                    }
                    Err(e) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Error listing sessions: {e}"),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if let Some(id_prefix) = input.strip_prefix("/resume ") {
                let id_prefix = id_prefix.trim();
                match nyzhi_core::session::list_sessions() {
                    Ok(sessions) => {
                        let matched: Vec<_> = sessions
                            .iter()
                            .filter(|s| s.id.starts_with(id_prefix))
                            .collect();
                        match matched.len() {
                            0 => {
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!("No session matching '{id_prefix}'"),
                                });
                            }
                            1 => {
                                let meta = matched[0];
                                match nyzhi_core::session::load_session(&meta.id) {
                                    Ok((loaded_thread, loaded_meta)) => {
                                        *thread = loaded_thread;
                                        app.items.clear();
                                        app.session_usage =
                                            nyzhi_core::agent::SessionUsage::default();

                                        for msg in thread.messages() {
                                            let role = match msg.role {
                                                nyzhi_provider::Role::User => "user",
                                                nyzhi_provider::Role::Assistant => "assistant",
                                                _ => "system",
                                            };
                                            let text = msg.content.as_text();
                                            if !text.is_empty() {
                                                app.items.push(DisplayItem::Message {
                                                    role: role.to_string(),
                                                    content: text.to_string(),
                                                });
                                            }
                                        }
                                        app.items.push(DisplayItem::Message {
                                            role: "system".to_string(),
                                            content: format!(
                                                "Resumed session: {} ({} messages)",
                                                loaded_meta.title,
                                                loaded_meta.message_count,
                                            ),
                                        });
                                    }
                                    Err(e) => {
                                        app.items.push(DisplayItem::Message {
                                            role: "system".to_string(),
                                            content: format!("Error loading session: {e}"),
                                        });
                                    }
                                }
                            }
                            n => {
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!(
                                        "Ambiguous: {n} sessions match '{id_prefix}'. Be more specific."
                                    ),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Error listing sessions: {e}"),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/help" {
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: [
                        "Commands:",
                        "  /help        Show this help",
                        "  /clear       Clear the session",
                        "  /compact     Compress conversation history",
                        "  /sessions    List saved sessions",
                        "  /resume <id> Restore a saved session",
                        "  /theme       Toggle light/dark theme",
                        "  /accent      Cycle accent color",
                        "  /quit        Exit nyzhi",
                        "",
                        "Shortcuts:",
                        "  ctrl+t       Toggle theme",
                        "  ctrl+a       Cycle accent",
                        "  ctrl+l       Clear session",
                        "  ctrl+u       Clear input line",
                        "  ctrl+c       Exit",
                    ]
                    .join("\n"),
                });
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

            let event_tx = event_tx.clone();
            if let Err(e) = nyzhi_core::agent::run_turn(
                provider,
                thread,
                &input,
                agent_config,
                &event_tx,
                registry,
                tool_ctx,
                model_info,
                &mut app.session_usage,
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
        KeyCode::Home => {
            app.cursor_pos = 0;
        }
        KeyCode::End => {
            app.cursor_pos = app.input.len();
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
