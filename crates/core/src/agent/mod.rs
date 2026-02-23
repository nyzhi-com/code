use anyhow::Result;
use futures::StreamExt;
use nyzhi_config::{TrustConfig, TrustMode};
use nyzhi_provider::{
    ChatRequest, ContentPart, Message, MessageContent, ModelInfo, Provider, ProviderError, Role,
    StreamEvent,
};
use tokio::sync::broadcast;

use crate::conversation::Thread;
use crate::streaming::StreamAccumulator;
use crate::tools::permission::ToolPermission;
use crate::tools::{ToolContext, ToolRegistry};

#[derive(Debug, Clone, Default)]
pub struct SessionUsage {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub total_cost_usd: f64,
    pub turn_input_tokens: u32,
    pub turn_output_tokens: u32,
    pub turn_cache_read_tokens: u32,
    pub turn_cache_creation_tokens: u32,
    pub turn_cost_usd: f64,
}

#[derive(Clone)]
pub enum AgentEvent {
    ThinkingDelta(String),
    TextDelta(String),
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta {
        id: String,
        args_delta: String,
    },
    ToolCallDone {
        id: String,
        name: String,
        output: String,
        elapsed_ms: u64,
    },
    ToolOutputDelta {
        tool_name: String,
        delta: String,
    },
    ApprovalRequest {
        tool_name: String,
        args_summary: String,
        respond: std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>,
    },
    Retrying {
        attempt: u32,
        max_retries: u32,
        wait_ms: u64,
        reason: String,
    },
    AutoCompacting {
        estimated_tokens: usize,
        context_window: u32,
    },
    RoutedModel {
        model_name: String,
        tier: String,
    },
    SubAgentSpawned {
        id: String,
        nickname: String,
        role: Option<String>,
    },
    SubAgentStatusChanged {
        id: String,
        nickname: String,
        status: String,
    },
    SubAgentCompleted {
        id: String,
        nickname: String,
        final_message: Option<String>,
    },
    ContextUpdate {
        estimated_tokens: usize,
        context_window: u32,
    },
    UserQuestion {
        question: String,
        options: Vec<(String, String)>,
        allow_custom: bool,
        respond: std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<String>>>>,
    },
    Usage(SessionUsage),
    SystemMessage(String),
    TurnComplete,
    Error(String),
}

impl std::fmt::Debug for AgentEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ThinkingDelta(s) => f.debug_tuple("ThinkingDelta").field(s).finish(),
            Self::TextDelta(s) => f.debug_tuple("TextDelta").field(s).finish(),
            Self::ToolCallStart { id, name } => f
                .debug_struct("ToolCallStart")
                .field("id", id)
                .field("name", name)
                .finish(),
            Self::ToolCallDelta { id, args_delta } => f
                .debug_struct("ToolCallDelta")
                .field("id", id)
                .field("args_delta", args_delta)
                .finish(),
            Self::ToolCallDone {
                id,
                name,
                output,
                elapsed_ms,
            } => f
                .debug_struct("ToolCallDone")
                .field("id", id)
                .field("name", name)
                .field("output", output)
                .field("elapsed_ms", elapsed_ms)
                .finish(),
            Self::ToolOutputDelta {
                tool_name, delta, ..
            } => f
                .debug_struct("ToolOutputDelta")
                .field("tool_name", tool_name)
                .field("delta", delta)
                .finish(),
            Self::ApprovalRequest {
                tool_name,
                args_summary,
                ..
            } => f
                .debug_struct("ApprovalRequest")
                .field("tool_name", tool_name)
                .field("args_summary", args_summary)
                .finish(),
            Self::Retrying {
                attempt,
                max_retries,
                wait_ms,
                reason,
            } => f
                .debug_struct("Retrying")
                .field("attempt", attempt)
                .field("max_retries", max_retries)
                .field("wait_ms", wait_ms)
                .field("reason", reason)
                .finish(),
            Self::AutoCompacting {
                estimated_tokens,
                context_window,
            } => f
                .debug_struct("AutoCompacting")
                .field("estimated_tokens", estimated_tokens)
                .field("context_window", context_window)
                .finish(),
            Self::RoutedModel { model_name, tier } => f
                .debug_struct("RoutedModel")
                .field("model_name", model_name)
                .field("tier", tier)
                .finish(),
            Self::SubAgentSpawned { id, nickname, role } => f
                .debug_struct("SubAgentSpawned")
                .field("id", id)
                .field("nickname", nickname)
                .field("role", role)
                .finish(),
            Self::SubAgentStatusChanged {
                id,
                nickname,
                status,
            } => f
                .debug_struct("SubAgentStatusChanged")
                .field("id", id)
                .field("nickname", nickname)
                .field("status", status)
                .finish(),
            Self::SubAgentCompleted {
                id,
                nickname,
                final_message,
            } => f
                .debug_struct("SubAgentCompleted")
                .field("id", id)
                .field("nickname", nickname)
                .field("final_message", final_message)
                .finish(),
            Self::ContextUpdate {
                estimated_tokens,
                context_window,
            } => f
                .debug_struct("ContextUpdate")
                .field("estimated_tokens", estimated_tokens)
                .field("context_window", context_window)
                .finish(),
            Self::UserQuestion {
                question,
                options,
                allow_custom,
                ..
            } => f
                .debug_struct("UserQuestion")
                .field("question", question)
                .field("options_count", &options.len())
                .field("allow_custom", allow_custom)
                .finish(),
            Self::Usage(u) => f.debug_struct("Usage").field("usage", u).finish(),
            Self::SystemMessage(s) => f.debug_tuple("SystemMessage").field(s).finish(),
            Self::TurnComplete => write!(f, "TurnComplete"),
            Self::Error(s) => f.debug_tuple("Error").field(s).finish(),
        }
    }
}

