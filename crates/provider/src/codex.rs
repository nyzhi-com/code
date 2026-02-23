use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::openai::OpenAIProvider;
use crate::types::*;
use crate::Provider;

/// Codex provider: delegates to OpenAIProvider with codex-optimized defaults.
/// Uses the same OpenAI API (Chat Completions or Responses API depending on token type).
pub struct CodexProvider {
    inner: OpenAIProvider,
}

impl CodexProvider {
    pub fn new(api_key: String, base_url: Option<String>, model: Option<String>) -> Self {
        let model = model.or_else(|| Some("gpt-5.3-codex".to_string()));
        Self {
            inner: OpenAIProvider::new(api_key, base_url, model),
        }
    }
}

#[async_trait]
impl Provider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
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
