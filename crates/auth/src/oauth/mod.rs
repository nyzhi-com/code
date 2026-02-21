pub mod google;
pub mod openai;
pub mod refresh;

use anyhow::Result;

use crate::token_store::StoredToken;

pub async fn login(provider: &str) -> Result<StoredToken> {
    match provider {
        "gemini" | "google" => google::login().await,
        "openai" => openai::login().await,
        "anthropic" => {
            anyhow::bail!(
                "Anthropic does not support OAuth. Use ANTHROPIC_API_KEY or set api_key in config."
            )
        }
        other => anyhow::bail!("Unknown provider for OAuth: {other}"),
    }
}

pub fn supports_oauth(provider: &str) -> bool {
    matches!(provider, "gemini" | "google" | "openai")
}