#[derive(Clone)]
pub struct AgentConfig {
    pub name: String,
    pub system_prompt: String,
    pub max_steps: u32,
    pub max_tokens: Option<u32>,
    pub trust: TrustConfig,
    pub retry: nyzhi_config::RetrySettings,
    pub routing: nyzhi_config::RoutingConfig,
    pub auto_compact_threshold: Option<f64>,
    pub compact_instructions: Option<String>,
    pub thinking_enabled: bool,
    pub thinking_budget: Option<u32>,
    pub reasoning_effort: Option<String>,
    pub thinking_level: Option<String>,
    pub team_name: Option<String>,
    pub agent_name: Option<String>,
    pub plan_mode: bool,
    pub act_after_plan: bool,
    pub auto_context: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "build".to_string(),
            system_prompt: crate::prompt::default_system_prompt(),
            max_steps: 100,
            max_tokens: None,
            trust: TrustConfig::default(),
            retry: nyzhi_config::RetrySettings::default(),
            routing: nyzhi_config::RoutingConfig::default(),
            auto_compact_threshold: None,
            compact_instructions: None,
            thinking_enabled: false,
            thinking_budget: None,
            reasoning_effort: None,
            thinking_level: None,
            team_name: None,
            agent_name: None,
            plan_mode: false,
            act_after_plan: false,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_turn(
    provider: &dyn Provider,
    thread: &mut Thread,
    user_input: &str,
    config: &AgentConfig,
    event_tx: &broadcast::Sender<AgentEvent>,
    registry: &ToolRegistry,
    ctx: &ToolContext,
    model_info: Option<&ModelInfo>,
    session_usage: &mut SessionUsage,
) -> Result<()> {
    run_turn_with_content(
        provider,
        thread,
        MessageContent::Text(user_input.to_string()),
        config,
        event_tx,
        registry,
        ctx,
        model_info,
        session_usage,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn run_turn_with_content(
    provider: &dyn Provider,
    thread: &mut Thread,
    user_content: MessageContent,
    config: &AgentConfig,
    event_tx: &broadcast::Sender<AgentEvent>,
    registry: &ToolRegistry,
    ctx: &ToolContext,
    model_info: Option<&ModelInfo>,
    session_usage: &mut SessionUsage,
) -> Result<()> {
    thread.push_message(Message {
        role: Role::User,
        content: user_content,
    });

    let tool_defs = if config.plan_mode {
        registry.definitions_read_only()
    } else if let Some(allowed) = &ctx.allowed_tool_names {
        registry.definitions_filtered(allowed)
    } else {
        registry.definitions()
    };
    let system_prompt = if config.plan_mode {
        format!(
            "{}{}",
            config.system_prompt,
            crate::prompt::plan_mode_instructions()
        )
    } else if config.act_after_plan {
        format!(
            "{}{}",
            config.system_prompt,
            crate::prompt::act_after_plan_instructions()
        )
    } else {
        config.system_prompt.clone()
    };
    let max_tokens = config
        .max_tokens
        .or_else(|| model_info.map(|m| m.max_output_tokens));

    session_usage.turn_input_tokens = 0;
    session_usage.turn_output_tokens = 0;
    session_usage.turn_cache_read_tokens = 0;
    session_usage.turn_cache_creation_tokens = 0;
    session_usage.turn_cost_usd = 0.0;

    let microcompact_dir = std::env::temp_dir()
        .join("nyzhi_microcompact")
        .join(&ctx.session_id);
    let context_dir = ctx.project_root.join(".nyzhi").join("context");
    let mut compact_count: u32 = 0;

    for step in 0..config.max_steps {
        // Inbox polling: inject unread teammate messages before the LLM call
        if let (Some(team), Some(agent)) = (&ctx.team_name, &ctx.agent_name) {
            if let Ok(unread) = crate::teams::mailbox::read_unread(team, agent) {
                if !unread.is_empty() {
                    let injected = crate::teams::mailbox::format_messages_for_injection(&unread);
                    thread.push_message(Message {
                        role: Role::User,
                        content: MessageContent::Text(injected),
                    });
                }
            }
        }

        if let Some(mi) = model_info {
            let est = thread.estimated_tokens(&system_prompt);
            let _ = event_tx.send(AgentEvent::ContextUpdate {
                estimated_tokens: est,
                context_window: mi.context_window,
            });

            let threshold = config.auto_compact_threshold.unwrap_or(0.85);

            // Phase 1: Progressive pruning (dedup, supersede writes, error prune, microcompact)
            let savings = crate::context::progressive_compact(
                thread.messages_mut(),
                &microcompact_dir,
                est,
                mi.context_window,
                threshold,
            );
            for (desc, saved) in &savings {
                let _ = event_tx.send(AgentEvent::SystemMessage(format!(
                    "compaction: {desc} (saved ~{saved} tokens)"
                )));
            }

            // Phase 2: Full summarization if still over threshold
            let est_after = thread.estimated_tokens(&system_prompt);
            if crate::context::needs_full_compact(
                est_after,
                mi.context_window,
                threshold,
                thread.message_count(),
            ) {
                let _ = event_tx.send(AgentEvent::AutoCompacting {
                    estimated_tokens: est_after,
                    context_window: mi.context_window,
                });

                let history_ref = crate::context::save_history_file(
                    thread.messages(),
                    &ctx.session_id,
                    compact_count,
                    &context_dir,
                );
                compact_count += 1;

                let recent_files = crate::context::extract_recent_file_paths(thread.messages(), 5);

                let previous_summary = if compact_count > 1 {
                    thread.messages().first().and_then(|m| {
                        let t = m.content.as_text();
                        if t.starts_with("[Conversation summary]") {
                            Some(t[22..].to_string())
                        } else {
                            None
                        }
                    })
                } else {
                    None
                };

                let summary_prompt = crate::context::build_compaction_prompt_full(
                    thread.messages(),
                    None,
                    previous_summary.as_deref(),
                    config.compact_instructions.as_deref(),
                );

                let summary_request = ChatRequest {
                    model: mi.id.to_string(),
                    messages: vec![Message {
                        role: Role::User,
                        content: MessageContent::Text(summary_prompt),
                    }],
                    tools: vec![],
                    max_tokens: Some(4096),
                    temperature: Some(0.0),
                    system: None,
                    stream: false,
                    thinking: None,
                };
                if let Ok(resp) = provider.chat(&summary_request).await {
                    let mut summary_text = resp.message.content.as_text().to_string();
                    if let Some(ref hist_path) = history_ref {
                        summary_text.push_str(&format!(
                            "\n\n[Full conversation history saved to {}. Use grep or read_file to search for details.]",
                            hist_path.display()
                        ));
                    }

                    let todo_summary = if let Some(ref store) = ctx.todo_store {
                        crate::tools::todo_incomplete_summary(store, &ctx.session_id).await
                    } else {
                        None
                    };
                    let plan_content = {
                        let plan_dir = ctx.project_root.join(".nyzhi").join("plans");
                        if plan_dir.exists() {
                            std::fs::read_dir(&plan_dir).ok().and_then(|entries| {
                                entries
                                    .filter_map(|e| e.ok())
                                    .filter(|e| {
                                        e.path().extension().map_or(false, |ext| ext == "md")
                                    })
                                    .max_by_key(|e| {
                                        e.metadata().ok().and_then(|m| m.modified().ok())
                                    })
                                    .and_then(|e| std::fs::read_to_string(e.path()).ok())
                            })
                        } else {
                            None
                        }
                    };

                    let notepad_content = {
                        let notepad_path = ctx.project_root.join(".nyzhi").join("notepad.md");
                        std::fs::read_to_string(&notepad_path).ok()
                    };

                    let keep_recent = if thread.message_count() > 20 { 6 } else { 4 };
                    thread.compact_with_rehydration(
                        &summary_text,
                        keep_recent,
                        &recent_files,
                        todo_summary.as_deref(),
                        plan_content.as_deref(),
                        notepad_content.as_deref(),
                    );

                    let new_est = thread.estimated_tokens(&system_prompt);
                    let _ = event_tx.send(AgentEvent::SystemMessage(format!(
                        "Full compaction complete: {} → {} tokens ({} messages kept)",
                        format_tokens(est_after),
                        format_tokens(new_est),
                        thread.message_count()
                    )));
                }
            }
        }

        let model_id = model_info.map(|m| m.id.to_string()).unwrap_or_default();

        let thinking = if config.thinking_enabled {
            Some(nyzhi_provider::ThinkingConfig {
                enabled: true,
                budget_tokens: config.thinking_budget,
                reasoning_effort: config.reasoning_effort.clone(),
                thinking_level: config.thinking_level.clone(),
            })
        } else {
            None
        };

        let request = ChatRequest {
            model: model_id.clone(),
            messages: thread.messages().to_vec(),
            tools: tool_defs.clone(),
            max_tokens,
            temperature: None,
            system: Some(system_prompt.clone()),
            stream: true,
            thinking,
        };

        let mut stream_attempt = 0u32;
        let acc = 'stream_retry: loop {
            let mut stream = match provider.chat_stream(&request).await {
                Ok(s) => s,
                Err(e) => {
                    if let Some(pe) = e.downcast_ref::<ProviderError>() {
                        if pe.is_retryable() && stream_attempt < config.retry.max_retries {
                            stream_attempt += 1;
                            let wait = pe
                                .retry_after_ms()
                                .unwrap_or_else(|| {
                                    config
                                        .retry
                                        .initial_backoff_ms
                                        .saturating_mul(2u64.saturating_pow(stream_attempt - 1))
                                })
                                .min(config.retry.max_backoff_ms);
                            let _ = event_tx.send(AgentEvent::Retrying {
                                attempt: stream_attempt,
                                max_retries: config.retry.max_retries,
                                wait_ms: wait,
                                reason: pe.to_string(),
                            });
                            tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                            continue 'stream_retry;
                        }
                    }
                    return Err(e);
                }
            };

            let mut acc = StreamAccumulator::new();
            let mut stream_err: Option<anyhow::Error> = None;

            while let Some(event) = stream.next().await {
                let event = match event {
                    Ok(ev) => ev,
                    Err(e) => {
                        stream_err = Some(e);
                        break;
                    }
                };
                acc.process(&event);

                match &event {
                    StreamEvent::ThinkingDelta(text) => {
                        let _ = event_tx.send(AgentEvent::ThinkingDelta(text.clone()));
                    }
                    StreamEvent::TextDelta(text) => {
                        let _ = event_tx.send(AgentEvent::TextDelta(text.clone()));
                    }
                    StreamEvent::ToolCallStart { id, name, .. } => {
                        let _ = event_tx.send(AgentEvent::ToolCallStart {
                            id: id.clone(),
                            name: name.clone(),
                        });
                    }
                    StreamEvent::ToolCallDelta {
                        arguments_delta, ..
                    } => {
                        if let Some(tc) = acc.tool_calls.last() {
                            let _ = event_tx.send(AgentEvent::ToolCallDelta {
                                id: tc.id.clone(),
                                args_delta: arguments_delta.clone(),
                            });
                        }
                    }
                    StreamEvent::Error(e) => {
                        let _ = event_tx.send(AgentEvent::Error(e.clone()));
                        return Ok(());
                    }
                    _ => {}
                }
            }

            if let Some(e) = stream_err {
                if let Some(pe) = e.downcast_ref::<ProviderError>() {
                    if pe.is_retryable() && stream_attempt < config.retry.max_retries {
                        stream_attempt += 1;
                        let wait = pe
                            .retry_after_ms()
                            .unwrap_or_else(|| {
                                config
                                    .retry
                                    .initial_backoff_ms
                                    .saturating_mul(2u64.saturating_pow(stream_attempt - 1))
                            })
                            .min(config.retry.max_backoff_ms);
                        let _ = event_tx.send(AgentEvent::Retrying {
                            attempt: stream_attempt,
                            max_retries: config.retry.max_retries,
                            wait_ms: wait,
                            reason: pe.to_string(),
                        });
                        tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
                        continue 'stream_retry;
                    }
                }
                return Err(e);
            }

            break acc;
        };

        if let Some(usage) = &acc.usage {
            session_usage.turn_input_tokens = session_usage
                .turn_input_tokens
                .saturating_add(usage.input_tokens);
            session_usage.turn_output_tokens = session_usage
                .turn_output_tokens
                .saturating_add(usage.output_tokens);
            session_usage.turn_cache_read_tokens = session_usage
                .turn_cache_read_tokens
                .saturating_add(usage.cache_read_tokens);
            session_usage.turn_cache_creation_tokens = session_usage
                .turn_cache_creation_tokens
                .saturating_add(usage.cache_creation_tokens);
            session_usage.total_input_tokens += usage.input_tokens as u64;
            session_usage.total_output_tokens += usage.output_tokens as u64;
            session_usage.total_cache_read_tokens += usage.cache_read_tokens as u64;
            session_usage.total_cache_creation_tokens += usage.cache_creation_tokens as u64;

            if let Some(mi) = model_info {
                let step_cost = mi.cost_usd(usage);
                session_usage.turn_cost_usd += step_cost;
                session_usage.total_cost_usd += step_cost;
            }

            let _ = event_tx.send(AgentEvent::Usage(session_usage.clone()));
        }

        if acc.has_tool_calls() {
            let mut tool_use_parts = Vec::new();
            let mut indexed_results: Vec<(usize, String)> =
                Vec::with_capacity(acc.tool_calls.len());

            let mut parallel_indices = Vec::new();
            let mut sequential_indices = Vec::new();

            for (i, tc) in acc.tool_calls.iter().enumerate() {
                let args: serde_json::Value =
                    serde_json::from_str(&tc.arguments).unwrap_or(serde_json::Value::Null);

                tool_use_parts.push(ContentPart::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: args.clone(),
                });

                let is_readonly = registry
                    .get(&tc.name)
                    .map(|t| t.permission() == ToolPermission::ReadOnly)
                    .unwrap_or(false);

                if is_readonly {
                    parallel_indices.push((i, args));
                } else {
                    sequential_indices.push((i, args));
                }
            }

            if !parallel_indices.is_empty() {
                let futs = parallel_indices.iter().map(|(i, args)| {
                    let i = *i;
                    let name = acc.tool_calls[i].name.clone();
                    let args = args.clone();
                    async move {
                        let start = std::time::Instant::now();
                        let output = match registry.execute(&name, args, ctx).await {
                            Ok(r) => r.output,
                            Err(e) => format!("Error executing tool: {e}"),
                        };
                        let elapsed_ms = start.elapsed().as_millis() as u64;
                        (i, output, elapsed_ms)
                    }
                });
                let results = futures::future::join_all(futs).await;
                for (i, output, elapsed_ms) in results {
                    let _ = event_tx.send(AgentEvent::ToolCallDone {
                        id: acc.tool_calls[i].id.clone(),
                        name: acc.tool_calls[i].name.clone(),
                        output: output.clone(),
                        elapsed_ms,
                    });
                    indexed_results.push((i, output));
                }
            }

            for (i, args) in sequential_indices {
                let tc = &acc.tool_calls[i];
                let start = std::time::Instant::now();
                let output = match execute_with_permission(
                    registry,
                    &tc.name,
                    args,
                    ctx,
                    event_tx,
                    &config.trust,
                    config.plan_mode,
                )
                .await
                {
                    Ok(r) => r.output,
                    Err(e) => format!("Error executing tool: {e}"),
                };
                let elapsed_ms = start.elapsed().as_millis() as u64;

                let _ = event_tx.send(AgentEvent::ToolCallDone {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    output: output.clone(),
                    elapsed_ms,
                });
                indexed_results.push((i, output));
            }

            indexed_results.sort_by_key(|(i, _)| *i);

            // Offload large tool results to files (Cursor files-for-everything pattern)
            let tool_result_parts: Vec<ContentPart> = indexed_results
                .into_iter()
                .map(|(i, output)| {
                    let tc = &acc.tool_calls[i];
                    // Auto-expand deferred tools on first use
                    if registry.is_deferred(&tc.name) {
                        // Safety: we can't mutate registry here since it's &, but the
                        // expansion is tracked via the agent loop re-building tool_defs.
                        // The tool executed successfully, so it exists.
                    }
                    let final_output = crate::context::offload_tool_result_to_file(
                        &tc.name,
                        &tc.id,
                        &output,
                        &context_dir,
                    )
                    .unwrap_or(output);
                    ContentPart::ToolResult {
                        tool_use_id: tc.id.clone(),
                        content: final_output,
                    }
                })
                .collect();

            thread.push_message(Message {
                role: Role::Assistant,
                content: MessageContent::Parts(tool_use_parts),
            });
            thread.push_message(Message {
                role: Role::User,
                content: MessageContent::Parts(tool_result_parts),
            });
        } else {
            if !acc.text.is_empty() {
                thread.push_message(Message {
                    role: Role::Assistant,
                    content: MessageContent::Text(acc.text),
                });
            }
            break;
        }

        if step + 1 >= config.max_steps {
            let _ = event_tx.send(AgentEvent::Error(
                "Reached maximum tool-calling steps".to_string(),
            ));
            break;
        }
    }

    // Auto-send idle notification if this agent is a teammate (not lead)
    if let (Some(team), Some(agent)) = (&ctx.team_name, &ctx.agent_name) {
        if !ctx.is_team_lead {
            if let Ok(config) = crate::teams::config::TeamConfig::load(team) {
                let lead_name = config.lead_name();
                let payload = crate::teams::mailbox::MessagePayload {
                    msg_type: crate::teams::mailbox::MessageType::IdleNotification,
                    data: serde_json::json!({
                        "teammate_name": agent,
                        "team_name": team,
                    }),
                };
                let msg = crate::teams::mailbox::TeamMessage::with_payload(agent, &payload, None);
                let _ = crate::teams::mailbox::send_message(team, &lead_name, msg);
            }
        }
    }

    let _ = event_tx.send(AgentEvent::TurnComplete);
    Ok(())
}

async fn execute_with_permission(
    registry: &ToolRegistry,
    tool_name: &str,
    args: serde_json::Value,
    ctx: &ToolContext,
    event_tx: &broadcast::Sender<AgentEvent>,
    trust: &TrustConfig,
    plan_mode: bool,
) -> Result<crate::tools::ToolResult> {
    let tool = registry
        .get(tool_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown tool: {tool_name}"))?;

    if plan_mode && tool.permission() == ToolPermission::NeedsApproval {
        return Ok(crate::tools::ToolResult {
            output: format!(
                "Tool `{tool_name}` is blocked in Plan Mode. Switch to Act mode (Shift+Tab) to make changes."
            ),
            title: format!("{tool_name} (plan mode)"),
            metadata: serde_json::json!({ "denied": true, "reason": "plan_mode" }),
        });
    }

    let target_path = extract_target_path(tool_name, &args);
    if crate::tools::permission::check_deny(tool_name, target_path.as_deref(), trust) {
        return Ok(crate::tools::ToolResult {
            output: format!("Tool `{tool_name}` is blocked by deny rules"),
            title: format!("{tool_name} (blocked)"),
            metadata: serde_json::json!({ "denied": true, "reason": "deny_rule" }),
        });
    }

    if tool.permission() == ToolPermission::NeedsApproval
        && !should_auto_approve(trust, tool_name, &args)
    {
        let args_summary = summarize_args(tool_name, &args).await;
        let (tx, rx) = tokio::sync::oneshot::channel();
        let respond = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));

        let _ = event_tx.send(AgentEvent::ApprovalRequest {
            tool_name: tool_name.to_string(),
            args_summary,
            respond,
        });

        let approved = rx.await.unwrap_or(false);
        if !approved {
            return Ok(crate::tools::ToolResult {
                output: "Tool execution denied by user".to_string(),
                title: format!("{tool_name} (denied)"),
                metadata: serde_json::json!({ "denied": true }),
            });
        }
    }

