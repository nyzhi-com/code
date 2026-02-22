pub mod google;
pub mod openai;
pub mod refresh;

use anyhow::Result;

use crate::token_store::StoredToken;

pub async fn login(provider: &str) -> Result<StoredToken> {
    match provider {
        "gemini" | "google" => google::login().await,
        "openai" => openai::login().await,
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

pub fn supports_oauth(provider: &str) -> bool {
    nyzhi_config::find_provider_def(provider)
        .map(|d| d.supports_oauth)
        .unwrap_or(false)
}
