use anyhow::Result;
use futures::StreamExt;
use nyzhi_provider::{ChatRequest, Message, MessageContent, Provider, Role, StreamEvent};
use tokio::sync::broadcast;

use crate::conversation::Thread;

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextDelta(String),
    ToolCallStart { id: String, name: String },
    ToolCallDelta { id: String, args_delta: String },
    ToolCallDone { id: String },
    TurnComplete,
    Error(String),
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
) -> Result<()> {
    thread.push_message(Message {
        role: Role::User,
        content: MessageContent::Text(user_input.to_string()),
    });

    let request = ChatRequest {
        model: String::new(),
        messages: thread.messages().to_vec(),
        tools: Vec::new(),
        max_tokens: Some(4096),
        temperature: None,
        system: Some(config.system_prompt.clone()),
        stream: true,
    };

    let mut stream = provider.chat_stream(&request).await?;
    let mut full_response = String::new();

    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::TextDelta(text) => {
                full_response.push_str(&text);
                let _ = event_tx.send(AgentEvent::TextDelta(text));
            }
            StreamEvent::Done => {
                break;
            }
            StreamEvent::Error(e) => {
                let _ = event_tx.send(AgentEvent::Error(e));
                break;
            }
            _ => {}
        }
    }

    if !full_response.is_empty() {
        thread.push_message(Message {
            role: Role::Assistant,
            content: MessageContent::Text(full_response),
        });
    }

    let _ = event_tx.send(AgentEvent::TurnComplete);
    Ok(())
}
