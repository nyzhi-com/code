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
        let kimi = kimi_models();
        models.insert("kimi".into(), kimi.clone());
        models.insert("kimi-coding".into(), kimi);
        let minimax = minimax_models();
        models.insert("minimax".into(), minimax.clone());
        models.insert("minimax-coding".into(), minimax);
        let glm = glm_models();
        models.insert("glm".into(), glm.clone());
        models.insert("glm-coding".into(), glm);
        models.insert("antigravity".into(), antigravity_models());
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

fn kimi_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "kimi-k2.5".into(), name: "Kimi K2.5".into(), provider: "kimi".into(),
            context_window: 262_144, max_output_tokens: 32_768,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 0.60, output_price_per_m: 3.0,
            cache_read_price_per_m: 0.10, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: Some(ThinkingSupport::kimi_thinking()),
        },
        ModelInfo {
            id: "kimi-k2-0905-preview".into(), name: "Kimi K2".into(), provider: "kimi".into(),
            context_window: 262_144, max_output_tokens: 32_768,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.60, output_price_per_m: 2.50,
            cache_read_price_per_m: 0.15, cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium, thinking: None,
        },
        ModelInfo {
            id: "kimi-k2-turbo-preview".into(), name: "Kimi K2 Turbo".into(), provider: "kimi".into(),
            context_window: 262_144, max_output_tokens: 32_768,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 1.15, output_price_per_m: 8.0,
            cache_read_price_per_m: 0.15, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: None,
        },
    ]
}

fn minimax_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "MiniMax-M2.5".into(), name: "MiniMax M2.5".into(), provider: "minimax".into(),
            context_window: 204_800, max_output_tokens: 65_536,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.30, output_price_per_m: 1.20,
            cache_read_price_per_m: 0.03, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: None,
        },
        ModelInfo {
            id: "MiniMax-M2.5-highspeed".into(), name: "MiniMax M2.5 Highspeed".into(), provider: "minimax".into(),
            context_window: 204_800, max_output_tokens: 65_536,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.30, output_price_per_m: 2.40,
            cache_read_price_per_m: 0.03, cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium, thinking: None,
        },
        ModelInfo {
            id: "MiniMax-M2.1".into(), name: "MiniMax M2.1".into(), provider: "minimax".into(),
            context_window: 204_800, max_output_tokens: 65_536,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.27, output_price_per_m: 0.95,
            cache_read_price_per_m: 0.03, cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium, thinking: None,
        },
    ]
}

fn antigravity_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gemini-3.1-pro".into(), name: "Gemini 3.1 Pro (Antigravity)".into(),
            provider: "antigravity".into(),
            context_window: 1_048_576, max_output_tokens: 65_535,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 0.0, output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0, cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::gemini_levels(&["low", "high"])),
        },
        ModelInfo {
            id: "gemini-3-flash".into(), name: "Gemini 3 Flash (Antigravity)".into(),
            provider: "antigravity".into(),
            context_window: 1_048_576, max_output_tokens: 65_536,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 0.0, output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0, cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium,
            thinking: Some(ThinkingSupport::gemini_levels(&["minimal", "low", "medium", "high"])),
        },
        ModelInfo {
            id: "claude-sonnet-4-6".into(), name: "Claude Sonnet 4.6 (Antigravity)".into(),
            provider: "antigravity".into(),
            context_window: 200_000, max_output_tokens: 64_000,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 0.0, output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0, cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::BudgetTokens { max: 32_768, default: 16_000 }),
        },
        ModelInfo {
            id: "claude-opus-4-6-thinking".into(), name: "Claude Opus 4.6 (Antigravity)".into(),
            provider: "antigravity".into(),
            context_window: 200_000, max_output_tokens: 64_000,
            supports_tools: true, supports_streaming: true, supports_vision: true,
            input_price_per_m: 0.0, output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0, cache_write_price_per_m: 0.0,
            tier: ModelTier::High,
            thinking: Some(ThinkingSupport::BudgetTokens { max: 32_768, default: 16_000 }),
        },
        ModelInfo {
            id: "gpt-oss-120b".into(), name: "GPT-OSS 120B (Antigravity)".into(),
            provider: "antigravity".into(),
            context_window: 128_000, max_output_tokens: 16_384,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.0, output_price_per_m: 0.0,
            cache_read_price_per_m: 0.0, cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium, thinking: None,
        },
    ]
}

fn glm_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "glm-5".into(), name: "GLM-5".into(), provider: "glm".into(),
            context_window: 200_000, max_output_tokens: 128_000,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 1.0, output_price_per_m: 3.20,
            cache_read_price_per_m: 0.20, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: None,
        },
        ModelInfo {
            id: "glm-5-code".into(), name: "GLM-5 Code".into(), provider: "glm".into(),
            context_window: 200_000, max_output_tokens: 128_000,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 1.20, output_price_per_m: 5.0,
            cache_read_price_per_m: 0.24, cache_write_price_per_m: 0.0,
            tier: ModelTier::High, thinking: None,
        },
        ModelInfo {
            id: "glm-4.7".into(), name: "GLM-4.7".into(), provider: "glm".into(),
            context_window: 200_000, max_output_tokens: 128_000,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.60, output_price_per_m: 2.20,
            cache_read_price_per_m: 0.11, cache_write_price_per_m: 0.0,
            tier: ModelTier::Medium, thinking: None,
        },
        ModelInfo {
            id: "glm-4.7-flashx".into(), name: "GLM-4.7 FlashX".into(), provider: "glm".into(),
            context_window: 200_000, max_output_tokens: 128_000,
            supports_tools: true, supports_streaming: true, supports_vision: false,
            input_price_per_m: 0.07, output_price_per_m: 0.40,
            cache_read_price_per_m: 0.01, cache_write_price_per_m: 0.0,
            tier: ModelTier::Low, thinking: None,
        },
    ]
}
