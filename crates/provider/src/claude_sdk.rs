use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::anthropic::AnthropicProvider;
use crate::types::*;
use crate::Provider;

/// Claude SDK provider: delegates to AnthropicProvider with agent-oriented defaults.
/// Uses the standard Anthropic Messages API with extended thinking enabled.
pub struct ClaudeSDKProvider {
    inner: AnthropicProvider,
}

impl ClaudeSDKProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        let model = model.or_else(|| Some("claude-sonnet-4-6-20260217".to_string()));
        Self {
            inner: AnthropicProvider::new(api_key, base_url, model),
        }
    }
}

#[async_trait]
impl Provider for ClaudeSDKProvider {
    fn name(&self) -> &str {
        "claude-sdk"
    }

    fn supported_models(&self) -> &[ModelInfo] {
        self.inner.supported_models()
    }

    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        self.inner.chat(request).await
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent>>> {
        self.inner.chat_stream(request).await
    }
}
