use anyhow::Result;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::token_store::{self, StoredToken};

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const SCOPE: &str = "openid profile email offline_access";
const CALLBACK_PORT: u16 = 1455;

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

fn generate_verifier() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn generate_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

fn generate_state() -> String {
    let bytes: [u8; 16] = rand::rng().random();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn build_authorize_url(challenge: &str, state: &str) -> String {
    format!(
        "{AUTHORIZE_URL}?\
         response_type=code\
         &client_id={CLIENT_ID}\
         &redirect_uri={}\
         &scope={}\
         &code_challenge={challenge}\
         &code_challenge_method=S256\
         &state={state}\
         &codex_cli_simplified_flow=true\
         &originator=codex_cli_rs",
        urlencoding(REDIRECT_URI),
        urlencoding(SCOPE),
    )
}

fn urlencoding(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

/// CLI login (prints to stderr).
pub async fn login() -> Result<StoredToken> {
    login_inner(None).await
}

/// TUI-safe login (sends messages through channel instead of stderr).
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

    let verifier = generate_verifier();
    let challenge = generate_challenge(&verifier);
    let state = generate_state();
    let auth_url = build_authorize_url(&challenge, &state);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", CALLBACK_PORT)).await?;

    send("Opening browser for OpenAI login...".to_string());
    if let Err(e) = open::that(&auth_url) {
        tracing::warn!(error = %e, "Failed to open browser");
        send(format!("Open this URL manually:\n{auth_url}"));
    }
    send("Waiting for authorization...".to_string());

    let (code, received_state) =
        wait_for_callback(&listener, std::time::Duration::from_secs(300)).await?;

    if received_state.as_deref() != Some(state.as_str()) {
        anyhow::bail!("OAuth state mismatch — possible CSRF attack");
    }

    send("Exchanging code for tokens...".to_string());

    let client = reqwest::Client::new();
    let token_resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("redirect_uri", REDIRECT_URI),
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
        provider: "openai".to_string(),
    };

    token_store::store_token("openai", &stored)?;
    send("OpenAI login successful.".to_string());

    Ok(stored)
}

/// Listen on the local callback server for the OAuth redirect.
/// Returns (code, state).
async fn wait_for_callback(
    listener: &tokio::net::TcpListener,
    timeout: std::time::Duration,
) -> Result<(String, Option<String>)> {
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let accept = tokio::time::timeout_at(deadline, listener.accept()).await;

        let (mut stream, _addr) = match accept {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                tracing::debug!("accept error: {e}");
                continue;
            }
            Err(_) => anyhow::bail!("Authorization timed out after 5 minutes"),
        };

        let mut buf = vec![0u8; 4096];
        let n = tokio::time::timeout_at(deadline, stream.read(&mut buf))
            .await
            .map_err(|_| anyhow::anyhow!("Read timeout"))??;

        let request = String::from_utf8_lossy(&buf[..n]);

        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("");

        if !path.starts_with("/auth/callback") {
            let body = "Not found";
            let resp = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            continue;
        }

        let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");
        let mut code = None;
        let mut state = None;
        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                match k {
                    "code" => code = Some(v.to_string()),
                    "state" => state = Some(v.to_string()),
                    _ => {}
                }
            }
        }

        let html = if code.is_some() {
            "<html><body><h2>Login successful!</h2><p>You can close this tab and return to nyzhi.</p></body></html>"
        } else {
            "<html><body><h2>Login failed</h2><p>No authorization code received.</p></body></html>"
        };
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
            html.len()
        );
        let _ = stream.write_all(resp.as_bytes()).await;

        if let Some(code) = code {
            return Ok((code, state));
        }
    }
}