    registry.execute(tool_name, args, ctx).await
}

fn should_auto_approve(trust: &TrustConfig, tool_name: &str, args: &serde_json::Value) -> bool {
    match trust.mode {
        TrustMode::Full => true,
        TrustMode::Limited => {
            let tool_allowed =
                trust.allow_tools.is_empty() || trust.allow_tools.iter().any(|t| t == tool_name);
            if !tool_allowed {
                return false;
            }
            if trust.allow_paths.is_empty() {
                return true;
            }
            if let Some(path) = extract_target_path(tool_name, args) {
                trust.allow_paths.iter().any(|p| path.starts_with(p))
            } else {
                tool_allowed
            }
        }
        TrustMode::AutoEdit => {
            let write_tools = [
                "write",
                "edit",
                "multi_edit",
                "apply_patch",
                "delete_file",
                "move_file",
                "copy_file",
                "create_dir",
            ];
            let read_tools_auto =
                trust.allow_tools.is_empty() || trust.allow_tools.iter().any(|t| t == tool_name);
            if write_tools.contains(&tool_name) && read_tools_auto {
                true
            } else {
                let tool_allowed = trust.allow_tools.is_empty()
                    || trust.allow_tools.iter().any(|t| t == tool_name);
                if !tool_allowed {
                    return false;
                }
                false
            }
        }
        TrustMode::Off => false,
    }
}

