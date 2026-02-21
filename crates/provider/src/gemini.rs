use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;

use crate::sse::parse_sse_stream;
use crate::types::*;
use crate::{Provider, ProviderError};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_MODEL: &str = "gemini-2.5-flash";

static MODELS: &[ModelInfo] = &[
    ModelInfo {
        id: "gemini-2.5-flash",
        name: "Gemini 2.5 Flash",
        context_window: 1_048_576,
        max_output_tokens: 65_536,
        supports_tools: true,
        supports_streaming: true,
    },
    ModelInfo {
        id: "gemini-2.5-pro",
        name: "Gemini 2.5 Pro",
        context_window: 1_048_576,
        max_output_tokens: 65_536,
        supports_tools: true,
        supports_streaming: true,
    },
];

pub struct GeminiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            api_key,
            default_model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }

    pub fn from_config(config: &nyzhi_config::Config) -> Result<Self> {
        let cred =
            nyzhi_auth::resolve_credential("gemini", config.provider.gemini.api_key.as_deref())?;
        Ok(Self::new(
            cred.header_value(),
            config.provider.gemini.base_url.clone(),
            config.provider.gemini.model.clone(),
        ))
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
        MODELS
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

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, model, self.api_key
        );

        let resp = self
            .client
            .post(&url)
            .header("content-type", "application/json")
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
        let content = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

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

        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, model, self.api_key
        );

        let resp = self
            .client
            .post(&url)
            .header("content-type", "application/json")
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

        let event_stream = sse_stream.filter_map(|result| async move {
            match result {
                Ok(sse) => {
                    let data: serde_json::Value = serde_json::from_str(&sse.data).ok()?;
                    let parts = data["candidates"][0]["content"]["parts"].as_array()?;

                    for part in parts {
                        if let Some(text) = part["text"].as_str() {
                            return Some(Ok(StreamEvent::TextDelta(text.to_string())));
                        }
                        if part.get("functionCall").is_some() {
                            return Some(Ok(StreamEvent::ToolCallStart {
                                index: 0,
                                id: uuid::Uuid::new_v4().to_string(),
                                name: part["functionCall"]["name"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                            }));
                        }
                    }

                    if data["candidates"][0]["finishReason"].is_string() {
                        return Some(Ok(StreamEvent::Done));
                    }

                    None
                }
                Err(e) => Some(Err(e)),
            }
        });

        Ok(Box::pin(event_stream))
    }
}
