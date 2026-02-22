pub mod manifest;
pub mod loader;
pub mod manager;

use std::path::PathBuf;

/// Plugin scopes, from most specific to least.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginScope {
    Local,
    Project,
    User,
}

impl std::fmt::Display for PluginScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginScope::Local => write!(f, "local"),
            PluginScope::Project => write!(f, "project"),
            PluginScope::User => write!(f, "user"),
        }
    }
}

fn user_plugins_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nyzhi")
        .join("plugins")
}

fn project_plugins_dir(root: &std::path::Path) -> PathBuf {
    root.join(".nyzhi").join("plugins")
}

fn local_plugins_dir(root: &std::path::Path) -> PathBuf {
    root.join(".nyzhi").join("plugins.local")
}
