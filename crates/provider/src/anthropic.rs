use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";
const DEFAULT_MODEL: &str = "claude-sonnet-4-6-20260217";
const API_VERSION: &str = "2023-06-01";

pub fn default_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-opus-4-6-20260205".into(),
            name: "Claude Opus 4.6".into(),
            provider: "anthropic".into(),
            context_window: 1_000_000,
            max_output_tokens: 128_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 15.0,
            output_price_per_m: 75.0,
            cache_read_price_per_m: 1.5,
            cache_write_price_per_m: 18.75,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::anthropic_adaptive()),
        },
        ModelInfo {
            id: "claude-sonnet-4-6-20260217".into(),
            name: "Claude Sonnet 4.6".into(),
            provider: "anthropic".into(),
            context_window: 1_000_000,
            max_output_tokens: 16_384,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 3.0,
            output_price_per_m: 15.0,
            cache_read_price_per_m: 0.3,
            cache_write_price_per_m: 3.75,
            tier: ModelTier::Medium,
            thinking: Some(ThinkingSupport::anthropic_adaptive()),
        },
        ModelInfo {
            id: "claude-haiku-4-5-20251022".into(),
            name: "Claude Haiku 4.5".into(),
            provider: "anthropic".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.8,
            output_price_per_m: 4.0,
            cache_read_price_per_m: 0.08,
            cache_write_price_per_m: 1.0,
            tier: ModelTier::Low,
            thinking: None,
        },
    ]
}

pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
    models: Vec<ModelInfo>,
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
            models: default_models(),
        }
    }

    pub fn with_models(mut self, models: Vec<ModelInfo>) -> Self {
        if !models.is_empty() {
            self.models = models;
        }
        self
    }

    pub fn from_config(config: &nyzhi_config::Config) -> Result<Self> {
        let entry = config.provider.entry("anthropic");
        let cred =
            nyzhi_auth::resolve_credential("anthropic", entry.and_then(|e| e.api_key.as_deref()))?;
        Ok(Self::new(
            cred.header_value(),
            entry.and_then(|e| e.base_url.clone()),
            entry.and_then(|e| e.model.clone()),
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
                                    ..
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
        let len = tools.len();
        tools
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let mut tool = json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                });
                if i == len - 1 {
                    tool["cache_control"] = json!({"type": "ephemeral"});
                }
                tool
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
        &self.models
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
            body["system"] = json!([{
                "type": "text",
                "text": system,
                "cache_control": {"type": "ephemeral"}
            }]);
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
        let msg_content = if let Some(blocks) = data["content"].as_array() {
            let mut parts = Vec::new();
            for b in blocks {
                match b["type"].as_str() {
                    Some("text") => {
                        if let Some(t) = b["text"].as_str() {
                            if !t.is_empty() {
                                parts.push(ContentPart::Text {
                                    text: t.to_string(),
                                });
                            }
                        }
                    }
                    Some("tool_use") => {
                        parts.push(ContentPart::ToolUse {
                            id: b["id"].as_str().unwrap_or("").to_string(),
                            name: b["name"].as_str().unwrap_or("").to_string(),
                            input: b["input"].clone(),
                        });
                    }
                    _ => {}
                }
            }
            if parts.len() == 1 {
                if let ContentPart::Text { text } = &parts[0] {
                    MessageContent::Text(text.clone())
                } else {
                    MessageContent::Parts(parts)
                }
            } else if parts.is_empty() {
                MessageContent::Text(String::new())
            } else {
                MessageContent::Parts(parts)
            }
        } else {
            MessageContent::Text(String::new())
        };

        let cache_read = data["usage"]["cache_read_input_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;
        let cache_creation = data["usage"]["cache_creation_input_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;
        let uncached = data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: msg_content,
            },
            usage: Some(Usage {
                input_tokens: uncached + cache_read + cache_creation,
                output_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
                cache_read_tokens: cache_read,
                cache_creation_tokens: cache_creation,
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

        let thinking_enabled = request
            .thinking
            .as_ref()
            .map(|t| t.enabled)
            .unwrap_or(false);

        let max_tokens = if thinking_enabled {
            request.max_tokens.unwrap_or(16_384).max(8192)
        } else {
            request.max_tokens.unwrap_or(4096)
        };

        let mut body = json!({
            "model": model,
            "messages": self.build_messages(request),
            "max_tokens": max_tokens,
            "stream": true,
        });

        if thinking_enabled {
            let is_adaptive_model = model.contains("opus-4-6") || model.contains("sonnet-4-6");
            if is_adaptive_model {
                let effort = request
                    .thinking
                    .as_ref()
                    .and_then(|t| t.thinking_level.as_deref())
                    .unwrap_or("high");
                body["thinking"] = json!({
                    "type": "adaptive",
                    "effort": effort
                });
            } else {
                let budget = request
                    .thinking
                    .as_ref()
                    .and_then(|t| t.budget_tokens)
                    .unwrap_or(10_000);
                body["thinking"] = json!({
                    "type": "enabled",
                    "budget_tokens": budget
                });
            }
            body.as_object_mut().unwrap().remove("temperature");
        }

        if let Some(system) = &request.system {
            body["system"] = json!([{
                "type": "text",
                "text": system,
                "cache_control": {"type": "ephemeral"}
            }]);
        }
        if !thinking_enabled {
            if let Some(temp) = request.temperature {
                body["temperature"] = json!(temp);
            }
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
                            let usage = &data["message"]["usage"];
                            let uncached = usage["input_tokens"].as_u64().unwrap_or(0) as u32;
                            let cache_read =
                                usage["cache_read_input_tokens"].as_u64().unwrap_or(0) as u32;
                            let cache_creation =
                                usage["cache_creation_input_tokens"].as_u64().unwrap_or(0) as u32;
                            let total = uncached + cache_read + cache_creation;
                            if total > 0 {
                                vec![Ok(StreamEvent::Usage(Usage {
                                    input_tokens: total,
                                    output_tokens: 0,
                                    cache_read_tokens: cache_read,
                                    cache_creation_tokens: cache_creation,
                                }))]
                            } else {
                                vec![]
                            }
                        }
                        "content_block_delta" => {
                            let delta = &data["delta"];
                            if delta["type"] == "thinking_delta" {
                                vec![Ok(StreamEvent::ThinkingDelta(
                                    delta["thinking"].as_str().unwrap_or("").to_string(),
                                ))]
                            } else if delta["type"] == "text_delta" {
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
                            let output =
                                data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
                            if output > 0 {
                                evts.push(Ok(StreamEvent::Usage(Usage {
                                    input_tokens: 0,
                                    output_tokens: output,
                                    ..Default::default()
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
