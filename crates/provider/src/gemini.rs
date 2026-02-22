use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_MODEL: &str = "gemini-3-flash";

pub fn default_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gemini-3.1-pro-preview".into(), name: "Gemini 3.1 Pro".into(), provider: "gemini".into(),
            context_window: 1_048_576, max_output_tokens: 65_536,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 1.25, output_price_per_m: 10.0,
            cache_read_price_per_m: 0.3125, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: Some(ThinkingSupport::gemini_levels(&["low", "high"])),
        },
        ModelInfo {
            id: "gemini-3-flash".into(), name: "Gemini 3 Flash".into(), provider: "gemini".into(),
            context_window: 1_048_576, max_output_tokens: 65_536,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 0.15, output_price_per_m: 0.60,
            cache_read_price_per_m: 0.0375, cache_write_price_per_m: 0.0,
            tier: ModelTier::Low, thinking: Some(ThinkingSupport::gemini_levels(&["minimal", "low", "medium", "high"])),
        },
        ModelInfo {
            id: "gemini-3-pro-preview".into(), name: "Gemini 3 Pro".into(), provider: "gemini".into(),
            context_window: 1_048_576, max_output_tokens: 65_536,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 1.25, output_price_per_m: 10.0,
            cache_read_price_per_m: 0.3125, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: Some(ThinkingSupport::gemini_levels(&["low", "high"])),
        },
        ModelInfo {
            id: "gemini-2.5-flash".into(), name: "Gemini 2.5 Flash".into(), provider: "gemini".into(),
            context_window: 1_048_576, max_output_tokens: 65_536,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 0.15, output_price_per_m: 0.60,
            cache_read_price_per_m: 0.0375, cache_write_price_per_m: 0.0,
            tier: ModelTier::Low, thinking: Some(ThinkingSupport::anthropic_budget(32768)),
        },
    ]
}

pub enum GeminiAuthMode {
    ApiKey(String),
    Bearer(String),
}

pub struct GeminiProvider {
    client: reqwest::Client,
    base_url: String,
    auth: GeminiAuthMode,
    default_model: String,
    models: Vec<ModelInfo>,
}

