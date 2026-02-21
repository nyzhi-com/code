use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4.1";

static MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "gpt-4.1",
        name: "GPT-4.1",
        context_window: 1_047_576,
        max_output_tokens: 32_768,
        supports_tools: true,
        supports_streaming: true,
    },
    ModelInfo {
        id: "gpt-4.1-mini",
        name: "GPT-4.1 Mini",
        context_window: 1_047_576,
        max_output_tokens: 32_768,
        supports_tools: true,
        supports_streaming: true,
    },
    ModelInfo {
        id: "o3",
        name: "o3",
        context_window: 200_000,
        max_output_tokens: 100_000,
        supports_tools: true,
        supports_streaming: true,
    },
    ModelInfo {
        id: "o4-mini",
        name: "o4-mini",
        context_window: 200_000,
        max_output_tokens: 100_000,
        supports_tools: true,
        supports_streaming: true,
    },
];

pub struct OpenAIProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            api_key,
            default_model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }

    pub fn from_config(config: &nyzhi_config::Config) -> Result<Self> {
        let cred = nyzhi_auth::resolve_credential("openai", config.provider.openai.api_key.as_deref())?;
        Ok(Self::new(
            cred.header_value(),
            config.provider.openai.base_url.clone(),
            config.provider.openai.model.clone(),
        ))
    }

    fn build_messages(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        let mut msgs = Vec::new();

        if let Some(system) = &request.system {
            msgs.push(json!({
                "role": "system",
                "content": system,
            }));
        }

        for msg in &request.messages {
            msgs.push(match &msg.content {
                MessageContent::Text(text) => json!({
                    "role": role_str(&msg.role),
                    "content": text,
                }),
                MessageContent::Parts(parts) => {
                    let content: Vec<serde_json::Value> = parts
                        .iter()
                        .map(|p| match p {
                            ContentPart::Text { text } => json!({"type": "text", "text": text}),
                            ContentPart::ToolUse { id, name, input } => json!({
                                "type": "function",
                                "id": id,
                                "function": {"name": name, "arguments": input.to_string()},
                            }),
                            ContentPart::ToolResult { tool_use_id, content } => json!({
                                "role": "tool",
                                "tool_call_id": tool_use_id,
                                "content": content,
                            }),
                        })
                        .collect();
                    json!({"role": role_str(&msg.role), "content": content})
                }
            });
        }

        msgs
    }

    fn build_tools(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect()
    }
}

fn role_str(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn supported_models(&self) -> &[ModelInfo] {
        MODELS
    }

    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        let mut body = json!({
            "model": model,
            "messages": self.build_messages(request),
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(self.build_tools(&request.tools));
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::HttpError {
                status: status.as_u16(),
                body,
            }
            .into());
        }

        let data: serde_json::Value = resp.json().await?;
        let choice = &data["choices"][0];
        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: MessageContent::Text(content),
            },
            usage: Some(Usage {
                input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            }),
            finish_reason: choice["finish_reason"].as_str().map(String::from),
        })
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent>>> {
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        let mut body = json!({
            "model": model,
            "messages": self.build_messages(request),
            "stream": true,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(self.build_tools(&request.tools));
        }

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::HttpError {
                status: status.as_u16(),
                body,
            }
            .into());
        }

        let sse_stream = parse_sse_stream(resp);

        let event_stream = sse_stream.map(|result| {
            result.and_then(|sse| {
                let data: serde_json::Value = serde_json::from_str(&sse.data)?;

                if data["choices"][0]["finish_reason"].is_string() {
                    return Ok(StreamEvent::Done);
                }

                let delta = &data["choices"][0]["delta"];

                if let Some(content) = delta["content"].as_str() {
                    return Ok(StreamEvent::TextDelta(content.to_string()));
                }

                if let Some(tool_calls) = delta["tool_calls"].as_array() {
                    for tc in tool_calls {
                        let index = tc["index"].as_u64().unwrap_or(0) as u32;
                        if let Some(function) = tc.get("function") {
                            if let Some(name) = function["name"].as_str() {
                                return Ok(StreamEvent::ToolCallStart {
                                    index,
                                    id: tc["id"].as_str().unwrap_or("").to_string(),
                                    name: name.to_string(),
                                });
                            }
                            if let Some(args) = function["arguments"].as_str() {
                                return Ok(StreamEvent::ToolCallDelta {
                                    index,
                                    arguments_delta: args.to_string(),
                                });
                            }
                        }
                    }
                }

                Ok(StreamEvent::TextDelta(String::new()))
            })
        });

        Ok(Box::pin(event_stream))
    }
}
