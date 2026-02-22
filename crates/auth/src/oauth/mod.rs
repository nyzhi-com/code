pub mod anthropic;
pub mod antigravity;
pub mod chatgpt;
pub mod google;
pub mod openai;
pub mod refresh;

use anyhow::Result;
use tokio::sync::mpsc;

use crate::token_store::StoredToken;

pub async fn login(provider: &str) -> Result<StoredToken> {
    match provider {
        "gemini" | "google" => google::login().await,
        "openai" => openai::login().await,
        "anthropic" => anthropic::login().await,
        "chatgpt" => chatgpt::login().await,
        other => {
            if let Some(def) = nyzhi_config::find_provider_def(other) {
                if !def.supports_oauth {
                    anyhow::bail!(
                        "{} does not support OAuth. Set {} or use `/connect` to add an API key.",
                        def.name, def.env_var
                    );
                }
            }
            anyhow::bail!("Unknown provider for OAuth: {other}. Use `/connect` to add an API key instead.")
        }
    }
}

/// TUI-safe login that sends status messages through a channel instead of stderr.
/// `method` selects which auth flow to use:
///   - "oauth" (default for anthropic)
///   - "codex" (OpenAI Codex subscription device code flow)
///   - "gemini-cli" (Google Gemini CLI OAuth)
///   - "antigravity" (Google Antigravity / Cloud Code OAuth)
pub async fn login_interactive(
    provider: &str,
    method: &str,
    msg_tx: mpsc::UnboundedSender<String>,
) -> Result<StoredToken> {
    match (provider, method) {
        ("openai", "codex") | ("openai", "oauth") => {
            openai::login_interactive(msg_tx).await
        }
        ("gemini", "gemini-cli") | ("gemini", "oauth") => {
            google::login_interactive(msg_tx).await
        }
        ("gemini", "antigravity") => {
            antigravity::login(Some(msg_tx)).await
        }
        ("anthropic", _) => {
            anthropic::login_interactive(msg_tx).await
        }
        _ => {
            anyhow::bail!("No interactive login for {provider}/{method}")
        }
    }
}

pub fn supports_oauth(provider: &str) -> bool {
    nyzhi_config::find_provider_def(provider)
        .map(|d| d.supports_oauth)
        .unwrap_or(false)
}
