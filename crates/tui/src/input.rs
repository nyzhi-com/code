use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nyzhi_core::agent::{AgentConfig, AgentEvent};
use nyzhi_core::conversation::Thread;
use nyzhi_core::tools::{ToolContext, ToolRegistry};
use nyzhi_provider::{ContentPart, MessageContent, ModelInfo, Provider};
use tokio::sync::broadcast;

use crate::app::{App, AppMode, DisplayItem, PendingImage};

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
    model_info_idx: &mut Option<usize>,
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
                                            let mut text = msg.content.as_text().to_string();
                                            if msg.content.has_images() {
                                                text.push_str("\n[image attached]");
                                            }
                                            if !text.is_empty() {
                                                app.items.push(DisplayItem::Message {
                                                    role: role.to_string(),
                                                    content: text,
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

            if input == "/init" {
                match nyzhi_core::workspace::scaffold_nyzhi_dir(&app.workspace.project_root) {
                    Ok(created) => {
                        if created.is_empty() {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!(
                                    ".nyzhi/ already exists in {}",
                                    app.workspace.project_root.display()
                                ),
                            });
                        } else {
                            let mut lines = vec![format!(
                                "Initialized .nyzhi/ in {}",
                                app.workspace.project_root.display()
                            )];
                            for p in &created {
                                lines.push(format!("  created {}", p.display()));
                            }
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: lines.join("\n"),
                            });
                            app.workspace.has_nyzhi_config = true;
                        }
                    }
                    Err(e) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Failed to initialize: {e}"),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/mcp" {
                if let Some(mgr) = &app.mcp_manager {
                    let servers = mgr.server_info_list().await;
                    if servers.is_empty() {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "No MCP servers connected.".to_string(),
                        });
                    } else {
                        let mut lines = vec![format!("MCP servers ({}):", servers.len())];
                        for s in &servers {
                            lines.push(format!(
                                "  {}  ({} tools: {})",
                                s.name,
                                s.tool_count,
                                s.tool_names.join(", "),
                            ));
                        }
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: lines.join("\n"),
                        });
                    }
                } else {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No MCP servers configured.".to_string(),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/model" {
                let models = provider.supported_models();
                let mut lines = vec!["Available models:".to_string()];
                for m in models {
                    let marker = if m.id == app.model_name { " *" } else { "" };
                    lines.push(format!(
                        "  {} ({}){marker}",
                        m.id, m.name,
                    ));
                }
                lines.push(String::new());
                lines.push("Use /model <id> to switch.".to_string());
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: lines.join("\n"),
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if let Some(new_model) = input.strip_prefix("/model ") {
                let new_model = new_model.trim();
                if let Some(idx) = provider
                    .supported_models()
                    .iter()
                    .position(|m| m.id == new_model)
                {
                    let mi = &provider.supported_models()[idx];
                    app.model_name = mi.id.to_string();
                    *model_info_idx = Some(idx);
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Switched to {} ({})", mi.id, mi.name),
                    });
                } else {
                    let available: Vec<&str> =
                        provider.supported_models().iter().map(|m| m.id).collect();
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!(
                            "Unknown model '{}'. Available: {}",
                            new_model,
                            available.join(", ")
                        ),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if let Some(path_str) = input.strip_prefix("/image ") {
                let path_str = path_str.trim();
                match load_image(path_str) {
                    Ok(img) => {
                        let kb = img.size_bytes / 1024;
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Image attached: {} ({} KB). Type your prompt and press Enter.",
                                img.filename, kb
                            ),
                        });
                        app.pending_images.push(img);
                    }
                    Err(e) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Failed to load image: {e}"),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/login" {
                let providers = ["openai", "gemini"];
                let mut lines = vec!["Auth status:".to_string()];
                for prov in &providers {
                    let has_token = nyzhi_auth::token_store::load_token(prov)
                        .ok()
                        .flatten()
                        .is_some();
                    let status = if has_token { "logged in" } else { "not logged in" };
                    let marker = if has_token { "✓" } else { "✗" };
                    lines.push(format!("  {marker} {prov}: {status}"));
                }
                lines.push(String::new());
                lines.push(
                    "Use `nyzhi login <provider>` in your terminal to log in via OAuth."
                        .to_string(),
                );
                lines.push("Use `nyzhi logout <provider>` to remove stored tokens.".to_string());
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: lines.join("\n"),
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/help" {
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: [
                        "Commands:",
                        "  /help           Show this help",
                        "  /model          List available models",
                        "  /model <id>     Switch to a different model",
                        "  /image <path>   Attach an image for the next prompt",
                        "  /login          Show OAuth login status",
                        "  /init           Initialize .nyzhi/ project config",
                        "  /mcp            List connected MCP servers",
                        "  /clear          Clear the session",
                        "  /compact        Compress conversation history",
                        "  /sessions       List saved sessions",
                        "  /resume <id>    Restore a saved session",
                        "  /theme          Toggle light/dark theme",
                        "  /accent         Cycle accent color",
                        "  /quit           Exit nyzhi",
                        "",
                        "Agent tools:",
                        "  git_status, git_diff, git_log, git_show, git_branch (read-only)",
                        "  git_commit, git_checkout (require approval)",
                        "  task (delegate sub-tasks to a child agent)",
                        "",
                        "Auth:",
                        "  nyzhi login <provider>    Log in via OAuth (gemini, openai)",
                        "  nyzhi logout <provider>   Remove stored OAuth token",
                        "  nyzhi whoami              Show auth status for all providers",
                        "",
                        "Shortcuts:",
                        "  ctrl+t          Toggle theme",
                        "  ctrl+a          Cycle accent",
                        "  ctrl+l          Clear session",
                        "  ctrl+u          Clear input line",
                        "  ctrl+c          Exit",
                    ]
                    .join("\n"),
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            let has_images = !app.pending_images.is_empty();
            let mut display_content = input.clone();
            if has_images {
                let names: Vec<&str> = app
                    .pending_images
                    .iter()
                    .map(|i| i.filename.as_str())
                    .collect();
                display_content = format!("{input}\n[images: {}]", names.join(", "));
            }

            app.items.push(DisplayItem::Message {
                role: "user".to_string(),
                content: display_content,
            });

            app.input.clear();
            app.cursor_pos = 0;
            app.mode = AppMode::Streaming;

            let event_tx = event_tx.clone();
            let result = if has_images {
                let images = std::mem::take(&mut app.pending_images);
                let content = build_multimodal_content(&input, &images);
                nyzhi_core::agent::run_turn_with_content(
                    provider,
                    thread,
                    content,
                    agent_config,
                    &event_tx,
                    registry,
                    tool_ctx,
                    model_info,
                    &mut app.session_usage,
                )
                .await
            } else {
                nyzhi_core::agent::run_turn(
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
            };
            if let Err(e) = result {
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

fn load_image(path_str: &str) -> anyhow::Result<PendingImage> {
    use base64::Engine;

    let path = std::path::Path::new(path_str);
    if !path.exists() {
        anyhow::bail!("File not found: {path_str}");
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let media_type = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => anyhow::bail!("Unsupported image format: .{ext} (use png, jpg, gif, or webp)"),
    };

    let bytes = std::fs::read(path)?;
    let size_bytes = bytes.len();
    let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image")
        .to_string();

    Ok(PendingImage {
        filename,
        media_type: media_type.to_string(),
        data,
        size_bytes,
    })
}

fn build_multimodal_content(text: &str, images: &[PendingImage]) -> MessageContent {
    let mut parts = Vec::new();
    for img in images {
        parts.push(ContentPart::Image {
            media_type: img.media_type.clone(),
            data: img.data.clone(),
        });
    }
    parts.push(ContentPart::Text {
        text: text.to_string(),
    });
    MessageContent::Parts(parts)
}
