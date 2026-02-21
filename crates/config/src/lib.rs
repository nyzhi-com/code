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
    #[serde(default)]
    pub retry: RetrySettings,
    #[serde(default)]
    pub hooks: Vec<HookConfig>,
    #[serde(default)]
    pub commands: Vec<CommandConfig>,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default)]
    pub auto_compact_threshold: Option<f64>,
    #[serde(default)]
    pub enforce_todos: bool,
    #[serde(default)]
    pub auto_simplify: bool,
    #[serde(default)]
    pub verify: VerifyConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifyConfig {
    #[serde(default)]
    pub checks: Vec<VerifyCheckConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyCheckConfig {
    pub kind: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub low_keywords: Vec<String>,
    #[serde(default)]
    pub high_keywords: Vec<String>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            low_keywords: vec![],
            high_keywords: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_hook_timeout() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub event: HookEvent,
    pub command: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default = "default_hook_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    AfterEdit,
    AfterTurn,
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookEvent::AfterEdit => write!(f, "after_edit"),
            HookEvent::AfterTurn => write!(f, "after_turn"),
        }
    }
}

fn default_max_retries() -> u32 {
    3
}

fn default_initial_backoff_ms() -> u64 {
    1000
}

fn default_max_backoff_ms() -> u64 {
    30000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySettings {
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_initial_backoff_ms")]
    pub initial_backoff_ms: u64,
    #[serde(default = "default_max_backoff_ms")]
    pub max_backoff_ms: u64,
}

impl Default for RetrySettings {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_backoff_ms: default_initial_backoff_ms(),
            max_backoff_ms: default_max_backoff_ms(),
        }
    }
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
    #[serde(default)]
    pub colors: ThemeOverrides,
    #[serde(default)]
    pub notify: NotifyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyConfig {
    #[serde(default = "default_true")]
    pub bell: bool,
    #[serde(default)]
    pub desktop: bool,
    #[serde(default = "default_min_duration_ms")]
    pub min_duration_ms: u64,
}

fn default_min_duration_ms() -> u64 {
    5000
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            bell: true,
            desktop: false,
            min_duration_ms: default_min_duration_ms(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeOverrides {
    pub bg_page: Option<String>,
    pub bg_surface: Option<String>,
    pub bg_elevated: Option<String>,
    pub bg_sunken: Option<String>,
    pub text_primary: Option<String>,
    pub text_secondary: Option<String>,
    pub text_tertiary: Option<String>,
    pub text_disabled: Option<String>,
    pub border_default: Option<String>,
    pub border_strong: Option<String>,
    pub accent: Option<String>,
    pub accent_muted: Option<String>,
    pub success: Option<String>,
    pub danger: Option<String>,
    pub warning: Option<String>,
    pub info: Option<String>,
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
            colors: ThemeOverrides::default(),
            notify: NotifyConfig::default(),
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

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        Self::ensure_dirs()?;
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        std::fs::write(&path, content).context("Failed to write config file")?;
        Ok(())
    }

    pub fn save_tui_preferences(theme: &str, accent: &str) -> Result<()> {
        let mut config = Self::load()?;
        config.tui.theme = theme.to_string();
        config.tui.accent = accent.to_string();
        config.save()
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
                retry: RetrySettings {
                    max_retries: if project.agent.retry.max_retries != default_max_retries() {
                        project.agent.retry.max_retries
                    } else {
                        global.agent.retry.max_retries
                    },
                    initial_backoff_ms: if project.agent.retry.initial_backoff_ms
                        != default_initial_backoff_ms()
                    {
                        project.agent.retry.initial_backoff_ms
                    } else {
                        global.agent.retry.initial_backoff_ms
                    },
                    max_backoff_ms: if project.agent.retry.max_backoff_ms
                        != default_max_backoff_ms()
                    {
                        project.agent.retry.max_backoff_ms
                    } else {
                        global.agent.retry.max_backoff_ms
                    },
                },
                hooks: {
                    let mut hooks = global.agent.hooks.clone();
                    hooks.extend(project.agent.hooks.clone());
                    hooks
                },
                commands: {
                    let mut cmds = global.agent.commands.clone();
                    cmds.extend(project.agent.commands.clone());
                    cmds
                },
                routing: if project.agent.routing.enabled {
                    project.agent.routing.clone()
                } else {
                    global.agent.routing.clone()
                },
                auto_compact_threshold: project
                    .agent
                    .auto_compact_threshold
                    .or(global.agent.auto_compact_threshold),
                enforce_todos: project.agent.enforce_todos || global.agent.enforce_todos,
                auto_simplify: project.agent.auto_simplify || global.agent.auto_simplify,
                verify: if !project.agent.verify.checks.is_empty() {
                    project.agent.verify.clone()
                } else {
                    global.agent.verify.clone()
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
