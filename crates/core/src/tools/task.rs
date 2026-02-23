use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::agent::{run_turn, AgentConfig, AgentEvent, SessionUsage};
use crate::conversation::Thread;

use super::{Tool, ToolContext, ToolResult, ToolRegistry};

pub struct TaskTool {
    provider: Arc<dyn nyzhi_provider::Provider>,
    registry: Arc<ToolRegistry>,
    max_depth: u32,
}

impl TaskTool {
    pub fn new(
        provider: Arc<dyn nyzhi_provider::Provider>,
        registry: Arc<ToolRegistry>,
        max_depth: u32,
    ) -> Self {
        Self {
            provider,
            registry,
            max_depth,
        }
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "task"
    }

    fn description(&self) -> &str {
        "Delegate a sub-task to a child agent. The sub-agent has access to all the same tools and runs independently. Use for research, analysis, or implementation tasks that benefit from focused attention."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The sub-task description for the child agent"
                },
                "context": {
                    "type": "string",
                    "description": "Additional context to provide to the child agent"
                }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, args: Value, ctx: &ToolContext) -> Result<ToolResult> {
        if ctx.depth >= self.max_depth {
            return Ok(ToolResult {
                output: format!(
                    "Cannot spawn sub-agent: maximum depth ({}) reached",
                    self.max_depth
                ),
                title: "task (depth limit)".to_string(),
                metadata: json!({ "error": "max_depth_exceeded", "depth": ctx.depth }),
            });
        }

        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("task tool requires 'prompt' parameter"))?;

        let extra_context = args.get("context").and_then(|v| v.as_str()).unwrap_or("");

        let full_prompt = if extra_context.is_empty() {
            prompt.to_string()
        } else {
            format!("{prompt}\n\nAdditional context:\n{extra_context}")
        };

        let agent_config = AgentConfig {
            name: format!("sub-task-d{}", ctx.depth + 1),
            system_prompt: "You are a focused sub-agent. Complete the assigned task thoroughly \
                and return your findings. Be concise but complete. You have access to all \
                standard tools."
                .to_string(),
            max_steps: 50,
            max_tokens: None,
            trust: nyzhi_config::TrustConfig::default(),
            retry: nyzhi_config::RetrySettings::default(),
            routing: nyzhi_config::RoutingConfig::default(),
            auto_compact_threshold: None,
            compact_instructions: None,
            thinking_enabled: false,
            thinking_budget: None,
            reasoning_effort: None,
            thinking_level: None,
            team_name: ctx.team_name.clone(),
            agent_name: ctx.agent_name.clone(),
            plan_mode: false,
            act_after_plan: false,
        };

        let mut child_thread = Thread::new();
        let (child_tx, mut child_rx) = broadcast::channel::<AgentEvent>(256);

        let parent_tx = ctx.event_tx.clone();
        let forward_handle = tokio::spawn(async move {
            if let Some(parent) = parent_tx {
                while let Ok(event) = child_rx.recv().await {
                    let forwarded = match event {
                        AgentEvent::TextDelta(text) => {
                            AgentEvent::TextDelta(format!("[sub-task] {text}"))
                        }
                        AgentEvent::ToolCallStart { id, name } => AgentEvent::ToolCallStart {
                            id,
                            name: format!("[sub-task] {name}"),
                        },
                        AgentEvent::ToolCallDone {
                            id,
                            name,
                            output,
                            elapsed_ms,
                        } => AgentEvent::ToolCallDone {
                            id,
                            name: format!("[sub-task] {name}"),
                            output,
                            elapsed_ms,
                        },
                        AgentEvent::TurnComplete => break,
                        other => other,
                    };
                    let _ = parent.send(forwarded);
                }
            } else {
                while let Ok(ev) = child_rx.recv().await {
                    if matches!(ev, AgentEvent::TurnComplete) {
                        break;
                    }
                }
            }
        });

        let child_ctx = ToolContext {
            session_id: ctx.session_id.clone(),
            cwd: ctx.cwd.clone(),
            project_root: ctx.project_root.clone(),
            depth: ctx.depth + 1,
            event_tx: Some(child_tx.clone()),
            change_tracker: ctx.change_tracker.clone(),
            allowed_tool_names: None,
            team_name: ctx.team_name.clone(),
            agent_name: ctx.agent_name.clone(),
            is_team_lead: ctx.is_team_lead,
            todo_store: ctx.todo_store.clone(),
        };

        let mut session_usage = SessionUsage::default();

        let result = run_turn(
            &*self.provider,
            &mut child_thread,
            &full_prompt,
            &agent_config,
            &child_tx,
            &self.registry,
            &child_ctx,
            None,
            &mut session_usage,
        )
        .await;

        let _ = forward_handle.await;

        match result {
            Ok(()) => {
                let final_text = child_thread
                    .messages()
                    .iter()
                    .rev()
                    .find(|m| m.role == nyzhi_provider::Role::Assistant)
                    .map(|m| m.content.as_text().to_string())
                    .unwrap_or_else(|| "Sub-agent completed but produced no output.".to_string());

                Ok(ToolResult {
                    output: final_text,
                    title: "task".to_string(),
                    metadata: json!({
                        "depth": ctx.depth + 1,
                        "input_tokens": session_usage.total_input_tokens,
                        "output_tokens": session_usage.total_output_tokens,
                    }),
                })
            }
            Err(e) => Ok(ToolResult {
                output: format!("Sub-agent failed: {e}"),
                title: "task (error)".to_string(),
                metadata: json!({ "error": e.to_string() }),
            }),
        }
    }
}
