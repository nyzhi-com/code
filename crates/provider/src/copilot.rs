use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;
use tokio::sync::Mutex;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const DEFAULT_ENDPOINT: &str = "https://api.githubcopilot.com";
const DEFAULT_MODEL: &str = "claude-sonnet-4";

const USER_AGENT: &str = "GitHubCopilotChat/0.26.7";
const EDITOR_VERSION: &str = "vscode/1.99.3";
const PLUGIN_VERSION: &str = "copilot-chat/0.26.7";
const INTEGRATION_ID: &str = "vscode-chat";
const API_VERSION: &str = "2025-04-01";

struct CopilotAccess {
    token: String,
    expires_at: i64,
    endpoint: String,
}

pub struct CopilotProvider {
    github_token: String,
    state: Arc<Mutex<CopilotAccess>>,
    default_model: String,
    models: Vec<ModelInfo>,
    http: reqwest::Client,
}

impl CopilotProvider {
    pub fn new(
        github_token: String,
        initial_copilot_token: String,
        initial_expires_at: i64,
        initial_endpoint: Option<String>,
        model_override: Option<String>,
    ) -> Self {
        let endpoint = initial_endpoint
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string());

        Self {
            github_token,
            state: Arc::new(Mutex::new(CopilotAccess {
                token: initial_copilot_token,
                expires_at: initial_expires_at,
                endpoint,
            })),
            default_model: model_override.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            models: copilot_models(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }

    async fn ensure_token(&self) -> Result<(String, String)> {
        let mut state = self.state.lock().await;
        let now = chrono::Utc::now().timestamp();

        if now < state.expires_at - 60 {
            return Ok((state.token.clone(), state.endpoint.clone()));
        }

        tracing::info!("Copilot token expired, exchanging for new one");
        let refreshed =
            nyzhi_auth::oauth::copilot::exchange_copilot_token(&self.github_token).await?;

        state.token = refreshed.token.clone();
        state.expires_at = refreshed.expires_at;
        if !refreshed.endpoints.api.is_empty() {
            state.endpoint = refreshed.endpoints.api;
        }

        let _ = nyzhi_auth::token_store::store_token(
            "github-copilot",
            &nyzhi_auth::token_store::StoredToken {
                access_token: refreshed.token.clone(),
                refresh_token: Some(self.github_token.clone()),
                expires_at: Some(refreshed.expires_at),
                provider: "github-copilot".to_string(),
            },
        );

        Ok((state.token.clone(), state.endpoint.clone()))
    }

    fn copilot_headers(token: &str) -> Vec<(&'static str, String)> {
        vec![
            ("Authorization", format!("Bearer {token}")),
            ("User-Agent", USER_AGENT.to_string()),
            ("Editor-Version", EDITOR_VERSION.to_string()),
            ("Editor-Plugin-Version", PLUGIN_VERSION.to_string()),
            ("Copilot-Integration-Id", INTEGRATION_ID.to_string()),
            ("OpenAI-Intent", "conversation-panel".to_string()),
            ("x-github-api-version", API_VERSION.to_string()),
            ("x-request-id", uuid::Uuid::new_v4().to_string()),
            ("Content-Type", "application/json".to_string()),
        ]
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
                                ..
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

    fn format_copilot_error(status: u16, body: &str, model: &str) -> String {
        let parsed_msg = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| {
                v["error"]["message"]
                    .as_str()
                    .or(v["message"].as_str())
                    .or(v["error"].as_str())
                    .map(String::from)
            });

        let base_msg = parsed_msg.as_deref().unwrap_or(body);

        match status {
            401 | 403 => format!(
                "{status}: {base_msg}\n\n\
                 Your GitHub Copilot subscription may not include model '{model}'. \
                 Check your plan at github.com/settings/copilot.\n\
                 If you recently enabled this model, it may take up to 12 hours to become available."
            ),
            400 if base_msg.contains("not available")
                || base_msg.contains("not enabled")
                || base_msg.contains("not supported") =>
            {
                format!(
                    "{status}: {base_msg}\n\n\
                     Enable '{model}' in GitHub Settings > Copilot > Models.\n\
                     Changes may take up to 12 hours to propagate."
                )
            }
            _ => format!("{status}: {base_msg}"),
        }
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
impl Provider for CopilotProvider {
    fn name(&self) -> &str {
        "github-copilot"
    }

    fn supported_models(&self) -> &[ModelInfo] {
        &self.models
    }

    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let (token, endpoint) = self.ensure_token().await?;
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

        let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
        let mut req = self.http.post(&url);
        for (k, v) in Self::copilot_headers(&token) {
            req = req.header(k, v);
        }

        let resp = req.json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let resp_body = resp.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(ProviderError::from_http(
                    429,
                    resp_body,
                    retry_after.as_deref(),
                )
                .into());
            }

            let msg = Self::format_copilot_error(status.as_u16(), &resp_body, model);
            return Err(ProviderError::HttpError {
                status: status.as_u16(),
                body: msg,
            }
            .into());
        }

        let data: serde_json::Value = resp.json().await?;
        let choice = &data["choices"][0];
        let message = &choice["message"];

        let msg_content = {
            let mut parts = Vec::new();
            if let Some(text) = message["content"].as_str() {
                if !text.is_empty() {
                    parts.push(ContentPart::Text {
                        text: text.to_string(),
                    });
                }
            }
            if let Some(tool_calls) = message["tool_calls"].as_array() {
                for tc in tool_calls {
                    parts.push(ContentPart::ToolUse {
                        id: tc["id"].as_str().unwrap_or("").to_string(),
                        name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                        input: serde_json::from_str(
                            tc["function"]["arguments"].as_str().unwrap_or("{}"),
                        )
                        .unwrap_or(json!({})),
                    });
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
        };

        let cached = data["usage"]["prompt_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: msg_content,
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
        let (token, endpoint) = self.ensure_token().await?;
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };
        let model_owned = model.to_string();

        let mut body = json!({
            "model": model,
            "messages": self.build_messages(request),
            "stream": true,
            "stream_options": {"include_usage": true},
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }

        let thinking_enabled = request
            .thinking
            .as_ref()
            .map(|t| t.enabled)
            .unwrap_or(false);
        if !thinking_enabled {
            if let Some(temp) = request.temperature {
                body["temperature"] = json!(temp);
            }
        }
        if thinking_enabled {
            let effort = request
                .thinking
                .as_ref()
                .and_then(|t| t.reasoning_effort.as_deref())
                .unwrap_or("high");
            body["reasoning_effort"] = json!(effort);
        }

        if !request.tools.is_empty() {
            body["tools"] = json!(self.build_tools(&request.tools));
        }

        let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
        let mut req = self.http.post(&url);
        for (k, v) in Self::copilot_headers(&token) {
            req = req.header(k, v);
        }

        let resp = req.json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let retry_after = resp
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let resp_body = resp.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(ProviderError::from_http(
                    429,
                    resp_body,
                    retry_after.as_deref(),
                )
                .into());
            }

            let msg = Self::format_copilot_error(status.as_u16(), &resp_body, &model_owned);
            return Err(ProviderError::HttpError {
                status: status.as_u16(),
                body: msg,
            }
            .into());
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

                let choice = &data["choices"][0];
                if choice.is_null() {
                    return Ok(StreamEvent::TextDelta(String::new()));
                }

                if choice["finish_reason"].is_string() {
                    return Ok(StreamEvent::Done);
                }

                let delta = &choice["delta"];

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

pub fn copilot_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gpt-5.3-codex".into(),
            name: "GPT-5.3 Codex".into(),
            provider: "github-copilot".into(),
            context_window: 400_000,
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
            id: "claude-opus-4-6-20260205".into(),
            name: "Claude Opus 4.6".into(),
            provider: "github-copilot".into(),
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
            thinking: Some(ThinkingSupport::openai_reasoning()),
        },
        ModelInfo {
            id: "claude-sonnet-4-6-20260217".into(),
            name: "Claude Sonnet 4.6".into(),
            provider: "github-copilot".into(),
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
            id: "gpt-5.2".into(),
            name: "GPT-5.2".into(),
            provider: "github-copilot".into(),
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
            id: "gemini-3.1-pro".into(),
            name: "Gemini 3.1 Pro".into(),
            provider: "github-copilot".into(),
            context_window: 1_000_000,
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
            id: "o4-mini".into(),
            name: "O4 Mini".into(),
            provider: "github-copilot".into(),
            context_window: 200_000,
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
            id: "gemini-3-flash".into(),
            name: "Gemini 3 Flash".into(),
            provider: "github-copilot".into(),
            context_window: 1_000_000,
            max_output_tokens: 65_536,
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
        ModelInfo {
            id: "claude-haiku-4-5-20251022".into(),
            name: "Claude Haiku 4.5".into(),
            provider: "github-copilot".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
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
    ]
}
