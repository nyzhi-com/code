use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub models: ModelsConfig,
    #[serde(default)]
    pub tui: TuiConfig,
    #[serde(default)]
    pub agent: AgentSettings,
    #[serde(default)]
    pub mcp: McpConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpServerConfig {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    Http {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSettings {
    #[serde(default)]
    pub max_steps: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub custom_instructions: Option<String>,
    #[serde(default)]
    pub trust: TrustConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustConfig {
    #[serde(default)]
    pub mode: TrustMode,
    #[serde(default)]
    pub allow_tools: Vec<String>,
    #[serde(default)]
    pub allow_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    #[default]
    Off,
    Limited,
    Full,
}

impl std::fmt::Display for TrustMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustMode::Off => write!(f, "off"),
            TrustMode::Limited => write!(f, "limited"),
            TrustMode::Full => write!(f, "full"),
        }
    }
}

impl std::str::FromStr for TrustMode {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(TrustMode::Off),
            "limited" => Ok(TrustMode::Limited),
            "full" => Ok(TrustMode::Full),
            other => Err(format!("unknown trust mode: {other} (use off, limited, or full)")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default = "default_provider")]
    pub default: String,
    #[serde(default)]
    pub openai: ProviderEntry,
    #[serde(default)]
    pub anthropic: ProviderEntry,
    #[serde(default)]
    pub gemini: ProviderEntry,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    #[serde(default = "default_true")]
    pub markdown: bool,
    #[serde(default = "default_true")]
    pub streaming: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_accent")]
    pub accent: String,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_accent() -> String {
    "copper".to_string()
}

impl ProviderConfig {
    pub fn entry(&self, name: &str) -> Option<&ProviderEntry> {
        match name {
            "openai" => Some(&self.openai),
            "anthropic" => Some(&self.anthropic),
            "gemini" => Some(&self.gemini),
            _ => None,
        }
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            default: default_provider(),
            openai: ProviderEntry::default(),
            anthropic: ProviderEntry::default(),
            gemini: ProviderEntry::default(),
        }
    }
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            max_tokens: default_max_tokens(),
            temperature: None,
        }
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            markdown: true,
            streaming: true,
            theme: default_theme(),
            accent: default_accent(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content =
                std::fs::read_to_string(&path).context("Failed to read config file")?;
            toml::from_str(&content).context("Failed to parse config file")
        } else {
            Ok(Self::default())
        }
    }

    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nyzhi")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nyzhi")
    }

    pub fn ensure_dirs() -> Result<()> {
        std::fs::create_dir_all(Self::config_dir())?;
        std::fs::create_dir_all(Self::data_dir())?;
        Ok(())
    }

    pub fn load_project(project_root: &std::path::Path) -> Result<Option<Self>> {
        let path = project_root.join(".nyzhi").join("config.toml");
        if path.exists() {
            let content =
                std::fs::read_to_string(&path).context("Failed to read project config")?;
            let config: Config =
                toml::from_str(&content).context("Failed to parse project config")?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    pub fn merge(global: &Config, project: &Config) -> Config {
        let provider = ProviderConfig {
            default: if project.provider.default != default_provider() {
                project.provider.default.clone()
            } else {
                global.provider.default.clone()
            },
            openai: merge_provider_entry(&global.provider.openai, &project.provider.openai),
            anthropic: merge_provider_entry(
                &global.provider.anthropic,
                &project.provider.anthropic,
            ),
            gemini: merge_provider_entry(&global.provider.gemini, &project.provider.gemini),
        };

        let mut mcp_servers = global.mcp.servers.clone();
        mcp_servers.extend(project.mcp.servers.clone());

        Config {
            provider,
            models: ModelsConfig {
                max_tokens: if project.models.max_tokens != default_max_tokens() {
                    project.models.max_tokens
                } else {
                    global.models.max_tokens
                },
                temperature: project.models.temperature.or(global.models.temperature),
            },
            tui: global.tui.clone(),
            agent: AgentSettings {
                max_steps: project.agent.max_steps.or(global.agent.max_steps),
                max_tokens: project.agent.max_tokens.or(global.agent.max_tokens),
                custom_instructions: project
                    .agent
                    .custom_instructions
                    .clone()
                    .or_else(|| global.agent.custom_instructions.clone()),
                trust: if project.agent.trust.mode != TrustMode::Off {
                    project.agent.trust.clone()
                } else {
                    global.agent.trust.clone()
                },
            },
            mcp: McpConfig {
                servers: mcp_servers,
            },
        }
    }
}

fn merge_provider_entry(global: &ProviderEntry, project: &ProviderEntry) -> ProviderEntry {
    ProviderEntry {
        api_key: project.api_key.clone().or_else(|| global.api_key.clone()),
        base_url: project.base_url.clone().or_else(|| global.base_url.clone()),
        model: project.model.clone().or_else(|| global.model.clone()),
    }
}
