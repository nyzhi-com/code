pub mod types;

pub mod openai;
pub mod anthropic;
pub mod gemini;

mod error;
mod sse;

pub use error::ProviderError;
pub use types::*;

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
        other => anyhow::bail!("Unsupported api_style '{other}' for provider '{name}'"),
    }
}

pub async fn create_provider_async(
    name: &str,
    config: &nyzhi_config::Config,
) -> Result<Box<dyn Provider>> {
    let style = resolve_api_style(name, config);
    let entry = config.provider.entry(name);

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
