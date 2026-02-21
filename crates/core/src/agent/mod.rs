use anyhow::Result;
use futures::StreamExt;
use nyzhi_provider::{
    ChatRequest, ContentPart, Message, MessageContent, Provider, Role, StreamEvent,
};
use tokio::sync::broadcast;

use crate::conversation::Thread;
use crate::streaming::StreamAccumulator;
use crate::tools::permission::ToolPermission;
use crate::tools::{ToolContext, ToolRegistry};

#[derive(Clone)]
pub enum AgentEvent {
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
    },
    ApprovalRequest {
        tool_name: String,
        args_summary: String,
        respond: std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<bool>>>>,
    },
    TurnComplete,
    Error(String),
}

impl std::fmt::Debug for AgentEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            Self::ToolCallDone { id, name, output } => f
                .debug_struct("ToolCallDone")
                .field("id", id)
                .field("name", name)
                .field("output", output)
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
            Self::TurnComplete => write!(f, "TurnComplete"),
            Self::Error(s) => f.debug_tuple("Error").field(s).finish(),
        }
    }
}

pub struct AgentConfig {
    pub name: String,
    pub system_prompt: String,
    pub max_steps: u32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "build".to_string(),
            system_prompt: crate::prompt::default_system_prompt(),
            max_steps: 100,
        }
    }
}

pub async fn run_turn(
    provider: &dyn Provider,
    thread: &mut Thread,
    user_input: &str,
    config: &AgentConfig,
    event_tx: &broadcast::Sender<AgentEvent>,
    registry: &ToolRegistry,
    ctx: &ToolContext,
) -> Result<()> {
    thread.push_message(Message {
        role: Role::User,
        content: MessageContent::Text(user_input.to_string()),
    });

    let tool_defs = registry.definitions();

    for step in 0..config.max_steps {
        let request = ChatRequest {
            model: String::new(),
            messages: thread.messages().to_vec(),
            tools: tool_defs.clone(),
            max_tokens: Some(16384),
            temperature: None,
            system: Some(config.system_prompt.clone()),
            stream: true,
        };

        let mut stream = provider.chat_stream(&request).await?;
        let mut acc = StreamAccumulator::new();

        while let Some(event) = stream.next().await {
            let event = event?;
            acc.process(&event);

            match &event {
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

        if acc.has_tool_calls() {
            let mut tool_use_parts = Vec::new();
            let mut tool_result_parts = Vec::new();

            for tc in &acc.tool_calls {
                let args: serde_json::Value =
                    serde_json::from_str(&tc.arguments).unwrap_or(serde_json::Value::Null);

                tool_use_parts.push(ContentPart::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: args.clone(),
                });

                let output = match execute_with_permission(
                    registry, &tc.name, args, ctx, event_tx,
                )
                .await
                {
                    Ok(result) => result.output,
                    Err(e) => format!("Error executing tool: {e}"),
                };

                let _ = event_tx.send(AgentEvent::ToolCallDone {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    output: output.clone(),
                });

                tool_result_parts.push(ContentPart::ToolResult {
                    tool_use_id: tc.id.clone(),
                    content: output,
                });
            }

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

    let _ = event_tx.send(AgentEvent::TurnComplete);
    Ok(())
}

async fn execute_with_permission(
    registry: &ToolRegistry,
    tool_name: &str,
    args: serde_json::Value,
    ctx: &ToolContext,
    event_tx: &broadcast::Sender<AgentEvent>,
) -> Result<crate::tools::ToolResult> {
    let tool = registry
        .get(tool_name)
        .ok_or_else(|| anyhow::anyhow!("Unknown tool: {tool_name}"))?;

    if tool.permission() == ToolPermission::NeedsApproval {
        let args_summary = summarize_args(tool_name, &args);
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

fn summarize_args(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "bash" => args
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown command)")
            .to_string(),
        "write" => args
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown file)")
            .to_string(),
        "edit" => args
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("(unknown file)")
            .to_string(),
        _ => serde_json::to_string(args).unwrap_or_default(),
    }
}
