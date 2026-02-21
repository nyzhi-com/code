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

pub fn create_provider(
    name: &str,
    config: &nyzhi_config::Config,
) -> Result<Box<dyn Provider>> {
    match name {
        "openai" => openai::OpenAIProvider::from_config(config).map(|p| Box::new(p) as _),
        "anthropic" => anthropic::AnthropicProvider::from_config(config).map(|p| Box::new(p) as _),
        "gemini" => gemini::GeminiProvider::from_config(config).map(|p| Box::new(p) as _),
        other => anyhow::bail!("Unknown provider: {other}"),
    }
}

pub async fn create_provider_async(
    name: &str,
    config: &nyzhi_config::Config,
) -> Result<Box<dyn Provider>> {
    let provider_conf = config.provider.entry(name);
    let cred = nyzhi_auth::resolve_credential_async(
        name,
        provider_conf.as_ref().and_then(|p| p.api_key.as_deref()),
    )
    .await?;

    match name {
        "openai" => Ok(Box::new(openai::OpenAIProvider::new(
            cred.header_value(),
            provider_conf.as_ref().and_then(|p| p.base_url.clone()),
            provider_conf.as_ref().and_then(|p| p.model.clone()),
        ))),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(
            cred.header_value(),
            provider_conf.as_ref().and_then(|p| p.base_url.clone()),
            provider_conf.as_ref().and_then(|p| p.model.clone()),
        ))),
        "gemini" => Ok(Box::new(gemini::GeminiProvider::with_credential(
            cred,
            provider_conf.as_ref().and_then(|p| p.base_url.clone()),
            provider_conf.as_ref().and_then(|p| p.model.clone()),
        ))),
        other => anyhow::bail!("Unknown provider: {other}"),
    }
}