fn extract_target_path(tool_name: &str, args: &serde_json::Value) -> Option<String> {
    match tool_name {
        "edit" | "multi_edit" | "write" | "read" | "delete_file" => args
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "move_file" | "copy_file" => args
            .get("destination")
            .or_else(|| args.get("source"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "create_dir" => args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "apply_patch" => args
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "bash" => args
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

async fn summarize_args(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "bash" => args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown command)")
            .to_string(),
        "edit" => summarize_edit_args(args).await,
        "write" => summarize_write_args(args).await,
        _ => serde_json::to_string(args).unwrap_or_default(),
    }
}

async fn summarize_edit_args(args: &serde_json::Value) -> String {
    let file_path = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown file)");
    let old_string = args.get("old_string").and_then(|v| v.as_str());
    let new_string = args.get("new_string").and_then(|v| v.as_str());

    let (Some(old_str), Some(new_str)) = (old_string, new_string) else {
        return file_path.to_string();
    };

    let Ok(content) = tokio::fs::read_to_string(file_path).await else {
        return file_path.to_string();
    };

    if content.matches(old_str).count() != 1 {
        return file_path.to_string();
    }

    let new_content = content.replacen(old_str, new_str, 1);
    let diff = crate::tools::diff::unified_diff(file_path, &content, &new_content, 3);
    let preview = crate::tools::diff::truncate_diff(&diff, 30);

    if preview.is_empty() {
        file_path.to_string()
    } else {
        format!("{file_path}\n{preview}")
    }
}

async fn summarize_write_args(args: &serde_json::Value) -> String {
    let file_path = args
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("(unknown file)");
    let content = args.get("content").and_then(|v| v.as_str());

    let Some(new_content) = content else {
        return file_path.to_string();
    };

    if let Ok(old_content) = tokio::fs::read_to_string(file_path).await {
        let diff = crate::tools::diff::unified_diff(file_path, &old_content, new_content, 3);
        let preview = crate::tools::diff::truncate_diff(&diff, 30);
        if preview.is_empty() {
            file_path.to_string()
        } else {
            format!("{file_path}\n{preview}")
        }
    } else {
        let lines: Vec<&str> = new_content.lines().take(15).collect();
        let preview = lines.join("\n");
        let total = new_content.lines().count();
        if total > 15 {
            format!(
                "{file_path} (new file, {total} lines)\n{preview}\n... ({} more lines)",
                total - 15
            )
        } else {
            format!("{file_path} (new file)\n{preview}")
        }
    }
}

fn format_tokens(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
