use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use async_trait::async_trait;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const BASE_URL: &str = "https://api2.cursor.sh";
const CLIENT_VERSION: &str = "0.50.7";
const DEFAULT_MODEL: &str = "claude-4.6-sonnet";

pub fn cursor_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "claude-4.6-sonnet".into(),
            name: "Claude 4.6 Sonnet".into(),
            provider: "cursor".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: None,
        },
        ModelInfo {
            id: "claude-4.6-opus".into(),
            name: "Claude 4.6 Opus".into(),
            provider: "cursor".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: None,
        },
        ModelInfo {
            id: "composer-1.5".into(),
            name: "Composer 1.5".into(),
            provider: "cursor".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: false,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: None,
        },
        ModelInfo {
            id: "gpt-5.3-codex".into(),
            name: "GPT-5.3 Codex".into(),
            provider: "cursor".into(),
            context_window: 272_000,
            max_output_tokens: 128_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "gpt-5.2".into(),
            name: "GPT-5.2".into(),
            provider: "cursor".into(),
            context_window: 272_000,
            max_output_tokens: 100_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium,
            thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "gemini-3.1-pro".into(),
            name: "Gemini 3.1 Pro".into(),
            provider: "cursor".into(),
            context_window: 1_048_576,
            max_output_tokens: 65_536,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: None,
        },
        ModelInfo {
            id: "gemini-3-flash".into(),
            name: "Gemini 3 Flash".into(),
            provider: "cursor".into(),
            context_window: 1_048_576,
            max_output_tokens: 65_536,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::Low,
            thinking: None,
        },
        ModelInfo {
            id: "grok-code".into(),
            name: "Grok Code".into(),
            provider: "cursor".into(),
            context_window: 256_000,
            max_output_tokens: 64_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: false,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium,
            thinking: None,
        },
        ModelInfo {
            id: "auto".into(),
            name: "Auto (server picks)".into(),
            provider: "cursor".into(),
            context_window: 200_000,
            max_output_tokens: 64_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 0.0,
            output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium,
            thinking: None,
        },
    ]
}

/// Jyh checksum: timestamp cipher + machine_id
fn compute_checksum(machine_id: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / 1_000_000;
    let ts = ts as u64;
    let mut bytes = [
        ((ts >> 40) & 0xFF) as u8,
        ((ts >> 32) & 0xFF) as u8,
        ((ts >> 24) & 0xFF) as u8,
        ((ts >> 16) & 0xFF) as u8,
        ((ts >> 8) & 0xFF) as u8,
        (ts & 0xFF) as u8,
    ];
    let mut key: u8 = 165;
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (*b ^ key).wrapping_add(i as u8);
        key = *b;
    }
    format!("{}{}", URL_SAFE_NO_PAD.encode(bytes), machine_id)
}

pub struct CursorProvider {
    client: reqwest::Client,
    access_token: String,
    machine_id: String,
    default_model: String,
    models: Vec<ModelInfo>,
}

impl CursorProvider {
    pub fn new(access_token: String, machine_id: String, model: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            access_token,
            machine_id,
            default_model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            models: cursor_models(),
        }
    }

    fn request(&self, path: &str) -> reqwest::RequestBuilder {
        let checksum = compute_checksum(&self.machine_id);
        self.client
            .post(format!("{BASE_URL}{path}"))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("x-cursor-checksum", checksum)
            .header("x-cursor-client-version", CLIENT_VERSION)
            .header("Content-Type", "application/json")
    }

    fn build_messages(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        let mut msgs = Vec::new();
        if let Some(system) = &request.system {
            msgs.push(json!({ "role": "system", "content": system }));
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
                                "image_url": { "url": format!("data:{media_type};base64,{data}") }
                            }),
                            ContentPart::ToolUse { id, name, input } => json!({
                                "type": "function",
                                "id": id,
                                "function": {"name": name, "arguments": input.to_string()},
                            }),
                            ContentPart::ToolResult {
                                tool_use_id,
                                content,
                            } => json!({
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
impl Provider for CursorProvider {
    fn name(&self) -> &str {
        "cursor"
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
            .request("/v1/chat/completions")
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
            let text = resp.text().await.unwrap_or_default();
            return Err(
                ProviderError::from_http(status.as_u16(), text, retry_after.as_deref()).into(),
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

        let thinking_enabled = request.thinking.as_ref().is_some_and(|t| t.enabled);

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if !thinking_enabled {
            if let Some(temp) = request.temperature {
                body["temperature"] = json!(temp);
            }
        }
        if thinking_enabled {
            if let Some(ref effort) = request
                .thinking
                .as_ref()
                .and_then(|t| t.reasoning_effort.clone())
            {
                body["reasoning_effort"] = json!(effort);
            }
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(self.build_tools(&request.tools));
        }

        let resp = self
            .request("/v1/chat/completions")
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
            let text = resp.text().await.unwrap_or_default();
            return Err(
                ProviderError::from_http(status.as_u16(), text, retry_after.as_deref()).into(),
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
                        output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
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
