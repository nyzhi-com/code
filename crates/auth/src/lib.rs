pub mod api_key;
pub mod oauth;
pub mod plugin_hook;
pub mod token_store;

mod error;

pub use error::AuthError;

use anyhow::Result;

#[derive(Debug, Clone)]
pub enum Credential {
    ApiKey(String),
    Bearer(String),
}

impl Credential {
    pub fn header_value(&self) -> String {
        match self {
            Credential::ApiKey(key) => key.clone(),
            Credential::Bearer(token) => token.clone(),
        }
    }

    pub fn is_bearer(&self) -> bool {
        matches!(self, Credential::Bearer(_))
    }
}

pub fn resolve_credential(provider: &str, config_key: Option<&str>) -> Result<Credential> {
    if let Some(key) = config_key {
        return Ok(Credential::ApiKey(key.to_string()));
    }

    if let Ok(cred) = api_key::from_env(provider) {
        return Ok(cred);
    }

    if let Ok(Some(token)) = token_store::load_token(provider) {
        if token.refresh_token.is_some() {
            if !oauth::refresh::is_expired(&token) {
                return Ok(Credential::Bearer(token.access_token));
            }
        } else {
            return Ok(Credential::ApiKey(token.access_token));
        }
    }

    let env_var = api_key::env_var_name(provider);
    let oauth_hint = if oauth::supports_oauth(provider) {
        format!(", or run `nyzhi login {provider}`")
    } else {
        String::new()
    };

    Err(AuthError::NoCredential {
        provider: provider.to_string(),
        env_var: env_var.to_string(),
        oauth_hint,
    }
    .into())
}

pub async fn resolve_credential_async(
    provider: &str,
    config_key: Option<&str>,
) -> Result<Credential> {
    if let Some(key) = config_key {
        return Ok(Credential::ApiKey(key.to_string()));
    }

    if let Ok(cred) = api_key::from_env(provider) {
        return Ok(cred);
    }

    if let Ok(Some(token)) = oauth::refresh::refresh_if_needed(provider).await {
        return Ok(Credential::Bearer(token.access_token));
    }

    if let Ok(Some(token)) = token_store::load_token(provider) {
        return Ok(Credential::ApiKey(token.access_token));
    }

    let env_var = api_key::env_var_name(provider);
    let oauth_hint = if oauth::supports_oauth(provider) {
        format!(", or run `nyzhi login {provider}`")
    } else {
        String::new()
    };

    Err(AuthError::NoCredential {
        provider: provider.to_string(),
        env_var: env_var.to_string(),
        oauth_hint,
    }
    .into())
}

/// Call when a 429 rate limit is received; rotates to the next account if available.
pub fn handle_rate_limit(provider: &str) -> Result<Option<Credential>> {
    if let Ok(Some(token)) = token_store::rotate_on_rate_limit(provider, 60) {
        if token.refresh_token.is_some() {
            Ok(Some(Credential::Bearer(token.access_token)))
        } else {
            Ok(Some(Credential::ApiKey(token.access_token)))
        }
    } else {
        Ok(None)
    }
}

/// Returns the auth status string for a provider (for UI display).
pub fn auth_status(provider: &str) -> &'static str {
    if api_key::from_env(provider).is_ok() {
        return "env";
    }
    if let Ok(Some(_)) = token_store::load_token(provider) {
        return "connected";
    }
    "not connected"
}
