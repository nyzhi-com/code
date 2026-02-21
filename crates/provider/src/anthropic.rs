use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const API_VERSION: &str = "2023-06-01";

static MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "claude-sonnet-4-20250514",
        name: "Claude Sonnet 4",
        context_window: 200_000,
        max_output_tokens: 16_384,
        supports_tools: true,
        supports_streaming: true,
        supports_vision: true,
        input_price_per_m: 3.0,
        output_price_per_m: 15.0,
    },
    ModelInfo {
        id: "claude-opus-4-20250514",
        name: "Claude Opus 4",
        context_window: 200_000,
        max_output_tokens: 32_768,
        supports_tools: true,
        supports_streaming: true,
        supports_vision: true,
        input_price_per_m: 15.0,
        output_price_per_m: 75.0,
    },
    ModelInfo {
        id: "claude-haiku-3-5-20241022",
        name: "Claude 3.5 Haiku",
        context_window: 200_000,
        max_output_tokens: 8_192,
        supports_tools: true,
        supports_streaming: true,
        supports_vision: true,
        input_price_per_m: 0.8,
        output_price_per_m: 4.0,
    },
];

pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            api_key,
            default_model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }

    pub fn from_config(config: &nyzhi_config::Config) -> Result<Self> {
        let cred = nyzhi_auth::resolve_credential(
            "anthropic",
            config.provider.anthropic.api_key.as_deref(),
        )?;
        Ok(Self::new(
            cred.header_value(),
            config.provider.anthropic.base_url.clone(),
            config.provider.anthropic.model.clone(),
        ))
    }

    fn build_messages(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|msg| {
                let content = match &msg.content {
                    MessageContent::Text(text) => json!(text),
                    MessageContent::Parts(parts) => {
                        let content: Vec<serde_json::Value> = parts
                            .iter()
                            .map(|p| match p {
                                ContentPart::Text { text } => {
                                    json!({"type": "text", "text": text})
                                }
                                ContentPart::Image { media_type, data } => json!({
                                    "type": "image",
                                    "source": {
                                        "type": "base64",
                                        "media_type": media_type,
                                        "data": data,
                                    }
                                }),
                                ContentPart::ToolUse { id, name, input } => json!({
                                    "type": "tool_use",
                                    "id": id,
                                    "name": name,
                                    "input": input,
                                }),
                                ContentPart::ToolResult {
                                    tool_use_id,
                                    content,
                                } => json!({
                                    "type": "tool_result",
                                    "tool_use_id": tool_use_id,
                                    "content": content,
                                }),
                            })
                            .collect();
                        json!(content)
                    }
                };
                json!({
                    "role": match msg.role {
                        Role::Assistant => "assistant",
                        _ => "user",
                    },
                    "content": content,
                })
            })
            .collect()
    }

    fn build_tools(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            })
            .collect()
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
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
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(system) = &request.system {
            body["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(self.build_tools(&request.tools));
        }

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp.text().await.unwrap_or_default();
            return Err(
                ProviderError::from_http(status.as_u16(), body, retry_after.as_deref()).into(),
            );
        }

        let data: serde_json::Value = resp.json().await?;
        let content = data["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: MessageContent::Text(content),
            },
            usage: Some(Usage {
                input_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            }),
            finish_reason: data["stop_reason"].as_str().map(String::from),
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
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(system) = &request.system {
            body["system"] = json!(system);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(self.build_tools(&request.tools));
        }

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp.text().await.unwrap_or_default();
            return Err(
                ProviderError::from_http(status.as_u16(), body, retry_after.as_deref()).into(),
            );
        }

        let sse_stream = parse_sse_stream(resp);

        let event_stream = sse_stream.flat_map(|result| {
            let events: Vec<Result<StreamEvent>> = match result {
                Ok(sse) => {
                    let data: serde_json::Value = match serde_json::from_str(&sse.data) {
                        Ok(v) => v,
                        Err(_) => return futures::stream::iter(vec![]),
                    };
                    let event_type = sse.event.as_deref().unwrap_or("");

                    match event_type {
                        "message_start" => {
                            let input = data["message"]["usage"]["input_tokens"]
                                .as_u64()
                                .unwrap_or(0) as u32;
                            if input > 0 {
                                vec![Ok(StreamEvent::Usage(Usage {
                                    input_tokens: input,
                                    output_tokens: 0,
                                }))]
                            } else {
                                vec![]
                            }
                        }
                        "content_block_delta" => {
                            let delta = &data["delta"];
                            if delta["type"] == "text_delta" {
                                vec![Ok(StreamEvent::TextDelta(
                                    delta["text"].as_str().unwrap_or("").to_string(),
                                ))]
                            } else if delta["type"] == "input_json_delta" {
                                vec![Ok(StreamEvent::ToolCallDelta {
                                    index: data["index"].as_u64().unwrap_or(0) as u32,
                                    arguments_delta: delta["partial_json"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                })]
                            } else {
                                vec![]
                            }
                        }
                        "content_block_start" => {
                            let block = &data["content_block"];
                            if block["type"] == "tool_use" {
                                vec![Ok(StreamEvent::ToolCallStart {
                                    index: data["index"].as_u64().unwrap_or(0) as u32,
                                    id: block["id"].as_str().unwrap_or("").to_string(),
                                    name: block["name"].as_str().unwrap_or("").to_string(),
                                })]
                            } else {
                                vec![]
                            }
                        }
                        "message_delta" => {
                            let mut evts = Vec::new();
                            let output = data["usage"]["output_tokens"]
                                .as_u64()
                                .unwrap_or(0) as u32;
                            if output > 0 {
                                evts.push(Ok(StreamEvent::Usage(Usage {
                                    input_tokens: 0,
                                    output_tokens: output,
                                })));
                            }
                            if data["delta"]["stop_reason"].is_string() {
                                evts.push(Ok(StreamEvent::Done));
                            }
                            evts
                        }
                        "message_stop" => vec![Ok(StreamEvent::Done)],
                        _ => vec![],
                    }
                }
                Err(e) => vec![Err(e)],
            };
            futures::stream::iter(events)
        });

        Ok(Box::pin(event_stream))
    }
}
