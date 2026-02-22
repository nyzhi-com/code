use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-5.3-codex";

pub fn default_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gpt-5.3-codex".into(), name: "GPT-5.3 Codex".into(), provider: "openai".into(),
            context_window: 272_000, max_output_tokens: 100_000,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 2.0, output_price_per_m: 8.0,
            cache_read_price_per_m: 0.5, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "gpt-5.2-codex".into(), name: "GPT-5.2 Codex".into(), provider: "openai".into(),
            context_window: 272_000, max_output_tokens: 100_000,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 2.0, output_price_per_m: 8.0,
            cache_read_price_per_m: 0.5, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "gpt-5.2".into(), name: "GPT-5.2".into(), provider: "openai".into(),
            context_window: 272_000, max_output_tokens: 100_000,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 2.0, output_price_per_m: 8.0,
            cache_read_price_per_m: 0.5, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "o3".into(), name: "o3".into(), provider: "openai".into(),
            context_window: 200_000, max_output_tokens: 100_000,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 2.0, output_price_per_m: 8.0,
            cache_read_price_per_m: 0.5, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "o4-mini".into(), name: "o4-mini".into(), provider: "openai".into(),
            context_window: 200_000, max_output_tokens: 100_000,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 1.1, output_price_per_m: 4.4,
            cache_read_price_per_m: 0.275, cache_write_price_per_m: 0.0,
            tier: ModelTier::Low, thinking: Some(ThinkingSupport::openai_reasoning()),
        },
    ]
}

pub struct OpenAIProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
    models: Vec<ModelInfo>,
}

impl OpenAIProvider {
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
        let entry = config.provider.entry("openai");
        let cred = nyzhi_auth::resolve_credential("openai", entry.and_then(|e| e.api_key.as_deref()))?;
        Ok(Self::new(
            cred.header_value(),
            entry.and_then(|e| e.base_url.clone()),
            entry.and_then(|e| e.model.clone()),
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
                            ContentPart::Image { media_type, data } => json!({
                                "type": "image_url",
                                "image_url": {
                                    "url": format!("data:{media_type};base64,{data}")
                                }
                            }),
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
        let choice = &data["choices"][0];
        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let cached = data["usage"]["prompt_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: MessageContent::Text(content),
            },
            usage: Some(Usage {
                input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
                cache_read_tokens: cached,
                cache_creation_tokens: 0,
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
            "stream_options": {"include_usage": true},
        });

        let thinking_enabled = request
            .thinking
            .as_ref()
            .map(|t| t.enabled)
            .unwrap_or(false);

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if !thinking_enabled {
            if let Some(temp) = request.temperature {
                body["temperature"] = json!(temp);
            }
        }
        if thinking_enabled {
            let is_kimi = model.starts_with("kimi-k2");
            if is_kimi {
                body["thinking"] = json!({"type": "enabled"});
                body["temperature"] = json!(1.0);
                body["top_p"] = json!(0.95);
            } else {
                let effort = request
                    .thinking
                    .as_ref()
                    .and_then(|t| t.reasoning_effort.as_deref())
                    .unwrap_or("high");
                body["reasoning_effort"] = json!(effort);
            }
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

        let event_stream = sse_stream.map(|result| {
            result.and_then(|sse| {
                let data: serde_json::Value = serde_json::from_str(&sse.data)?;

                if let Some(usage) = data.get("usage").filter(|u| u.is_object()) {
                    let cached = usage["prompt_tokens_details"]["cached_tokens"]
                        .as_u64()
                        .unwrap_or(0) as u32;
                    return Ok(StreamEvent::Usage(Usage {
                        input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                        output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0)
                            as u32,
                        cache_read_tokens: cached,
                        cache_creation_tokens: 0,
                    }));
                }

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
