use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::types::*;
use crate::Provider;

pub struct CodexProvider {
    default_model: String,
    models: Vec<ModelInfo>,
}

impl CodexProvider {
    pub fn new(model: Option<String>) -> Self {
        Self {
            default_model: model.unwrap_or_else(|| "gpt-5.3-codex".to_string()),
            models: vec![ModelInfo {
                id: "codex".into(),
                name: "OpenAI Codex CLI".into(),
                provider: "codex".into(),
                context_window: 272_000,
                max_output_tokens: 100_000,
                supports_tools: true,
                supports_streaming: true,
                supports_vision: true,
                input_price_per_m: 2.0,
                output_price_per_m: 8.0,
                cache_read_price_per_m: 0.5,
                cache_write_price_per_m: 0.0,
                tier: ModelTier::High,
                thinking: Some(ThinkingSupport::openai_reasoning()),
            }],
        }
    }

    pub fn from_config(config: &nyzhi_config::Config) -> Result<Self> {
        let entry = config.provider.entry("codex");
        Ok(Self::new(entry.and_then(|e| e.model.clone())))
    }

    fn check_codex_installed() -> Result<()> {
        match std::process::Command::new("codex").arg("--version").output() {
            Ok(output) if output.status.success() => Ok(()),
            _ => anyhow::bail!(
                "OpenAI Codex CLI is not installed or not in PATH. \
                 Install it with: npm install -g @openai/codex"
            ),
        }
    }
}

#[async_trait]
impl Provider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    fn supported_models(&self) -> &[ModelInfo] {
        &self.models
    }

    async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        Self::check_codex_installed()?;

        let _model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        // Codex provider communicates via `codex mcp-server` subprocess using MCP stdio.
        // nyzhi already uses `rmcp` for MCP transport.
        // Full integration requires spawning the codex process and piping MCP messages.
        anyhow::bail!(
            "Codex MCP provider is not yet fully integrated. \
             Use the OpenAI provider directly with Codex models (gpt-5.3-codex)."
        )
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent>>> {
        Self::check_codex_installed()?;

        let _model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        anyhow::bail!(
            "Codex MCP provider is not yet fully integrated. \
             Use the OpenAI provider directly with Codex models (gpt-5.3-codex)."
        )
    }
}
