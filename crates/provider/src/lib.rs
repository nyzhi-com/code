pub mod types;

pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod claude_sdk;
pub mod codex;

mod error;
mod sse;

pub use error::ProviderError;
pub use types::*;

use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn supported_models(&self) -> &[ModelInfo];

    fn model_for_tier(&self, tier: ModelTier) -> Option<&ModelInfo> {
        let models = self.supported_models();
        models
            .iter()
            .find(|m| m.tier == tier)
            .or_else(|| models.first())
    }

    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse>;

    async fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent>>>;
}

fn resolve_api_style(name: &str, config: &nyzhi_config::Config) -> String {
    if let Some(entry) = config.provider.entry(name) {
        if let Some(style) = &entry.api_style {
            return style.clone();
        }
    }
    if let Some(def) = nyzhi_config::find_provider_def(name) {
        return def.api_style.to_string();
    }
    "openai".to_string()
}

pub fn create_provider(
    name: &str,
    config: &nyzhi_config::Config,
) -> Result<Box<dyn Provider>> {
    let style = resolve_api_style(name, config);
    let entry = config.provider.entry(name);

    match style.as_str() {
        "openai" => {
            let cred = nyzhi_auth::resolve_credential(name, entry.and_then(|e| e.api_key.as_deref()))?;
            let base_url = entry.and_then(|e| e.base_url.clone())
                .or_else(|| nyzhi_config::find_provider_def(name).map(|d| d.default_base_url.to_string()));
            Ok(Box::new(openai::OpenAIProvider::new(
                cred.header_value(), base_url, entry.and_then(|e| e.model.clone()),
            )))
        }
        "anthropic" => {
            let cred = nyzhi_auth::resolve_credential(name, entry.and_then(|e| e.api_key.as_deref()))?;
            let base_url = entry.and_then(|e| e.base_url.clone())
                .or_else(|| nyzhi_config::find_provider_def(name).map(|d| d.default_base_url.to_string()));
            Ok(Box::new(anthropic::AnthropicProvider::new(
                cred.header_value(), base_url, entry.and_then(|e| e.model.clone()),
            )))
        }
        "gemini" => {
            let cred = nyzhi_auth::resolve_credential(name, entry.and_then(|e| e.api_key.as_deref()))?;
            let base_url = entry.and_then(|e| e.base_url.clone())
                .or_else(|| nyzhi_config::find_provider_def(name).map(|d| d.default_base_url.to_string()));
            Ok(Box::new(gemini::GeminiProvider::with_credential(
                cred, base_url, entry.and_then(|e| e.model.clone()),
            )))
        }
        "claude-sdk" => {
            Ok(Box::new(claude_sdk::ClaudeSDKProvider::from_config(config)?))
        }
        "codex" => {
            Ok(Box::new(codex::CodexProvider::from_config(config)?))
        }
        other => anyhow::bail!("Unsupported api_style '{other}' for provider '{name}'"),
    }
}

pub async fn create_provider_async(
    name: &str,
    config: &nyzhi_config::Config,
) -> Result<Box<dyn Provider>> {
    let style = resolve_api_style(name, config);
    let entry = config.provider.entry(name);

    match style.as_str() {
        "claude-sdk" => {
            return Ok(Box::new(claude_sdk::ClaudeSDKProvider::from_config(config)?));
        }
        "codex" => {
            return Ok(Box::new(codex::CodexProvider::from_config(config)?));
        }
        _ => {}
    }

    let cred = nyzhi_auth::resolve_credential_async(
        name,
        entry.and_then(|p| p.api_key.as_deref()),
    )
    .await?;

    let base_url = entry.and_then(|e| e.base_url.clone())
        .or_else(|| nyzhi_config::find_provider_def(name).map(|d| d.default_base_url.to_string()));

    match style.as_str() {
        "openai" => Ok(Box::new(openai::OpenAIProvider::new(
            cred.header_value(), base_url, entry.and_then(|e| e.model.clone()),
        ))),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(
            cred.header_value(), base_url, entry.and_then(|e| e.model.clone()),
        ))),
        "gemini" => Ok(Box::new(gemini::GeminiProvider::with_credential(
            cred, base_url, entry.and_then(|e| e.model.clone()),
        ))),
        other => anyhow::bail!("Unsupported api_style '{other}' for provider '{name}'"),
    }
}

/// Model registry: collects models from all providers, supports per-provider override.
pub struct ModelRegistry {
    models: HashMap<String, Vec<ModelInfo>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        let mut models = HashMap::new();
        models.insert("openai".into(), openai::default_models());
        models.insert("anthropic".into(), anthropic::default_models());
        models.insert("gemini".into(), gemini::default_models());
        models.insert("deepseek".into(), deepseek_models());
        models.insert("groq".into(), groq_models());
        Self { models }
    }

    pub fn models_for(&self, provider: &str) -> &[ModelInfo] {
        self.models.get(provider).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn all_models(&self) -> Vec<&ModelInfo> {
        self.models.values().flat_map(|v| v.iter()).collect()
    }

    pub fn find(&self, provider: &str, model_id: &str) -> Option<&ModelInfo> {
        self.models.get(provider)?.iter().find(|m| m.id == model_id)
    }

    pub fn find_any<'a>(&'a self, model_id: &'a str) -> Option<(&'a str, &'a ModelInfo)> {
        if let Some((provider, model_id)) = model_id.split_once('/') {
            return self.find(provider, model_id).map(|m| (provider, m));
        }
        for (provider, models) in &self.models {
            if let Some(m) = models.iter().find(|m| m.id == model_id) {
                return Some((provider.as_str(), m));
            }
        }
        None
    }

    pub fn providers(&self) -> Vec<&str> {
        self.models.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn deepseek_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "deepseek-chat".into(), name: "DeepSeek V3.2".into(), provider: "deepseek".into(),
            context_window: 164_000, max_output_tokens: 16_384,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.27, output_price_per_m: 1.1,
            cache_read_price_per_m: 0.07, cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium, thinking: None,
        },
        ModelInfo {
            id: "deepseek-reasoner".into(), name: "DeepSeek R1".into(), provider: "deepseek".into(),
            context_window: 164_000, max_output_tokens: 16_384,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.55, output_price_per_m: 2.19,
            cache_read_price_per_m: 0.14, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: None,
        },
    ]
}

fn groq_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "llama-3.3-70b-versatile".into(), name: "Llama 3.3 70B".into(), provider: "groq".into(),
            context_window: 128_000, max_output_tokens: 32_768,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.59, output_price_per_m: 0.79,
            cache_read_price_per_m: 0.0, cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium, thinking: None,
        },
        ModelInfo {
            id: "llama-3.1-8b-instant".into(), name: "Llama 3.1 8B".into(), provider: "groq".into(),
            context_window: 128_000, max_output_tokens: 8_192,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.05, output_price_per_m: 0.08,
            cache_read_price_per_m: 0.0, cache_write_price_per_m: 0.0,
            tier: ModelTier::Low, thinking: None,
        },
    ]
}
