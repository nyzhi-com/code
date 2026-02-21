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
