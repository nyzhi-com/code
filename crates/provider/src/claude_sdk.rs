use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::types::*;
use crate::Provider;

pub struct ClaudeSDKProvider {
    default_model: String,
    models: Vec<ModelInfo>,
}

impl ClaudeSDKProvider {
    pub fn new(model: Option<String>) -> Self {
        Self {
            default_model: model.unwrap_or_else(|| "claude-sonnet-4-6-20260217".to_string()),
            models: vec![ModelInfo {
                id: "claude-sdk".into(),
                name: "Claude Agent SDK".into(),
                provider: "claude-sdk".into(),
                context_window: 1_000_000,
                max_output_tokens: 32_768,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: true,
                input_price_per_m: 3.0,
                output_price_per_m: 15.0,
                cache_read_price_per_m: 0.3,
                cache_write_price_per_m: 3.75,
                tier: ModelTier::High,
                thinking: Some(ThinkingSupport::anthropic_budget(32768)),
            }],
        }
    }

    pub fn from_config(config: &nyzhi_config::Config) -> Result<Self> {
        let entry = config.provider.entry("claude-sdk");
        Ok(Self::new(entry.and_then(|e| e.model.clone())))
    }
}

#[async_trait]
impl Provider for ClaudeSDKProvider {
    fn name(&self) -> &str {
        "claude-sdk"
    }

    fn supported_models(&self) -> &[ModelInfo] {
        &self.models
    }

    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let _model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        // The Claude Agent SDK delegates to the Claude Code CLI subprocess.
        // Integration requires `claude-agent-sdk-rs` crate (feature-gated).
        // For now, provide a clear error until the feature is enabled.
        anyhow::bail!(
            "Claude Agent SDK provider requires the 'claude-sdk' feature. \
             Install the Claude Code CLI and enable the feature: \
             cargo build --features claude-sdk"
        )
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent>>> {
        let _model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        anyhow::bail!(
            "Claude Agent SDK provider requires the 'claude-sdk' feature. \
             Install the Claude Code CLI and enable the feature: \
             cargo build --features claude-sdk"
        )
    }
}
