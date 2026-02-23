use anyhow::Result;
use async_trait::async_trait;
use base64::Engine as _;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
const DEFAULT_MODEL: &str = "gpt-5.3-codex";

pub fn default_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gpt-5.3-codex".into(),
            name: "GPT-5.3 Codex".into(),
            provider: "openai".into(),
            context_window: 400_000,
            max_output_tokens: 128_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 2.0,
            output_price_per_m: 8.0,
            cache_read_price_per_m: 0.5,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "gpt-5.2-codex".into(),
            name: "GPT-5.2 Codex".into(),
            provider: "openai".into(),
            context_window: 272_000,
            max_output_tokens: 100_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 2.0,
            output_price_per_m: 8.0,
            cache_read_price_per_m: 0.5,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "gpt-5.2".into(),
            name: "GPT-5.2".into(),
            provider: "openai".into(),
            context_window: 272_000,
            max_output_tokens: 100_000,
            supports_tools: true,
            supports_streaming: true,
            supports_vision: true,
            input_price_per_m: 2.0,
            output_price_per_m: 8.0,
            cache_read_price_per_m: 0.5,
            cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::openai_reasoning()),
        },
    ]
}

pub struct OpenAIProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
    models: Vec<ModelInfo>,
    is_codex_sub: bool,
    account_id: Option<String>,
    is_openrouter: bool,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        let is_codex_sub = api_key.starts_with("ey");
        let account_id = if is_codex_sub {
            extract_account_id_from_jwt(&api_key)
        } else {
            None
        };
        let effective_base = if is_codex_sub
            && (base_url.is_none() || base_url.as_deref() == Some(DEFAULT_BASE_URL))
        {
            CODEX_BASE_URL.to_string()
        } else {
            base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
        };
        let is_openrouter = effective_base.contains("openrouter.ai");
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            base_url: effective_base,
            api_key,
            default_model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            models: default_models(),
            is_codex_sub,
            account_id,
            is_openrouter,
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
        let cred =
            nyzhi_auth::resolve_credential("openai", entry.and_then(|e| e.api_key.as_deref()))?;
        Ok(Self::new(
            cred.header_value(),
            entry.and_then(|e| e.base_url.clone()),
            entry.and_then(|e| e.model.clone()),
        ))
    }

    fn chat_request(&self, url: &str) -> reqwest::RequestBuilder {
        let mut req = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key));
        if self.is_openrouter {
            req = req
                .header("HTTP-Referer", "https://github.com/nyzhi/code")
                .header("X-Title", "nyzhi");
        }
        req
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

    fn build_tools_chat(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
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

    fn build_tools_responses(&self, tools: &[ToolDefinition]) -> Vec<serde_json::Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                })
            })
            .collect()
    }

    fn build_codex_request(&self, model: &str) -> reqwest::RequestBuilder {
        let mut req = self
            .client
            .post(format!("{}/responses", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("OpenAI-Beta", "responses=experimental")
            .header("originator", "codex_cli_rs");
        if let Some(ref acct) = self.account_id {
            req = req.header("chatgpt-account-id", acct);
        }
        tracing::debug!(model, base_url = %self.base_url, "Codex sub request");
        req
    }
}

fn extract_account_id_from_jwt(token: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload = parts[1];
    let padded = match payload.len() % 4 {
        2 => format!("{payload}=="),
        3 => format!("{payload}="),
        _ => payload.to_string(),
    };
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(padded.trim_end_matches('='))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(payload))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    claims["https://api.openai.com/auth"]["chatgpt_account_id"]
        .as_str()
        .map(String::from)
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

        if self.is_codex_sub {
            return self.chat_responses_api(model, request).await;
        }

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
            body["tools"] = json!(self.build_tools_chat(&request.tools));
        }

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self.chat_request(&url).json(&body).send().await?;

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

        if self.is_codex_sub {
            return self.chat_stream_responses_api(model, request).await;
        }

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
            body["tools"] = json!(self.build_tools_chat(&request.tools));
        }

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self.chat_request(&url).json(&body).send().await?;

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

