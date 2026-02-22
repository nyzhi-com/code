use anyhow::Result;
use serde::Deserialize;

use crate::token_store::{self, StoredToken};

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_CLIENT_ID: &str =
    "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";

const OPENAI_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";

const ANTHROPIC_TOKEN_URL: &str = "https://console.anthropic.com/oauth/token";
const ANTHROPIC_CLIENT_ID: &str = "9d578f71-0cdb-4744-8473-89d98ac13a3a";

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
}

pub fn is_expired(token: &StoredToken) -> bool {
    match token.expires_at {
        Some(exp) => {
            let now = chrono::Utc::now().timestamp();
            now >= exp - 60 // 60s buffer
        }
        None => false,
    }
}

pub async fn refresh_if_needed(provider: &str) -> Result<Option<StoredToken>> {
    let token = match token_store::load_token(provider)? {
        Some(t) => t,
        None => return Ok(None),
    };

    if !is_expired(&token) {
        return Ok(Some(token));
    }

    let refresh_token = match &token.refresh_token {
        Some(rt) => rt.clone(),
        None => {
            tracing::warn!("Token expired but no refresh token for {provider}");
            return Ok(None);
        }
    };

    tracing::info!("Refreshing expired token for {provider}");

    let refreshed = match provider {
        "gemini" | "google" => refresh_google(&refresh_token).await?,
        "antigravity" => refresh_antigravity(&refresh_token).await?,
        "openai" | "chatgpt" => refresh_openai(&refresh_token).await?,
        "anthropic" => refresh_anthropic(&refresh_token).await?,
        _ => return Ok(None),
    };

    let stored = StoredToken {
        access_token: refreshed.access_token,
        refresh_token: refreshed.refresh_token.or(Some(refresh_token)),
        expires_at: refreshed
            .expires_in
            .map(|secs| chrono::Utc::now().timestamp() + secs as i64),
        provider: provider.to_string(),
    };

    token_store::store_token(provider, &stored)?;
    Ok(Some(stored))
}

async fn refresh_google(refresh_token: &str) -> Result<RefreshResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", GOOGLE_CLIENT_ID),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Google token refresh failed: {body}");
    }

    Ok(resp.json().await?)
}

const ANTIGRAVITY_CLIENT_ID: &str =
    "1071006060591-tmhssin2h21lcre235vtolojh4g403ep.apps.googleusercontent.com";
const ANTIGRAVITY_CLIENT_SECRET: &str = "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";

async fn refresh_antigravity(refresh_token: &str) -> Result<RefreshResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", ANTIGRAVITY_CLIENT_ID),
            ("client_secret", ANTIGRAVITY_CLIENT_SECRET),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Antigravity token refresh failed: {body}");
    }

    Ok(resp.json().await?)
}

async fn refresh_openai(refresh_token: &str) -> Result<RefreshResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(OPENAI_TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", OPENAI_CLIENT_ID),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenAI token refresh failed: {body}");
    }

    Ok(resp.json().await?)
}

async fn refresh_anthropic(refresh_token: &str) -> Result<RefreshResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(ANTHROPIC_TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", ANTHROPIC_CLIENT_ID),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic token refresh failed: {body}");
    }

    Ok(resp.json().await?)
}
