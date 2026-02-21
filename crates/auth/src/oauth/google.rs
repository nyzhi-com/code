use anyhow::{Context, Result};
use oauth2::{CsrfToken, PkceCodeChallenge};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::token_store::{self, StoredToken};

const CLIENT_ID: &str = "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/generative-language.retriever";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

pub async fn login() -> Result<StoredToken> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/oauth2callback");

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let csrf_state = CsrfToken::new_random();

    let auth_url = format!(
        "{AUTH_URL}?client_id={CLIENT_ID}\
         &redirect_uri={redirect}\
         &response_type=code\
         &scope={SCOPE}\
         &code_challenge={challenge}\
         &code_challenge_method=S256\
         &state={state}\
         &access_type=offline\
         &prompt=consent",
        redirect = urlencoding(&redirect_uri),
        challenge = pkce_challenge.as_str(),
        state = csrf_state.secret(),
    );

    eprintln!("Opening browser for Google login...");
    eprintln!("If the browser doesn't open, visit:\n  {auth_url}\n");

    if let Err(e) = open::that(&auth_url) {
        tracing::warn!(error = %e, "Failed to open browser");
    }

    let (code, state) = accept_callback(&listener).await?;

    if state != *csrf_state.secret() {
        anyhow::bail!("CSRF state mismatch -- possible attack or stale request");
    }

    eprintln!("Authorization code received. Exchanging for tokens...");

    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", CLIENT_ID),
            ("code", &code),
            ("code_verifier", pkce_verifier.secret()),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Token exchange failed: {body}");
    }

    let tokens: TokenResponse = resp.json().await?;

    let expires_at = tokens
        .expires_in
        .map(|secs| chrono::Utc::now().timestamp() + secs as i64);

    let stored = StoredToken {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at,
        provider: "gemini".to_string(),
    };

    token_store::store_token("gemini", &stored)?;
    eprintln!("Google/Gemini login successful. Token stored.");

    Ok(stored)
}

async fn accept_callback(listener: &TcpListener) -> Result<(String, String)> {
    let (mut stream, _) = listener
        .accept()
        .await
        .context("Failed to accept OAuth callback connection")?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");

    let query = path.split('?').nth(1).unwrap_or("");
    let params: std::collections::HashMap<&str, &str> = query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            Some((parts.next()?, parts.next()?))
        })
        .collect();

    let code = params
        .get("code")
        .ok_or_else(|| anyhow::anyhow!("No authorization code in callback"))?
        .to_string();
    let state = params.get("state").unwrap_or(&"").to_string();

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body><h2>Login successful!</h2>\
        <p>You can close this tab and return to the terminal.</p></body></html>";
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    Ok((code, state))
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}
