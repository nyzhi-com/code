use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nyzhi_core::agent::AgentConfig;
use nyzhi_core::conversation::Thread;
use nyzhi_core::tools::{ToolContext, ToolRegistry};
use nyzhi_provider::{ContentPart, MessageContent, ModelInfo, Provider};
use tokio::sync::broadcast;

use crate::app::{App, AppMode, DisplayItem, PendingImage, TurnRequest};

#[allow(clippy::too_many_arguments)]
pub async fn handle_key(
    app: &mut App,
    key: KeyEvent,
    provider: Option<&dyn Provider>,
    thread: &mut Thread,
    agent_config: &mut AgentConfig,
    _event_tx: &broadcast::Sender<nyzhi_core::agent::AgentEvent>,
    _registry: &ToolRegistry,
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

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.code == KeyCode::Char('n')
        && app.search_query.is_some()
    {
        app.search_next();
        return;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL)
        && key.code == KeyCode::Char('p')
        && app.search_query.is_some()
    {
        app.search_prev();
        return;
    }

    match key.code {
        KeyCode::Tab | KeyCode::Down if app.completion.is_some() => {
            if let Some(ref mut state) = app.completion {
                state.cycle_forward();
            }
        }
        KeyCode::BackTab | KeyCode::Up if app.completion.is_some() => {
            if let Some(ref mut state) = app.completion {
                state.cycle_backward();
            }
        }
        KeyCode::Tab if app.completion.is_none() => {
            if app.input.is_empty() {
                cycle_thinking_level(app, model_info, false);
            } else {
                try_open_completion(app, &tool_ctx.cwd);
            }
        }
        KeyCode::BackTab if app.completion.is_none() && app.input.is_empty() => {
            cycle_thinking_level(app, model_info, true);
        }
        KeyCode::Esc => {
            if app.completion.is_some() {
                app.completion = None;
            } else if app.search_query.is_some() {
                app.clear_search();
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

            if input == "/compact" || input.starts_with("/compact ") {
                let focus_hint = input.strip_prefix("/compact").unwrap().trim();
                let focus = if focus_hint.is_empty() { None } else { Some(focus_hint) };

                if model_info.is_some() {
                    let est = thread.estimated_tokens(&agent_config.system_prompt);
                    let hint_msg = focus.map(|h| format!(" (focus: {h})")).unwrap_or_default();
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!(
                            "Compacting... (~{est} tokens, {} messages){hint_msg}",
                            thread.message_count()
                        ),
                    });
                    app.mode = AppMode::Streaming;

                    let summary_prompt =
                        nyzhi_core::context::build_compaction_prompt(thread.messages(), focus);
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
                        thinking: None,
                    };

                    let recent_files = nyzhi_core::context::extract_recent_file_paths(thread.messages(), 3);
                    let Some(provider) = provider else {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "No provider configured. Use /login first.".to_string(),
                        });
                        return;
                    };
                    match provider.chat(&summary_request).await {
                        Ok(resp) => {
                            let summary = resp.message.content.as_text().to_string();
                            thread.compact_with_restore(&summary, 4, &recent_files);
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

            if input == "/context" {
                let threshold = agent_config.auto_compact_threshold.unwrap_or(0.85);
                let cw = model_info.map(|m| m.context_window).unwrap_or(0);
                let breakdown = nyzhi_core::context::ContextBreakdown::compute(
                    thread.messages(),
                    &agent_config.system_prompt,
                    cw,
                    threshold,
                );
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: breakdown.format_display(),
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/sessions" || input.starts_with("/sessions ") {
                let query = input.strip_prefix("/sessions").unwrap().trim();
                let result = if query.is_empty() {
                    nyzhi_core::session::list_sessions()
                } else {
                    nyzhi_core::session::find_sessions(query)
                };
                match result {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            let msg = if query.is_empty() {
                                "No saved sessions.".to_string()
                            } else {
                                format!("No sessions matching '{query}'.")
                            };
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: msg,
                            });
                        } else {
                            let header = if query.is_empty() {
                                "Saved sessions:".to_string()
                            } else {
                                format!("Sessions matching '{query}':")
                            };
                            let mut lines = vec![header];
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

            if let Some(rest) = input.strip_prefix("/session delete ") {
                let id_prefix = rest.trim();
                if id_prefix.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Usage: /session delete <id-prefix>".to_string(),
                    });
                } else {
                    match nyzhi_core::session::find_sessions(id_prefix) {
                        Ok(matched) => match matched.len() {
                            0 => {
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!("No session matching '{id_prefix}'"),
                                });
                            }
                            1 => {
                                let target = &matched[0];
                                if target.id == thread.id {
                                    app.items.push(DisplayItem::Message {
                                        role: "system".to_string(),
                                        content: "Cannot delete the active session.".to_string(),
                                    });
                                } else {
                                    match nyzhi_core::session::delete_session(&target.id) {
                                        Ok(()) => {
                                            app.items.push(DisplayItem::Message {
                                                role: "system".to_string(),
                                                content: format!(
                                                    "Deleted session: [{}] {}",
                                                    &target.id[..8],
                                                    target.title,
                                                ),
                                            });
                                        }
                                        Err(e) => {
                                            app.items.push(DisplayItem::Message {
                                                role: "system".to_string(),
                                                content: format!("Error deleting session: {e}"),
                                            });
                                        }
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
                        },
                        Err(e) => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Error finding sessions: {e}"),
                            });
                        }
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if let Some(rest) = input.strip_prefix("/session rename ") {
                let new_title = rest.trim();
                if new_title.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Usage: /session rename <new title>".to_string(),
                    });
                } else {
                    match nyzhi_core::session::rename_session(&thread.id, new_title) {
                        Ok(()) => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Session renamed to: {new_title}"),
                            });
                        }
                        Err(e) => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Error renaming session: {e}"),
                            });
                        }
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

            if input == "/hooks" {
                if app.hooks_config.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No hooks configured.\n\nAdd hooks in .nyzhi/config.toml:\n\n  [[agent.hooks]]\n  event = \"after_edit\"\n  command = \"cargo fmt -- {file}\"\n  pattern = \"*.rs\"".to_string(),
                    });
                } else {
                    let mut lines = vec![format!("Configured hooks ({}):", app.hooks_config.len())];
                    for (i, h) in app.hooks_config.iter().enumerate() {
                        let pat = h.pattern.as_deref().unwrap_or("*");
                        lines.push(format!(
                            "  {}. [{}] {} (pattern: {}, timeout: {}s)",
                            i + 1,
                            h.event,
                            h.command,
                            pat,
                            h.timeout,
                        ));
                    }
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: lines.join("\n"),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/notify" || input.starts_with("/notify ") {
                let arg = input.strip_prefix("/notify").unwrap().trim();
                if arg.is_empty() {
                    let bell = if app.notify.bell { "on" } else { "off" };
                    let desktop = if app.notify.desktop { "on" } else { "off" };
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!(
                            "Notification settings:\n  bell:     {bell}\n  desktop:  {desktop}\n  duration: {}ms\n\nUsage:\n  /notify bell on|off\n  /notify desktop on|off\n  /notify duration <ms>",
                            app.notify.min_duration_ms,
                        ),
                    });
                } else {
                    let parts: Vec<&str> = arg.splitn(2, ' ').collect();
                    match parts.as_slice() {
                        ["bell", val] => match *val {
                            "on" | "true" | "1" => {
                                app.notify.bell = true;
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Terminal bell enabled.".to_string(),
                                });
                            }
                            "off" | "false" | "0" => {
                                app.notify.bell = false;
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Terminal bell disabled.".to_string(),
                                });
                            }
                            _ => {
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Usage: /notify bell on|off".to_string(),
                                });
                            }
                        },
                        ["desktop", val] => match *val {
                            "on" | "true" | "1" => {
                                app.notify.desktop = true;
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Desktop notifications enabled.".to_string(),
                                });
                            }
                            "off" | "false" | "0" => {
                                app.notify.desktop = false;
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Desktop notifications disabled.".to_string(),
                                });
                            }
                            _ => {
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Usage: /notify desktop on|off".to_string(),
                                });
                            }
                        },
                        ["duration", val] => {
                            if let Ok(ms) = val.parse::<u64>() {
                                app.notify.min_duration_ms = ms;
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!("Notification threshold set to {ms}ms."),
                                });
                            } else {
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Usage: /notify duration <ms> (e.g. /notify duration 5000)".to_string(),
                                });
                            }
                        }
                        _ => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: "Usage: /notify [bell on|off] [desktop on|off] [duration <ms>]".to_string(),
                            });
                        }
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/todo" {
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: "Reading todo list from agent store...".to_string(),
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/autopilot" || input.starts_with("/autopilot ") {
                let arg = input.strip_prefix("/autopilot").unwrap().trim();
                if arg.is_empty() {
                    match nyzhi_core::autopilot::load_state(&tool_ctx.project_root) {
                        Ok(Some(state)) => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: state.summary(),
                            });
                        }
                        _ => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: "No autopilot session active.\nUsage: /autopilot <idea> or use `autopilot:` prefix".to_string(),
                            });
                        }
                    }
                } else if arg == "cancel" {
                    if let Ok(Some(mut state)) = nyzhi_core::autopilot::load_state(&tool_ctx.project_root) {
                        state.cancel();
                        let _ = nyzhi_core::autopilot::save_state(&tool_ctx.project_root, &state);
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "Autopilot cancelled.".to_string(),
                        });
                    }
                } else if arg == "clear" {
                    let _ = nyzhi_core::autopilot::clear_state(&tool_ctx.project_root);
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Autopilot state cleared.".to_string(),
                    });
                } else {
                    let state = nyzhi_core::autopilot::AutopilotState::new(arg);
                    let _ = nyzhi_core::autopilot::save_state(&tool_ctx.project_root, &state);
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Autopilot initialized for: {arg}\n\n{}\n\nSending expansion prompt...", state.summary()),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input.starts_with("/team ") {
                let arg = input.strip_prefix("/team").unwrap().trim();
                let parts: Vec<&str> = arg.splitn(2, ' ').collect();
                if parts.len() < 2 {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Usage: /team <N> <task description>\nSpawns N coordinated sub-agents.".to_string(),
                    });
                } else if let Ok(n) = parts[0].parse::<u32>() {
                    let task = parts[1].to_string();
                    let config = nyzhi_core::team::TeamConfig { team_size: n, task: task.clone() };
                    let state = nyzhi_core::team::TeamState::new(&config);
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Team created for: {task}\n\n{}", state.summary()),
                    });
                } else {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "First argument must be a number. Usage: /team 3 refactor auth module".to_string(),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/learn" || input.starts_with("/learn ") {
                let arg = input.strip_prefix("/learn").unwrap().trim();
                if arg.is_empty() {
                    let skills = nyzhi_core::skills::load_skills(&tool_ctx.project_root).unwrap_or_default();
                    if skills.is_empty() {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "No skills learned yet.\nUsage: /learn <skill-name> to create a skill from this session.".to_string(),
                        });
                    } else {
                        let names: Vec<String> = skills.iter().map(|s| format!("  - {}", s.name)).collect();
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Learned skills:\n{}", names.join("\n")),
                        });
                    }
                } else {
                    let template = nyzhi_core::skills::build_skill_template(arg, "Extracted from session", &[]);
                    match nyzhi_core::skills::save_skill(&tool_ctx.project_root, arg, &template) {
                        Ok(path) => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Skill template saved to {}\nEdit the file to fill in details.", path.display()),
                            });
                        }
                        Err(e) => {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Error saving skill: {e}"),
                            });
                        }
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/notepad" || input.starts_with("/notepad ") {
                let arg = input.strip_prefix("/notepad").unwrap().trim();
                if arg.is_empty() {
                    let plans = nyzhi_core::notepad::list_notepads(&tool_ctx.project_root).unwrap_or_default();
                    if plans.is_empty() {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "No notepads found.".to_string(),
                        });
                    } else {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Notepads:\n{}", plans.iter().map(|p| format!("  - {p}")).collect::<Vec<_>>().join("\n")),
                        });
                    }
                } else if let Ok(content) = nyzhi_core::notepad::read_notepad(&tool_ctx.project_root, arg) {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content,
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/plan" || input.starts_with("/plan ") {
                let arg = input.strip_prefix("/plan").unwrap().trim();
                if arg.is_empty() {
                    let plans = nyzhi_core::planning::list_plans(&tool_ctx.project_root).unwrap_or_default();
                    if plans.is_empty() {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "No plans saved.\nUse `plan: <task>` prefix to activate iterative planning.".to_string(),
                        });
                    } else {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Saved plans:\n{}",
                                plans.iter().map(|p| format!("  - {p}")).collect::<Vec<_>>().join("\n"),
                            ),
                        });
                    }
                } else if let Ok(Some(content)) = nyzhi_core::planning::load_plan(&tool_ctx.project_root, arg) {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content,
                    });
                } else {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Plan '{arg}' not found."),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/persist" || input.starts_with("/persist ") {
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: "Persistence mode: agent will verify after each turn and auto-fix failures.\nUse `persist:` prefix in your next prompt to activate.".to_string(),
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/qa" || input.starts_with("/qa ") {
                let checks = nyzhi_core::verify::detect_checks(&tool_ctx.project_root);
                if checks.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No verification checks detected for QA.".to_string(),
                    });
                } else {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!(
                            "QA mode available: {} checks detected.\nUse `qa:` prefix in your next prompt to run autonomous QA cycling.",
                            checks.len()
                        ),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/verify" {
                let checks = nyzhi_core::verify::detect_checks(&tool_ctx.project_root);
                if checks.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No verification checks detected for this project.".to_string(),
                    });
                } else {
                    let cmds: Vec<String> = checks.iter().map(|c| format!("  [{}] {}", c.kind, c.command)).collect();
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!(
                            "Detected {} checks:\n{}\n\nUse the `verify` tool in a prompt to run them.",
                            checks.len(),
                            cmds.join("\n"),
                        ),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/commands" {
                if app.custom_commands.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No custom commands defined.\n\nCreate commands as .md files in .nyzhi/commands/ or in .nyzhi/config.toml:\n\n  [[agent.commands]]\n  name = \"review\"\n  prompt = \"Review $ARGUMENTS for bugs and improvements\"\n  description = \"Code review\"".to_string(),
                    });
                } else {
                    let mut lines = vec![format!("Custom commands ({}):", app.custom_commands.len())];
                    for cmd in &app.custom_commands {
                        let desc = if cmd.description.is_empty() {
                            "(no description)".to_string()
                        } else {
                            cmd.description.clone()
                        };
                        lines.push(format!("  /{:<16} {}", cmd.name, desc));
                    }
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: lines.join("\n"),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/export" || input.starts_with("/export ") {
                let arg = input.strip_prefix("/export").unwrap().trim();
                let path = if arg.is_empty() {
                    std::path::PathBuf::from(crate::export::default_export_path())
                } else if let Some(rest) = arg.strip_prefix('~') {
                    if let Some(home) = dirs::home_dir() {
                        home.join(rest.strip_prefix('/').unwrap_or(rest))
                    } else {
                        std::path::PathBuf::from(arg)
                    }
                } else {
                    std::path::PathBuf::from(arg)
                };

                let meta = crate::export::ExportMeta {
                    provider: app.provider_name.clone(),
                    model: app.model_name.clone(),
                    usage: app.session_usage.clone(),
                    timestamp: chrono::Utc::now(),
                };
                let markdown = crate::export::export_session_markdown(&app.items, &meta);

                match std::fs::write(&path, &markdown) {
                    Ok(()) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Exported {} items ({} bytes) to {}",
                                app.items.len() - 1,
                                markdown.len(),
                                path.display(),
                            ),
                        });
                    }
                    Err(e) => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Export failed: {e}"),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input.starts_with("/search ") {
                let query = input.strip_prefix("/search ").unwrap().trim();
                if query.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Usage: /search <query>".to_string(),
                    });
                } else {
                    app.run_search(query);
                    if app.search_matches.is_empty() {
                        app.clear_search();
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("No results for \"{query}\""),
                        });
                    } else {
                        let count = app.search_matches.len();
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Found {count} match{} for \"{query}\" -- Ctrl+N/Ctrl+P to navigate, Esc to clear",
                                if count == 1 { "" } else { "es" },
                            ),
                        });
                        app.scroll_offset = 0;
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/model" {
                let models = provider.map(|p| p.supported_models()).unwrap_or(&[]);
                if models.is_empty() {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "No models available. Configure a provider first.".to_string(),
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
                let registry = nyzhi_provider::ModelRegistry::new();
                if let Some((prov, found)) = registry.find_any(new_model) {
                    app.provider_name = prov.to_string();
                    app.model_name = found.id.clone();
                    *model_info_idx = None;
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Switched to {}/{} ({})", prov, found.id, found.name),
                    });
                } else if let Some(idx) = provider
                    .and_then(|p| p.supported_models()
                        .iter()
                        .position(|m| m.id == new_model))
                {
                    let mi = &provider.unwrap().supported_models()[idx];
                    app.model_name = mi.id.clone();
                    *model_info_idx = Some(idx);
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Switched to {} ({})", mi.id, mi.name),
                    });
                } else {
                    let available: Vec<&str> = provider
                        .map(|p| p.supported_models().iter().map(|m| m.id.as_str()).collect())
                        .unwrap_or_default();
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

            if input == "/thinking" {
                app.open_thinking_selector(model_info);
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if let Some(level) = input.strip_prefix("/thinking ") {
                let level = level.trim();
                if level == "off" {
                    app.thinking_level = None;
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Thinking disabled.".to_string(),
                    });
                } else {
                    app.thinking_level = Some(level.to_string());
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: format!("Thinking level set to: {}", level),
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

            if input == "/connect" {
                app.open_provider_selector();
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/login" {
                let mut lines = vec!["Auth status:".to_string()];
                for def in nyzhi_config::BUILT_IN_PROVIDERS {
                    let status = nyzhi_auth::auth_status(def.id);
                    let marker = if status != "not connected" { "✓" } else { "✗" };
                    let mut line = format!("  {marker} {}: {status}", def.name);
                    if let Ok(accounts) = nyzhi_auth::token_store::list_accounts(def.id) {
                        if accounts.len() > 1 {
                            let active_count = accounts.iter().filter(|a| a.active).count();
                            let rl_count = accounts.iter()
                                .filter(|a| a.rate_limited_until.is_some())
                                .count();
                            line.push_str(&format!(
                                " ({} accounts, {} active{})",
                                accounts.len(),
                                active_count,
                                if rl_count > 0 { format!(", {} rate-limited", rl_count) } else { String::new() }
                            ));
                        }
                    }
                    lines.push(line);
                }
                if let Some(ref level) = app.thinking_level {
                    lines.push(format!("\nThinking: {level}"));
                }
                lines.push(String::new());
                lines.push("Use /connect to add a provider (OAuth or API key).".to_string());
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
                let deny_tools = &agent_config.trust.deny_tools;
                let deny_paths = &agent_config.trust.deny_paths;
                let mut msg = format!("Trust mode: {mode}");
                if !tools.is_empty() {
                    msg.push_str(&format!("\nAllowed tools: {}", tools.join(", ")));
                }
                if !paths.is_empty() {
                    msg.push_str(&format!("\nAllowed paths: {}", paths.join(", ")));
                }
                if !deny_tools.is_empty() {
                    msg.push_str(&format!("\nDenied tools: {}", deny_tools.join(", ")));
                }
                if !deny_paths.is_empty() {
                    msg.push_str(&format!("\nDenied paths: {}", deny_paths.join(", ")));
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

            if input == "/bg" || input == "/background" || input.starts_with("/bg ") || input.starts_with("/background ") {
                let arg = input
                    .strip_prefix("/background")
                    .or_else(|| input.strip_prefix("/bg"))
                    .unwrap_or("")
                    .trim();
                if arg.is_empty() {
                    if app.background_tasks.is_empty() {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "No background tasks running.".to_string(),
                        });
                    } else {
                        let mut lines = vec![format!("Background tasks ({}):", app.background_tasks.len())];
                        for bg in &app.background_tasks {
                            let elapsed = bg.started.elapsed();
                            lines.push(format!(
                                "  #{}: {} ({:.1}s)",
                                bg.id, bg.label, elapsed.as_secs_f64()
                            ));
                        }
                        lines.push(String::new());
                        lines.push("Use /bg kill <id> to cancel a task.".to_string());
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: lines.join("\n"),
                        });
                    }
                } else if let Some(rest) = arg.strip_prefix("kill") {
                    let rest = rest.trim();
                    if let Ok(id) = rest.parse::<usize>() {
                        if let Some(pos) = app.background_tasks.iter().position(|b| b.id == id) {
                            let bg = app.background_tasks.remove(pos);
                            bg.join_handle.abort();
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("Killed background task #{id}: {}", bg.label),
                            });
                        } else {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: format!("No background task with id #{id}"),
                            });
                        }
                    } else {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "Usage: /bg kill <id>".to_string(),
                        });
                    }
                } else {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Usage: /bg [kill <id>]".to_string(),
                    });
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/enable_exa" {
                // Check if already configured
                let config_check = nyzhi_config::Config::load().ok();
                let already = config_check
                    .as_ref()
                    .map(|c| c.mcp.servers.contains_key("exa"))
                    .unwrap_or(false);

                if already {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Exa is already configured in your config.".to_string(),
                    });
                } else {
                    use crate::components::text_prompt::{TextPromptKind, TextPromptState};
                    app.text_prompt = Some(TextPromptState::new(
                        TextPromptKind::ExaApiKey,
                        "Enable Exa Web Search",
                        &[
                            "Exa provides AI-powered web search with",
                            "clean, structured results.",
                            "Get your API key at: dashboard.exa.ai",
                        ],
                        "Paste your API key from dashboard.exa.ai",
                        true,
                    ));
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/doctor" {
                let results = nyzhi_core::diagnostics::run_diagnostics(
                    &app.provider_name,
                    &tool_ctx.project_root,
                    0, // MCP count approximation
                    _registry.names().len(),
                );
                let mut output = nyzhi_core::diagnostics::format_diagnostics(&results);

                output.push_str("\n\nConfig Compatibility:\n");
                output.push_str(&format!("  Config source: {}\n", app.workspace.config_source.label()));
                if let Some(rf) = &app.workspace.rules_file {
                    output.push_str(&format!("  Rules loaded from: {rf}\n"));
                }
                let root = &app.workspace.project_root;
                for (label, path) in [
                    ("AGENTS.md", root.join("AGENTS.md")),
                    ("CLAUDE.md", root.join("CLAUDE.md")),
                    (".cursorrules", root.join(".cursorrules")),
                    (".nyzhi/", root.join(".nyzhi")),
                    (".claude/", root.join(".claude")),
                ] {
                    if path.exists() {
                        output.push_str(&format!("  Found: {label}\n"));
                    }
                }

                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: output,
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/bug" {
                let report = nyzhi_core::diagnostics::generate_bug_report(
                    &app.provider_name,
                    &app.model_name,
                    &format!("{}", agent_config.trust.mode),
                    &tool_ctx.session_id,
                );
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: report,
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/status" {
                let usage = &app.session_usage;
                let elapsed = app.session_start.elapsed();
                let mins = elapsed.as_secs() / 60;
                let secs = elapsed.as_secs() % 60;
                let content = format!(
                    "Session Status\n\n\
                     Provider: {}\n\
                     Model: {}\n\
                     Trust mode: {}\n\
                     Output style: {}\n\
                     Thinking: {}\n\
                     Config source: {}\n\
                     Rules: {}\n\
                     Session duration: {mins}m {secs}s\n\
                     Token usage:\n\
                       Input:  {} (cached read: {}, cached write: {})\n\
                       Output: {}\n\
                       Cost:   ${:.4}\n\
                     Hooks: {} configured\n\
                     Messages: {} items",
                    app.provider_name,
                    app.model_name,
                    agent_config.trust.mode,
                    app.output_style,
                    if agent_config.thinking_enabled { "on" } else { "off" },
                    app.workspace.config_source.label(),
                    app.workspace.rules_file.as_deref().unwrap_or("none"),
                    usage.total_input_tokens,
                    usage.total_cache_read_tokens,
                    usage.total_cache_creation_tokens,
                    usage.total_output_tokens,
                    usage.total_cost_usd,
                    app.hooks_config.len(),
                    app.items.len(),
                );
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content,
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/style" || input.starts_with("/style ") {
                let arg = input.strip_prefix("/style").unwrap().trim();
                match arg {
                    "normal" => {
                        app.output_style = nyzhi_config::OutputStyle::Normal;
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "Output style: normal".to_string(),
                        });
                    }
                    "verbose" => {
                        app.output_style = nyzhi_config::OutputStyle::Verbose;
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "Output style: verbose (all tool args/outputs expanded)".to_string(),
                        });
                    }
                    "minimal" => {
                        app.output_style = nyzhi_config::OutputStyle::Minimal;
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "Output style: minimal (tool details hidden)".to_string(),
                        });
                    }
                    "structured" => {
                        app.output_style = nyzhi_config::OutputStyle::Structured;
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "Output style: structured (JSON)".to_string(),
                        });
                    }
                    "" => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!("Current output style: {}", app.output_style),
                        });
                    }
                    _ => {
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "Usage: /style [normal|verbose|minimal|structured]".to_string(),
                        });
                    }
                }
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/agents" {
                let built_in = nyzhi_core::agent_roles::built_in_roles();
                let empty = std::collections::HashMap::new();
                let file_roles =
                    nyzhi_core::agent_files::load_file_based_roles(&tool_ctx.project_root);
                let list = nyzhi_core::agent_files::format_role_list(
                    &built_in,
                    &empty,
                    &file_roles,
                );
                app.items.push(DisplayItem::Message {
                    role: "system".to_string(),
                    content: list,
                });
                app.input.clear();
                app.cursor_pos = 0;
                return;
            }

            if input == "/think" || input.starts_with("/think ") {
                let arg = input.strip_prefix("/think").unwrap().trim();
                match arg {
                    "on" | "" => {
                        agent_config.thinking_enabled = true;
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: format!(
                                "Extended thinking enabled (budget: {} tokens)",
                                agent_config.thinking_budget.unwrap_or(10_000)
                            ),
                        });
                    }
                    "off" => {
                        agent_config.thinking_enabled = false;
                        app.items.push(DisplayItem::Message {
                            role: "system".to_string(),
                            content: "Extended thinking disabled".to_string(),
                        });
                    }
                    other => {
                        if let Some(rest) = other.strip_prefix("budget ") {
                            if let Ok(b) = rest.trim().parse::<u32>() {
                                agent_config.thinking_budget = Some(b);
                                agent_config.thinking_enabled = true;
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: format!(
                                        "Extended thinking enabled with budget: {b} tokens"
                                    ),
                                });
                            } else {
                                app.items.push(DisplayItem::Message {
                                    role: "system".to_string(),
                                    content: "Usage: /think budget <number>".to_string(),
                                });
                            }
                        } else {
                            app.items.push(DisplayItem::Message {
                                role: "system".to_string(),
                                content: "Usage: /think [on|off|budget <N>]".to_string(),
                            });
                        }
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
                        "  /help           Show this help",
                        "  /model          Pick model from all providers",
                        "  /model <id>     Switch model (e.g. anthropic/claude-opus-4.6)",
                        "  /thinking       Set thinking/reasoning level",
                        "  /thinking <lvl> Set level directly (off/low/medium/high)",
                        "  /image <path>   Attach an image for the next prompt",
                        "  /connect        Connect a provider (OAuth or API key)",
                        "  /login          Show auth status for all providers",
                        "  /init           Initialize .nyzhi/ project config",
                        "  /mcp            List connected MCP servers",
                        "  /commands       List custom commands",
                        "  /hooks          List configured hooks",
                        "  /clear          Clear the session",
                        "  /compact [hint] Compress conversation history (optional focus hint)",
                        "  /context        Show context window usage breakdown",
                        "  /sessions [q]   List saved sessions (optionally filter)",
                        "  /resume <id>    Restore a saved session",
                        "  /session delete <id>  Delete a saved session",
                        "  /session rename <t>   Rename current session",
                        "  /theme          Choose theme (dark/light)",
                        "  /accent         Choose accent color",
                        "  /trust          Show current trust mode",
                        "  /trust <mode>   Set trust mode (off, limited, full)",
                        "  /editor         Open $EDITOR for multi-line input",
                        "  /retry          Resend the last prompt",
                        "  /undo           Undo the last file change",
                        "  /undo all       Undo all file changes in this session",
                        "  /changes        List all file changes in this session",
                        "  /export [path]  Export conversation as markdown",
                        "  /search <q>     Search session (Ctrl+N/P next/prev, Esc clear)",
                        "  /think          Toggle extended thinking (on/off/budget N)",
                        "  /bg             List background tasks",
                        "  /bg kill <id>   Cancel a background task",
                        "  /notify         Show notification settings",
                        "  /notify bell|desktop on|off  Toggle notifications",
                        "  /notify duration <ms>        Set min turn duration threshold",
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
                        "Config compatibility:",
                        "  Recognizes AGENTS.md, CLAUDE.md, .cursorrules for project rules",
                        "  Scans .nyzhi/ and .claude/ for commands, agents, and skills",
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
                        "Background:",
                        "  & <prompt>      Run prompt in background",
                        "  ctrl+b          Move current task to background (during streaming)",
                        "  ctrl+f          Kill all background tasks (double-press)",
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

            if input == "/retry" {
                if let Some(ref last) = app.last_prompt {
                    let retry_input = last.clone();
                    app.input.clear();
                    app.cursor_pos = 0;
                    let label = truncate_label(&retry_input);
                    app.items.push(DisplayItem::Message {
                        role: "user".to_string(),
                        content: format!("[retry] {retry_input}"),
                    });
                    app.mode = AppMode::Streaming;
                    app.turn_request = Some(TurnRequest {
                        input: retry_input,
                        content: None,
                        is_background: false,
                        label,
                    });
                } else {
                    app.items.push(DisplayItem::Message {
                        role: "system".to_string(),
                        content: "Nothing to retry".to_string(),
                    });
                    app.input.clear();
                    app.cursor_pos = 0;
                }
                return;
            }

            if let Some(cmd) = app.custom_commands.iter().find(|c| {
                input == format!("/{}", c.name)
                    || input.starts_with(&format!("/{} ", c.name))
            }) {
                let args = input
                    .strip_prefix(&format!("/{}", cmd.name))
                    .unwrap_or("")
                    .trim();
                let expanded = nyzhi_core::commands::expand_template(&cmd.prompt_template, args);
                app.last_prompt = Some(expanded.clone());
                app.history.push(input.clone());
                let label = truncate_label(format!("/{} {args}", cmd.name).trim());
                app.items.push(DisplayItem::Message {
                    role: "user".to_string(),
                    content: format!("/{} {args}", cmd.name).trim().to_string(),
                });
                app.input.clear();
                app.cursor_pos = 0;
                app.mode = AppMode::Streaming;
                app.turn_request = Some(TurnRequest {
                    input: expanded,
                    content: None,
                    is_background: false,
                    label,
                });
                return;
            }

            let is_background = input.starts_with('&');
            let input = if is_background {
                input[1..].trim().to_string()
            } else {
                input
            };
            if input.is_empty() {
                return;
            }

            let (flags, cleaned_input) = nyzhi_core::keywords::detect_keywords(&input);
            let input = if flags.any() { cleaned_input } else { input };

            if flags.think {
                agent_config.thinking_enabled = true;
            }

            if let Some(ref level) = app.thinking_level {
                agent_config.thinking_enabled = true;
                match level.as_str() {
                    "low" => {
                        agent_config.reasoning_effort = Some("low".into());
                        agent_config.thinking_budget = Some(4096);
                        agent_config.thinking_level = Some("low".into());
                    }
                    "medium" => {
                        agent_config.reasoning_effort = Some("medium".into());
                        agent_config.thinking_budget = Some(8192);
                        agent_config.thinking_level = Some("medium".into());
                    }
                    "high" => {
                        agent_config.reasoning_effort = Some("high".into());
                        agent_config.thinking_budget = Some(16384);
                        agent_config.thinking_level = Some("high".into());
                    }
                    "xhigh" => {
                        agent_config.reasoning_effort = Some("xhigh".into());
                        agent_config.thinking_budget = Some(32768);
                        agent_config.thinking_level = Some("xhigh".into());
                    }
                    "max" => {
                        agent_config.reasoning_effort = Some("max".into());
                        agent_config.thinking_budget = Some(32768);
                        agent_config.thinking_level = Some("max".into());
                    }
                    other => {
                        agent_config.reasoning_effort = Some(other.into());
                        agent_config.thinking_level = Some(other.into());
                    }
                }
            }

            app.last_prompt = Some(input.clone());
            app.history.push(if is_background { format!("&{input}") } else { input.clone() });

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
            if is_background {
                display_content = format!("[bg] {display_content}");
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

            let label = truncate_label(&input);
            let content = if has_images {
                let images = std::mem::take(&mut app.pending_images);
                Some(build_multimodal_content(&agent_input, &images))
            } else {
                None
            };

            app.input.clear();
            app.cursor_pos = 0;

            if is_background {
                app.turn_request = Some(TurnRequest {
                    input: agent_input,
                    content,
                    is_background: true,
                    label,
                });
            } else {
                app.mode = AppMode::Streaming;
                app.turn_request = Some(TurnRequest {
                    input: agent_input,
                    content,
                    is_background: false,
                    label,
                });
            }
        }
        KeyCode::Char(c) => {
            if c == '/' && app.input.is_empty() {
                app.open_command_selector();
            } else {
                app.input.insert(app.cursor_pos, c);
                app.cursor_pos += 1;
                app.history.reset_cursor();
                app.completion = None;
                try_open_completion(app, &tool_ctx.cwd);
            }
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                app.input.remove(app.cursor_pos);
            }
            app.completion = None;
            if !app.input.is_empty() {
                try_open_completion(app, &tool_ctx.cwd);
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
        KeyCode::PageUp => {
            app.scroll_offset = app.scroll_offset.saturating_add(20);
        }
        KeyCode::PageDown => {
            app.scroll_offset = app.scroll_offset.saturating_sub(20);
        }
        KeyCode::Delete => {
            if app.cursor_pos < app.input.len() {
                app.input.remove(app.cursor_pos);
            }
            app.completion = None;
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

fn cycle_thinking_level(app: &mut App, model_info: Option<&ModelInfo>, reverse: bool) {
    let levels: Vec<&str> = model_info
        .and_then(|m| m.thinking.as_ref())
        .map(|ts| ts.cycle_levels())
        .unwrap_or_else(|| vec!["off", "low", "medium", "high"]);

    if levels.is_empty() {
        return;
    }

    let current = app.thinking_level.as_deref().unwrap_or("off");
    let current_idx = levels.iter().position(|&l| l == current).unwrap_or(0);

    let next_idx = if reverse {
        if current_idx == 0 { levels.len() - 1 } else { current_idx - 1 }
    } else {
        (current_idx + 1) % levels.len()
    };

    let next = levels[next_idx];
    if next == "off" {
        app.thinking_level = None;
        app.items.push(DisplayItem::Message {
            role: "system".to_string(),
            content: "Thinking: off".to_string(),
        });
    } else {
        app.thinking_level = Some(next.to_string());
        app.items.push(DisplayItem::Message {
            role: "system".to_string(),
            content: format!("Thinking: {}", next),
        });
    }
    app.scroll_offset = 0;
}

fn try_open_completion(app: &mut App, cwd: &std::path::Path) {
    use crate::completion::{detect_context, generate_candidates, CompletionState};

    let Some((ctx, prefix, start)) = detect_context(&app.input, app.cursor_pos) else {
        return;
    };
    let (candidates, descriptions) = generate_candidates(&ctx, &prefix, cwd, &app.custom_commands);
    if candidates.is_empty() {
        return;
    }
    app.completion = Some(CompletionState {
        candidates,
        descriptions,
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
            let (candidates, descriptions) =
                generate_candidates(&ctx, &prefix, cwd, &app.custom_commands);
            if !candidates.is_empty() {
                app.completion = Some(CompletionState {
                    candidates,
                    descriptions,
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

fn truncate_label(s: &str) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > 60 {
        format!("{}...", &first_line[..57])
    } else {
        first_line.to_string()
    }
}