impl GeminiProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            auth: GeminiAuthMode::ApiKey(api_key),
            default_model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            models: default_models(),
        }
    }

    pub fn with_credential(
        credential: nyzhi_auth::Credential,
        base_url: Option<String>,
        model: Option<String>,
    ) -> Self {
        let auth = match credential {
            nyzhi_auth::Credential::Bearer(token) => GeminiAuthMode::Bearer(token),
            nyzhi_auth::Credential::ApiKey(key) => GeminiAuthMode::ApiKey(key),
        };
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            auth,
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
        let entry = config.provider.entry("gemini");
        let cred =
            nyzhi_auth::resolve_credential("gemini", entry.and_then(|e| e.api_key.as_deref()))?;
        Ok(Self::with_credential(
            cred,
            entry.and_then(|e| e.base_url.clone()),
            entry.and_then(|e| e.model.clone()),
        ))
    }

    fn build_url(&self, model: &str, action: &str) -> String {
        match &self.auth {
            GeminiAuthMode::ApiKey(key) => {
                format!("{}/models/{}:{}?key={}", self.base_url, model, action, key)
            }
            GeminiAuthMode::Bearer(_) => {
                format!("{}/models/{}:{}", self.base_url, model, action)
            }
        }
    }

    fn apply_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.auth {
            GeminiAuthMode::ApiKey(_) => builder,
            GeminiAuthMode::Bearer(token) => {
                builder.header("authorization", format!("Bearer {token}"))
            }
        }
    }

    fn build_contents(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|msg| {
                let role = match msg.role {
                    Role::Assistant => "model",
                    _ => "user",
                };
                let parts = match &msg.content {
                    MessageContent::Text(text) => vec![json!({"text": text})],
                    MessageContent::Parts(parts) => parts
                        .iter()
                        .map(|p| match p {
                            ContentPart::Text { text } => json!({"text": text}),
                            ContentPart::Image { media_type, data } => json!({
                                "inline_data": {
                                    "mime_type": media_type,
                                    "data": data,
                                }
                            }),
                            ContentPart::ToolUse { name, input, .. } => json!({
                                "functionCall": {"name": name, "args": input}
                            }),
                            ContentPart::ToolResult {
                                tool_use_id,
                                content,
                            } => json!({
                                "functionResponse": {
                                    "name": tool_use_id,
                                    "response": {"result": content},
                                }
                            }),
                        })
                        .collect(),
                };
                json!({"role": role, "parts": parts})
            })
            .collect()
    }

    fn build_tools(&self, tools: &[ToolDefinition]) -> serde_json::Value {
        let declarations: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                })
            })
            .collect();

        json!([{"functionDeclarations": declarations}])
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
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
            "contents": self.build_contents(request),
        });

        if let Some(system) = &request.system {
            body["systemInstruction"] = json!({"parts": [{"text": system}]});
        }
        if !request.tools.is_empty() {
            body["tools"] = self.build_tools(&request.tools);
        }

        let mut config = json!({});
        if let Some(max_tokens) = request.max_tokens {
            config["maxOutputTokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            config["temperature"] = json!(temp);
        }
        if !config.as_object().unwrap().is_empty() {
            body["generationConfig"] = config;
        }

        let url = self.build_url(model, "generateContent");

        let req = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body);
        let resp = self.apply_auth(req).send().await?;

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
        let content = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let cached = data["usageMetadata"]["cachedContentTokenCount"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content: MessageContent::Text(content),
            },
            usage: Some(Usage {
                input_tokens: data["usageMetadata"]["promptTokenCount"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                output_tokens: data["usageMetadata"]["candidatesTokenCount"]
                    .as_u64()
                    .unwrap_or(0) as u32,
                cache_read_tokens: cached,
                cache_creation_tokens: 0,
            }),
            finish_reason: data["candidates"][0]["finishReason"]
                .as_str()
                .map(String::from),
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
            "contents": self.build_contents(request),
        });

        if let Some(system) = &request.system {
            body["systemInstruction"] = json!({"parts": [{"text": system}]});
        }
        if !request.tools.is_empty() {
            body["tools"] = self.build_tools(&request.tools);
        }

        let thinking_enabled = request
            .thinking
            .as_ref()
            .map(|t| t.enabled)
            .unwrap_or(false);

        let mut config = json!({});
        if let Some(max_tokens) = request.max_tokens {
            config["maxOutputTokens"] = json!(max_tokens);
        }
        if !thinking_enabled {
            if let Some(temp) = request.temperature {
                config["temperature"] = json!(temp);
            }
        }
        if thinking_enabled {
            let budget = request
                .thinking
                .as_ref()
                .and_then(|t| t.budget_tokens)
                .unwrap_or(8192);
            config["thinkingConfig"] = json!({
                "thinkingBudget": budget
            });
        }
        if !config.as_object().unwrap().is_empty() {
            body["generationConfig"] = config;
        }

        let url = {
            let base = self.build_url(model, "streamGenerateContent");
            if base.contains('?') {
                format!("{base}&alt=sse")
            } else {
                format!("{base}?alt=sse")
            }
        };

        let req = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body);
        let resp = self.apply_auth(req).send().await?;

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
                    let mut evts = Vec::new();

                    if let Some(parts) =
                        data["candidates"][0]["content"]["parts"].as_array()
                    {
                        for part in parts {
                            if part.get("thought").and_then(|v| v.as_bool()).unwrap_or(false) {
                                if let Some(text) = part["text"].as_str() {
                                    evts.push(Ok(StreamEvent::ThinkingDelta(text.to_string())));
                                    continue;
                                }
                            }
                            if let Some(text) = part["text"].as_str() {
                                evts.push(Ok(StreamEvent::TextDelta(text.to_string())));
                            }
                            if part.get("functionCall").is_some() {
                                evts.push(Ok(StreamEvent::ToolCallStart {
                                    index: 0,
                                    id: uuid::Uuid::new_v4().to_string(),
                                    name: part["functionCall"]["name"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                }));
                            }
                        }
                    }

                    if let Some(meta) =
                        data.get("usageMetadata").filter(|m| m.is_object())
                    {
                        let cached = meta["cachedContentTokenCount"]
                            .as_u64()
                            .unwrap_or(0) as u32;
                        evts.push(Ok(StreamEvent::Usage(Usage {
                            input_tokens: meta["promptTokenCount"]
                                .as_u64()
                                .unwrap_or(0) as u32,
                            output_tokens: meta["candidatesTokenCount"]
                                .as_u64()
                                .unwrap_or(0) as u32,
                            cache_read_tokens: cached,
                            cache_creation_tokens: 0,
                        })));
                    }

                    if data["candidates"][0]["finishReason"].is_string() {
                        evts.push(Ok(StreamEvent::Done));
                    }

                    evts
                }
                Err(e) => vec![Err(e)],
            };
            futures::stream::iter(events)
        });

        Ok(Box::pin(event_stream))
    }
}
