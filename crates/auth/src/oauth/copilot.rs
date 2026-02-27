use anyhow::Result;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::token_store::{self, StoredToken};

const CLIENT_ID: &str = "Iv1.b507a08c87ecfe98";
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const COPILOT_TOKEN_URL: &str = "https://api.github.com/copilot_internal/v2/token";
const SCOPE: &str = "read:user";

pub const COPILOT_USER_AGENT: &str = "GitHubCopilotChat/0.26.7";
pub const COPILOT_EDITOR_VERSION: &str = "vscode/1.99.3";
pub const COPILOT_PLUGIN_VERSION: &str = "copilot-chat/0.26.7";
pub const COPILOT_INTEGRATION_ID: &str = "vscode-chat";
pub const COPILOT_API_VERSION: &str = "2025-04-01";
pub const DEFAULT_COPILOT_ENDPOINT: &str = "https://api.githubcopilot.com";

const PROVIDER_NAME: &str = "github-copilot";

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default = "default_interval")]
    interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    access_token: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CopilotTokenResponse {
    pub token: String,
    pub expires_at: i64,
    #[allow(dead_code)]
    pub refresh_in: Option<i64>,
    #[serde(default)]
    pub endpoints: CopilotEndpoints,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CopilotEndpoints {
    #[serde(default = "default_api_endpoint")]
    pub api: String,
}

fn default_api_endpoint() -> String {
    DEFAULT_COPILOT_ENDPOINT.to_string()
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

fn copilot_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("User-Agent", COPILOT_USER_AGENT),
        ("Editor-Version", COPILOT_EDITOR_VERSION),
        ("Editor-Plugin-Version", COPILOT_PLUGIN_VERSION),
        ("Accept", "application/json"),
    ]
}

async fn authorize() -> Result<DeviceCodeResponse> {
    let client = http_client();
    let mut req = client
        .post(DEVICE_CODE_URL)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json");
    for (k, v) in copilot_headers() {
        req = req.header(k, v);
    }

    let resp = req
        .json(&serde_json::json!({
            "client_id": CLIENT_ID,
            "scope": SCOPE,
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub device code request failed: {body}");
    }

    Ok(resp.json().await?)
}

async fn poll_for_token(device_code: &str, interval: u64) -> Result<String> {
    let client = http_client();
    let poll_interval = std::time::Duration::from_secs(interval.max(5));

    loop {
        tokio::time::sleep(poll_interval).await;

        let mut req = client
            .post(ACCESS_TOKEN_URL)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json");
        for (k, v) in copilot_headers() {
            req = req.header(k, v);
        }

        let resp = req
            .json(&serde_json::json!({
                "client_id": CLIENT_ID,
                "device_code": device_code,
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            continue;
        }

        let data: AccessTokenResponse = resp.json().await?;

        if let Some(token) = data.access_token {
            return Ok(token);
        }

        match data.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
            Some("expired_token") => anyhow::bail!("Device code expired. Please try again."),
            Some("access_denied") => anyhow::bail!("Authorization was denied by the user."),
            Some(other) => anyhow::bail!("GitHub OAuth error: {other}"),
            None => continue,
        }
    }
}

pub async fn exchange_copilot_token(github_token: &str) -> Result<CopilotTokenResponse> {
    let client = http_client();
    let mut req = client
        .get(COPILOT_TOKEN_URL)
        .header("Authorization", format!("token {github_token}"));
    for (k, v) in copilot_headers() {
        req = req.header(k, v);
    }

    let resp = req.send().await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        if status == 401 || status == 403 {
            anyhow::bail!(
                "GitHub Copilot token exchange failed ({status}): Your GitHub account may not \
                 have an active Copilot subscription. Check github.com/settings/copilot"
            );
        }
        anyhow::bail!("GitHub Copilot token exchange failed ({status}): {body}");
    }

    let mut token_resp: CopilotTokenResponse = resp.json().await?;
    if token_resp.endpoints.api.is_empty() {
        token_resp.endpoints.api = DEFAULT_COPILOT_ENDPOINT.to_string();
    }
    Ok(token_resp)
}

fn store_copilot_tokens(github_token: &str, copilot: &CopilotTokenResponse) -> Result<()> {
    let stored = StoredToken {
        access_token: copilot.token.clone(),
        refresh_token: Some(github_token.to_string()),
        expires_at: Some(copilot.expires_at),
        provider: PROVIDER_NAME.to_string(),
    };
    token_store::store_token(PROVIDER_NAME, &stored)
}

pub async fn login() -> Result<StoredToken> {
    login_inner(None).await
}

pub async fn login_interactive(msg_tx: mpsc::UnboundedSender<String>) -> Result<StoredToken> {
    login_inner(Some(msg_tx)).await
}

async fn login_inner(msg_tx: Option<mpsc::UnboundedSender<String>>) -> Result<StoredToken> {
    let send = |s: String| {
        if let Some(ref tx) = msg_tx {
            let _ = tx.send(s);
        } else {
            eprintln!("{s}");
        }
    };

    send("Starting GitHub Copilot device authorization...".to_string());

    let device = authorize().await?;

    send(format!(
        "Please visit: {}\nEnter code: {}",
        device.verification_uri, device.user_code
    ));

    if let Err(e) = open::that(&device.verification_uri) {
        tracing::warn!(error = %e, "Failed to open browser");
        send(format!(
            "Could not open browser. Open {} manually.",
            device.verification_uri
        ));
    }

    send("Waiting for authorization...".to_string());
    let github_token = poll_for_token(&device.device_code, device.interval).await?;

    send("GitHub authorized. Exchanging for Copilot token...".to_string());
    let copilot = exchange_copilot_token(&github_token).await?;

    store_copilot_tokens(&github_token, &copilot)?;

    send("GitHub Copilot login successful.".to_string());

    Ok(StoredToken {
        access_token: copilot.token,
        refresh_token: Some(github_token),
        expires_at: Some(copilot.expires_at),
        provider: PROVIDER_NAME.to_string(),
    })
}
