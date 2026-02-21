use anyhow::Result;

use crate::{AuthError, Credential};

const ENV_VARS: &[(&str, &str)] = &[
    ("openai", "OPENAI_API_KEY"),
    ("anthropic", "ANTHROPIC_API_KEY"),
    ("gemini", "GEMINI_API_KEY"),
];

pub fn env_var_name(provider: &str) -> &str {
    ENV_VARS
        .iter()
        .find(|(p, _)| *p == provider)
        .map(|(_, v)| *v)
        .unwrap_or("UNKNOWN_API_KEY")
}

pub fn from_env(provider: &str) -> Result<Credential> {
    let var = env_var_name(provider);
    match std::env::var(var) {
        Ok(key) if !key.is_empty() => Ok(Credential::ApiKey(key)),
        _ => Err(AuthError::NoApiKey {
            provider: provider.to_string(),
            env_var: var.to_string(),
        }
        .into()),
    }
}