impl OpenAIProvider {
    fn build_messages_no_system(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        let mut msgs = Vec::new();
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

    async fn chat_responses_api(&self, model: &str, request: &ChatRequest) -> Result<ChatResponse> {
        let mut body = json!({
            "model": model,
            "input": self.build_messages_no_system(request),
            "store": false,
        });
        if let Some(ref sys) = request.system {
            body["instructions"] = json!(sys);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(self.build_tools_responses(&request.tools));
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        let thinking_enabled = request
            .thinking
            .as_ref()
            .map(|t| t.enabled)
            .unwrap_or(false);
        if thinking_enabled {
            let effort = request
                .thinking
                .as_ref()
                .and_then(|t| t.reasoning_effort.as_deref())
                .unwrap_or("high");
            body["reasoning"] = json!({"effort": effort});
        }

        let resp = self.build_codex_request(model).json(&body).send().await?;

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
        let mut text = String::new();
        if let Some(output) = data["output"].as_array() {
            for item in output {
                if item["type"] == "message" {
                    if let Some(content) = item["content"].as_array() {
                        for part in content {
                            if part["type"] == "output_text" {
                                if let Some(t) = part["text"].as_str() {
                                    text.push_str(t);
                                }
                            }
                        }
                    }
                }
            }
        }

        let usage = &data["usage"];
        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: MessageContent::Text(text),
            },
            usage: Some(Usage {
                input_tokens: usage["input_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: usage["output_tokens"].as_u64().unwrap_or(0) as u32,
                cache_read_tokens: 0,
                cache_creation_tokens: 0,
            }),
            finish_reason: data["status"].as_str().map(String::from),
        })
    }

    async fn chat_stream_responses_api(
        &self,
        model: &str,
        request: &ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent>>> {
        let mut body = json!({
            "model": model,
            "input": self.build_messages_no_system(request),
            "stream": true,
            "store": false,
        });
        if let Some(ref sys) = request.system {
            body["instructions"] = json!(sys);
        }
        if !request.tools.is_empty() {
            body["tools"] = json!(self.build_tools_responses(&request.tools));
        }

        let thinking_enabled = request
            .thinking
            .as_ref()
            .map(|t| t.enabled)
            .unwrap_or(false);
        if thinking_enabled {
            let effort = request
                .thinking
                .as_ref()
                .and_then(|t| t.reasoning_effort.as_deref())
                .unwrap_or("high");
            body["reasoning"] = json!({"effort": effort});
        }

        let resp = self.build_codex_request(model).json(&body).send().await?;

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
        let mut tool_index_counter: u32 = 0;

        let event_stream = sse_stream.map(move |result| {
            result.and_then(|sse| {
                let event_type = sse.event.as_deref().unwrap_or("");
                let data: serde_json::Value = serde_json::from_str(&sse.data)?;

                match event_type {
                    "response.output_text.delta" => {
                        let delta = data["delta"].as_str().unwrap_or("");
                        Ok(StreamEvent::TextDelta(delta.to_string()))
                    }
                    "response.function_call_arguments.delta" => {
                        let delta = data["delta"].as_str().unwrap_or("");
                        Ok(StreamEvent::ToolCallDelta {
                            index: tool_index_counter.saturating_sub(1),
                            arguments_delta: delta.to_string(),
                        })
                    }
                    "response.output_item.added" => {
                        let item = &data["item"];
                        if item["type"] == "function_call" {
                            let idx = tool_index_counter;
                            tool_index_counter += 1;
                            Ok(StreamEvent::ToolCallStart {
                                index: idx,
                                id: item["call_id"].as_str().unwrap_or("").to_string(),
                                name: item["name"].as_str().unwrap_or("").to_string(),
                            })
                        } else {
                            Ok(StreamEvent::TextDelta(String::new()))
                        }
                    }
                    "response.completed" => {
                        let usage = &data["response"]["usage"];
                        let input = usage["input_tokens"].as_u64().unwrap_or(0) as u32;
                        let output = usage["output_tokens"].as_u64().unwrap_or(0) as u32;
                        Ok(StreamEvent::Usage(Usage {
                            input_tokens: input,
                            output_tokens: output,
                            cache_read_tokens: 0,
                            cache_creation_tokens: 0,
                        }))
                    }
                    "response.done" => Ok(StreamEvent::Done),
                    _ => Ok(StreamEvent::TextDelta(String::new())),
                }
            })
        });

        Ok(Box::pin(event_stream))
    }
}
