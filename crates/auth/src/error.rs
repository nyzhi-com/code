#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("No API key found for provider '{provider}'. Set {env_var} or configure in ~/.config/nyzhi/config.toml")]
    NoApiKey {
        provider: String,
        env_var: String,
    },

    #[error("Token expired for provider '{0}'")]
    TokenExpired(String),

    #[error("OAuth error: {0}")]
    OAuthError(String),

    #[error("Keyring error: {0}")]
    KeyringError(String),
}
