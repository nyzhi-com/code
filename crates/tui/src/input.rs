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
    agent_config: &mut AgentConfig,
    event_tx: &broadcast::Sender<AgentEvent>,
    registry: &ToolRegistry,
    tool_ctx: &ToolContext,
    model_info: Option<&ModelInfo>,
    model_info_idx: &mut Option<usize>,
) {
    if matches!(app.mode, AppMode::Streaming | AppMode::AwaitingApproval) {
        return;
    }

    if app.history_search.is_some() {
        handle_history_search_key(app, key);
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
        app.history_search = Some(crate::history::HistorySearch::new());
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('u') {
        app.input.clear();
        app.cursor_pos = 0;
        return;
    }

    match key.code {
        KeyCode::Tab => {
            if let Some(ref mut state) = app.completion {
                state.cycle_forward();
            } else {
                try_open_completion(app, &tool_ctx.cwd);
            }
        }
        KeyCode::BackTab => {
            if let Some(ref mut state) = app.completion {
                state.cycle_backward();
            }
        }
        KeyCode::Esc => {
            if app.completion.is_some() {
                app.completion = None;
            }
        }
        KeyCode::Enter => {
            if app.completion.is_some() {
                accept_completion(app, &tool_ctx.cwd);
                return;
            }

            if key.modifiers.contains(KeyModifiers::ALT)
                || key.modifiers.contains(KeyModifiers::SHIFT)
            {
                app.input.insert(app.cursor_pos, '\n');
                app.cursor_pos += 1;
                return;
            }

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
                app.open_theme_selector();
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/accent" {
                app.open_accent_selector();
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
                if models.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No models available.".to_string(),
                    });
                } else {
                    app.open_model_selector(models);
                }
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

            if input == "/undo" {
                let mut tracker = tool_ctx.change_tracker.lock().await;
                match tracker.undo_last().await {
                    Ok(Some(change)) => {
                        let action = if change.original.is_some() {
                            "Reverted"
                        } else {
                            "Removed (was newly created)"
                        };
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "{action}: {} ({})",
                                change.path.display(),
                                change.tool_name,
                            ),
                        });
                    }
                    Ok(None) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "No changes to undo.".to_string(),
                        });
                    }
                    Err(e) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Undo failed: {e}"),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/undo all" {
                let mut tracker = tool_ctx.change_tracker.lock().await;
                match tracker.undo_all().await {
                    Ok(reverted) => {
                        if reverted.is_empty() {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: "No changes to undo.".to_string(),
                            });
                        } else {
                            let mut msg = format!("Reverted {} change(s):", reverted.len());
                            for c in &reverted {
                                msg.push_str(&format!(
                                    "\n  {} ({})",
                                    c.path.display(),
                                    c.tool_name,
                                ));
                            }
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: msg,
                            });
                        }
                    }
                    Err(e) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Undo all failed: {e}"),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/changes" {
                let tracker = tool_ctx.change_tracker.lock().await;
                if tracker.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No file changes in this session.".to_string(),
                    });
                } else {
                    let mut msg = format!("{} change(s):", tracker.len());
                    for c in tracker.changes() {
                        let kind = if c.original.is_some() {
                            "modified"
                        } else {
                            "created"
                        };
                        msg.push_str(&format!(
                            "\n  {} ({}, {}, {})",
                            c.path.display(),
                            c.tool_name,
                            kind,
                            c.timestamp.format("%H:%M:%S"),
                        ));
                    }
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: msg,
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/trust" {
                let mode = &agent_config.trust.mode;
                let tools = &agent_config.trust.allow_tools;
                let paths = &agent_config.trust.allow_paths;
                let mut msg = format!("Trust mode: {mode}");
                if !tools.is_empty() {
                    msg.push_str(&format!("\nAllowed tools: {}", tools.join(", ")));
                }
                if !paths.is_empty() {
                    msg.push_str(&format!("\nAllowed paths: {}", paths.join(", ")));
                }
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: msg,
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/trust full" || input == "/trust limited" || input == "/trust off" {
                let mode_str = input.strip_prefix("/trust ").unwrap();
                match mode_str.parse::<nyzhi_config::TrustMode>() {
                    Ok(mode) => {
                        agent_config.trust.mode = mode.clone();
                        app.trust_mode = mode;
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Trust mode set to: {mode_str}"),
                        });
                    }
                    Err(e) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Invalid trust mode: {e}"),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/editor" {
                app.wants_editor = true;
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
                        "  /theme          Choose theme (dark/light)",
                        "  /accent         Choose accent color",
                        "  /trust          Show current trust mode",
                        "  /trust <mode>   Set trust mode (off, limited, full)",
                        "  /editor         Open $EDITOR for multi-line input",
                        "  /undo           Undo the last file change",
                        "  /undo all       Undo all file changes in this session",
                        "  /changes        List all file changes in this session",
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
                        "Context:",
                        "  @path           Attach file or directory contents to your prompt",
                        "                  e.g. explain @src/main.rs or list @src/",
                        "",
                        "Input:",
                        "  tab             Auto-complete commands, @paths, file paths",
                        "  shift+tab       Cycle completion backward",
                        "  alt+enter       Insert newline (multi-line mode)",
                        "  shift+enter     Insert newline (kitty protocol)",
                        "  enter           Submit message / accept completion",
                        "  up/down         Navigate input history (single-line)",
                        "  ctrl+r          Reverse search history",
                        "",
                        "Shortcuts:",
                        "  ctrl+t          Open theme picker",
                        "  ctrl+a          Open accent picker",
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

            app.history.push(input.clone());

            let mentions = nyzhi_core::context_files::parse_mentions(&input);
            let context_files = if mentions.is_empty() {
                Vec::new()
            } else {
                nyzhi_core::context_files::resolve_context_files(
                    &mentions,
                    &tool_ctx.project_root,
                    &tool_ctx.cwd,
                )
            };

            let has_images = !app.pending_images.is_empty();
            let has_context = !context_files.is_empty();
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

            if has_context {
                let summary =
                    nyzhi_core::context_files::format_attachment_summary(&context_files);
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: summary,
                });
            }

            let agent_input = if has_context {
                nyzhi_core::context_files::build_context_message(&input, &context_files)
            } else {
                input.clone()
            };

            app.input.clear();
            app.cursor_pos = 0;
            app.mode = AppMode::Streaming;

            let event_tx = event_tx.clone();
            let result = if has_images {
                let images = std::mem::take(&mut app.pending_images);
                let content = build_multimodal_content(&agent_input, &images);
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
                    &agent_input,
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
            app.history.reset_cursor();
            app.completion = None;
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                app.input.remove(app.cursor_pos);
            }
            app.completion = None;
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
            let before = &app.input[..app.cursor_pos];
            let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
            app.cursor_pos = line_start;
        }
        KeyCode::End => {
            let after = &app.input[app.cursor_pos..];
            let line_end = after
                .find('\n')
                .map(|i| app.cursor_pos + i)
                .unwrap_or(app.input.len());
            app.cursor_pos = line_end;
        }
        KeyCode::Up => {
            if app.input.contains('\n') {
                move_cursor_up(app);
            } else if let Some(entry) = app.history.navigate_up(&app.input) {
                app.input = entry;
                app.cursor_pos = app.input.len();
            } else {
                app.scroll_offset = app.scroll_offset.saturating_add(1);
            }
        }
        KeyCode::Down => {
            if app.input.contains('\n') {
                move_cursor_down(app);
            } else if let Some(entry) = app.history.navigate_down() {
                app.input = entry;
                app.cursor_pos = app.input.len();
            } else {
                app.scroll_offset = app.scroll_offset.saturating_sub(1);
            }
        }
        _ => {}
    }
}

