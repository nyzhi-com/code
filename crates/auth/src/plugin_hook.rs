use anyhow::Result;

use crate::Credential;

/// Describes an authentication method a plugin provides.
#[derive(Debug, Clone)]
pub enum AuthMethod {
    OAuth { label: String },
    ApiKey { label: String },
}

/// Result of starting an authorization flow.
#[derive(Debug, Clone)]
pub struct AuthorizationResult {
    /// URL to open in the browser, if any.
    pub auth_url: Option<String>,
    /// Device code to display, if any.
    pub device_code: Option<String>,
    /// State token for CSRF verification.
    pub state: Option<String>,
}

/// Trait that auth plugins implement to register custom auth providers.
pub trait AuthPlugin: Send + Sync {
    /// The provider ID this plugin handles (e.g. "gitlab", "bitbucket").
    fn provider_id(&self) -> &str;

    /// Human-readable provider name.
    fn provider_name(&self) -> &str;

    /// Available authentication methods.
    fn auth_methods(&self) -> Vec<AuthMethod>;

    /// Start an authorization flow for the given method.
    fn authorize(&self, method: &str) -> Result<AuthorizationResult>;

    /// Handle the OAuth callback code and return a credential.
    fn callback(&self, code: &str) -> Result<Credential>;

    /// Refresh an expired token. Return None if refresh is not supported.
    fn refresh(&self, access_token: &str) -> Result<Option<String>>;
}

/// Registry of auth plugins loaded at startup.
#[derive(Default)]
pub struct AuthPluginRegistry {
    plugins: Vec<Box<dyn AuthPlugin>>,
}

impl AuthPluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, plugin: Box<dyn AuthPlugin>) {
        self.plugins.push(plugin);
    }

    pub fn find(&self, provider_id: &str) -> Option<&dyn AuthPlugin> {
        self.plugins
            .iter()
            .find(|p| p.provider_id() == provider_id)
            .map(|p| p.as_ref())
    }

    pub fn providers(&self) -> Vec<(&str, &str)> {
        self.plugins
            .iter()
            .map(|p| (p.provider_id(), p.provider_name()))
            .collect()
    }
}
