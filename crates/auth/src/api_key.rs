use anyhow::Result;

use crate::{AuthError, Credential};

pub fn env_var_name(provider: &str) -> String {
    if let Some(def) = nyzhi_config::find_provider_def(provider) {
        return def.env_var.to_string();
    }
    format!("{}_API_KEY", provider.to_uppercase().replace('-', "_"))
}

pub fn from_env(provider: &str) -> Result<Credential> {
    let var = env_var_name(provider);
    match std::env::var(&var) {
        Ok(key) if !key.is_empty() => Ok(Credential::ApiKey(key)),
        _ => Err(AuthError::NoApiKey {
            provider: provider.to_string(),
            env_var: var,
        }
        .into()),
    }
}