fn handle_history_search_key(app: &mut App, key: KeyEvent) {
    let search = app.history_search.as_mut().unwrap();
    match key.code {
        KeyCode::Esc => {
            app.history_search = None;
        }
        KeyCode::Enter => {
            let query = search.query.clone();
            let selected = search.selected;
            let matches = app.history.search(&query);
            if let Some((_, entry)) = matches.get(selected) {
                app.input = entry.to_string();
                app.cursor_pos = app.input.len();
            }
            app.history_search = None;
        }
        KeyCode::Backspace => {
            search.query.pop();
            search.selected = 0;
        }
        KeyCode::Up => {
            let matches_len = app.history.search(&search.query).len();
            if matches_len > 0 && search.selected + 1 < matches_len {
                search.selected += 1;
            }
        }
        KeyCode::Down => {
            if search.selected > 0 {
                search.selected -= 1;
            }
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'r' {
                let matches_len = app.history.search(&search.query).len();
                if matches_len > 0 && search.selected + 1 < matches_len {
                    search.selected += 1;
                }
            } else {
                search.query.push(c);
                search.selected = 0;
            }
        }
        _ => {}
    }
}

fn move_cursor_up(app: &mut App) {
    let before = &app.input[..app.cursor_pos];
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    if line_start == 0 {
        // already on the first line, scroll chat instead
        app.scroll_offset = app.scroll_offset.saturating_add(1);
        return;
    }
    let col = app.cursor_pos - line_start;
    // prev_line is the line before current
    let prev_line_end = line_start - 1; // the '\n' separating previous and current line
    let prev_line_start = before[..prev_line_end]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    let prev_line_len = prev_line_end - prev_line_start;
    app.cursor_pos = prev_line_start + col.min(prev_line_len);
}

fn move_cursor_down(app: &mut App) {
    let after = &app.input[app.cursor_pos..];
    let Some(nl_offset) = after.find('\n') else {
        // already on the last line, scroll chat instead
        app.scroll_offset = app.scroll_offset.saturating_sub(1);
        return;
    };
    let before = &app.input[..app.cursor_pos];
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = app.cursor_pos - line_start;

    let next_line_start = app.cursor_pos + nl_offset + 1;
    let rest = &app.input[next_line_start..];
    let next_line_len = rest.find('\n').unwrap_or(rest.len());
    app.cursor_pos = next_line_start + col.min(next_line_len);
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

fn try_open_completion(app: &mut App, cwd: &std::path::Path) {
    use crate::completion::{detect_context, generate_candidates, CompletionState};

    let Some((ctx, prefix, start)) = detect_context(&app.input, app.cursor_pos) else {
        return;
    };
    let candidates = generate_candidates(&ctx, &prefix, cwd);
    if candidates.is_empty() {
        return;
    }
    app.completion = Some(CompletionState {
        candidates,
        selected: 0,
        prefix,
        prefix_start: start,
        context: ctx,
        scroll_offset: 0,
    });
}

fn accept_completion(app: &mut App, cwd: &std::path::Path) {
    use crate::completion::{apply_completion, detect_context, generate_candidates, CompletionState};

    let state = app.completion.take().unwrap();
    let is_dir = apply_completion(&mut app.input, &mut app.cursor_pos, &state);

    if is_dir {
        if let Some((ctx, prefix, start)) = detect_context(&app.input, app.cursor_pos) {
            let candidates = generate_candidates(&ctx, &prefix, cwd);
            if !candidates.is_empty() {
                app.completion = Some(CompletionState {
                    candidates,
                    selected: 0,
                    prefix,
                    prefix_start: start,
                    context: ctx,
                    scroll_offset: 0,
                });
            }
        }
    }
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
