use anyhow::Result;
use serde::Deserialize;

use crate::token_store::{self, StoredToken};

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const DEVICE_CODE_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/usercode";
const DEVICE_POLL_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const DEVICE_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";
const VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    #[serde(alias = "device_auth_id")]
    device_auth_id: String,
    #[serde(alias = "user_code", alias = "usercode")]
    user_code: String,
    #[serde(default = "default_interval")]
    interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Deserialize)]
struct DevicePollResponse {
    authorization_code: Option<String>,
    code_verifier: Option<String>,
    #[allow(dead_code)]
    code_challenge: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    #[allow(dead_code)]
    id_token: Option<String>,
}

/// Login via ChatGPT Plus/Pro subscription OAuth (device code flow).
/// Uses the same official flow as `opencode-openai-codex-auth`.
/// The resulting token authenticates against the Codex backend API.
pub async fn login() -> Result<StoredToken> {
    let client = reqwest::Client::new();

    let resp = client
        .post(DEVICE_CODE_URL)
        .json(&serde_json::json!({ "client_id": CLIENT_ID }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to request device code: {body}");
    }

    let device: DeviceCodeResponse = resp.json().await?;

    eprintln!("To log in with your ChatGPT Plus/Pro subscription, visit:");
    eprintln!("  {VERIFICATION_URL}");
    eprintln!();
    eprintln!("Enter code: {}", device.user_code);
    eprintln!();
    eprintln!("Waiting for authorization...");

    if let Err(e) = open::that(VERIFICATION_URL) {
        tracing::warn!(error = %e, "Failed to open browser");
    }

    let poll_result = poll_for_authorization(&client, &device).await?;

    let auth_code = poll_result
        .authorization_code
        .ok_or_else(|| anyhow::anyhow!("No authorization code in poll response"))?;
    let code_verifier = poll_result
        .code_verifier
        .ok_or_else(|| anyhow::anyhow!("No code_verifier in poll response"))?;

    let token_resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", &auth_code),
            ("code_verifier", &code_verifier),
            ("redirect_uri", DEVICE_REDIRECT_URI),
        ])
        .send()
        .await?;

    if !token_resp.status().is_success() {
        let body = token_resp.text().await.unwrap_or_default();
        anyhow::bail!("Token exchange failed: {body}");
    }

    let tokens: TokenResponse = token_resp.json().await?;

    let expires_at = tokens
        .expires_in
        .map(|secs| chrono::Utc::now().timestamp() + secs as i64);

    let stored = StoredToken {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at,
        provider: "chatgpt".to_string(),
    };

    token_store::store_token("chatgpt", &stored)?;
    eprintln!("ChatGPT subscription login successful. Token stored.");
    eprintln!("You can now use Codex models with your subscription.");

    Ok(stored)
}

async fn poll_for_authorization(
    client: &reqwest::Client,
    device: &DeviceCodeResponse,
) -> Result<DevicePollResponse> {
    let interval = std::time::Duration::from_secs(device.interval.max(2));
    let timeout = std::time::Duration::from_secs(300);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Device authorization timed out after 5 minutes");
        }

        tokio::time::sleep(interval).await;

        let resp = client
            .post(DEVICE_POLL_URL)
            .json(&serde_json::json!({
                "device_auth_id": device.device_auth_id,
                "user_code": device.user_code,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            continue;
        }

        let poll: DevicePollResponse = resp.json().await?;

        if poll.authorization_code.is_some() {
            return Ok(poll);
        }

        if let Some(err) = &poll.error {
            if err == "authorization_pending" || err == "slow_down" {
                continue;
            }
            anyhow::bail!("Device authorization error: {err}");
        }
    }
}
