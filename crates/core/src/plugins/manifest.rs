use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: Option<AuthorInfo>,
    #[serde(default)]
    pub commands: Option<String>,
    #[serde(default)]
    pub agents: Option<Vec<String>>,
    #[serde(default)]
    pub skills: Option<String>,
    #[serde(default)]
    pub hooks: Option<String>,
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: Option<String>,
    #[serde(default)]
    pub settings: Option<String>,
    #[serde(default)]
    pub auth: Option<PluginAuthConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    #[serde(default)]
    pub url: Option<String>,
}

/// Auth extension point for plugins that provide custom auth providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthConfig {
    pub provider: String,
    #[serde(default)]
    pub methods: Vec<String>,
}

impl PluginManifest {
    /// Parse a plugin manifest from a .nyzhi-plugin/plugin.json file.
    pub fn load(plugin_root: &Path) -> Result<Self> {
        let manifest_path = plugin_root.join(".nyzhi-plugin").join("plugin.json");
        let content = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read manifest at {}", manifest_path.display()))?;

        let mut manifest: Self = serde_json::from_str(&content)
            .context("Failed to parse plugin manifest")?;

        let root_str = plugin_root.display().to_string();
        if let Some(ref mut cmds) = manifest.commands {
            *cmds = cmds.replace("${NYZHI_PLUGIN_ROOT}", &root_str);
        }
        if let Some(ref mut skills) = manifest.skills {
            *skills = skills.replace("${NYZHI_PLUGIN_ROOT}", &root_str);
        }
        if let Some(ref mut hooks) = manifest.hooks {
            *hooks = hooks.replace("${NYZHI_PLUGIN_ROOT}", &root_str);
        }
        if let Some(ref mut agents) = manifest.agents {
            for a in agents.iter_mut() {
                *a = a.replace("${NYZHI_PLUGIN_ROOT}", &root_str);
            }
        }

        Ok(manifest)
    }

    /// Resolve a relative path from the manifest against the plugin root.
    pub fn resolve_path(&self, plugin_root: &Path, relative: &str) -> PathBuf {
        plugin_root.join(relative)
    }
}

/// Detect if a directory is a plugin (has .nyzhi-plugin/plugin.json).
pub fn is_plugin_dir(dir: &Path) -> bool {
    dir.join(".nyzhi-plugin").join("plugin.json").exists()
}

/// Detect by conventional structure even without manifest.
pub fn has_conventional_structure(dir: &Path) -> bool {
    dir.join("commands").is_dir()
        || dir.join("agents").is_dir()
        || dir.join("skills").is_dir()
}
