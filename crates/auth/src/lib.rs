pub mod api_key;
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
            Credential::Bearer(token) => format!("Bearer {token}"),
        }
    }
}

pub fn resolve_credential(provider: &str, config_key: Option<&str>) -> Result<Credential> {
    if let Some(key) = config_key {
        return Ok(Credential::ApiKey(key.to_string()));
    }

    api_key::from_env(provider)
}
