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
    #[serde(default)]
    pub external_notify: ExternalNotifyConfig,
    #[serde(default)]
    pub shell: ShellConfig,
    #[serde(default)]
    pub browser: BrowserConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShellConfig {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub startup_commands: Vec<String>,
    #[serde(default)]
    pub sandbox: SandboxSettings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allow_network: Vec<String>,
    #[serde(default)]
    pub allow_read: Vec<String>,
    #[serde(default)]
    pub allow_write: Vec<String>,
    #[serde(default = "default_true")]
    pub block_dotfiles: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BrowserConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub executable_path: Option<String>,
    #[serde(default = "default_true")]
    pub headless: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default)]
    pub auto_memory: bool,
}

fn default_check_interval_hours() -> u32 {
    4
}

fn default_release_url() -> String {
    "https://get.nyzhi.com".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_check_interval_hours")]
    pub check_interval_hours: u32,
    #[serde(default = "default_release_url")]
    pub release_url: String,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_hours: default_check_interval_hours(),
            release_url: default_release_url(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExternalNotifyConfig {
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub telegram_bot_token: Option<String>,
    #[serde(default)]
    pub telegram_chat_id: Option<String>,
    #[serde(default)]
    pub discord_webhook_url: Option<String>,
    #[serde(default)]
    pub slack_webhook_url: Option<String>,
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
    pub compact_instructions: Option<String>,
    #[serde(default)]
    pub enforce_todos: bool,
    #[serde(default)]
    pub auto_simplify: bool,
    #[serde(default)]
    pub verify: VerifyConfig,
    #[serde(default)]
    pub agents: AgentManagerConfig,
}

fn default_max_agents() -> usize {
    4
}

fn default_max_agent_depth() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManagerConfig {
    #[serde(default = "default_max_agents")]
    pub max_threads: usize,
    #[serde(default = "default_max_agent_depth")]
    pub max_depth: u32,
    #[serde(default)]
    pub roles: HashMap<String, AgentRoleToml>,
}

impl Default for AgentManagerConfig {
    fn default() -> Self {
        Self {
            max_threads: default_max_agents(),
            max_depth: default_max_agent_depth(),
            roles: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentRoleToml {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub config_file: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_steps: Option<u32>,
    #[serde(default)]
    pub read_only: Option<bool>,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    pub disallowed_tools: Option<Vec<String>>,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoutingConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub low_keywords: Vec<String>,
    #[serde(default)]
    pub high_keywords: Vec<String>,
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
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub hook_type: HookType,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub block: bool,
    #[serde(default = "default_hook_timeout")]
    pub timeout: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    #[default]
    Command,
    Prompt,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    SessionStart,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    PermissionRequest,
    Notification,
    AfterEdit,
    AfterTurn,
    SubagentStart,
    SubagentEnd,
    CompactContext,
    WorktreeCreate,
    WorktreeRemove,
    ConfigChange,
    TeammateIdle,
    TaskCompleted,
}

impl std::fmt::Display for HookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookEvent::SessionStart => write!(f, "session_start"),
            HookEvent::UserPromptSubmit => write!(f, "user_prompt_submit"),
            HookEvent::PreToolUse => write!(f, "pre_tool_use"),
            HookEvent::PostToolUse => write!(f, "post_tool_use"),
            HookEvent::PostToolUseFailure => write!(f, "post_tool_use_failure"),
            HookEvent::PermissionRequest => write!(f, "permission_request"),
            HookEvent::Notification => write!(f, "notification"),
            HookEvent::AfterEdit => write!(f, "after_edit"),
            HookEvent::AfterTurn => write!(f, "after_turn"),
            HookEvent::SubagentStart => write!(f, "subagent_start"),
            HookEvent::SubagentEnd => write!(f, "subagent_end"),
            HookEvent::CompactContext => write!(f, "compact_context"),
            HookEvent::WorktreeCreate => write!(f, "worktree_create"),
            HookEvent::WorktreeRemove => write!(f, "worktree_remove"),
            HookEvent::ConfigChange => write!(f, "config_change"),
            HookEvent::TeammateIdle => write!(f, "teammate_idle"),
            HookEvent::TaskCompleted => write!(f, "task_completed"),
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
    #[serde(default)]
    pub deny_tools: Vec<String>,
    #[serde(default)]
    pub deny_paths: Vec<String>,
    #[serde(default)]
    pub auto_approve: Vec<String>,
    #[serde(default)]
    pub always_ask: Vec<String>,
    #[serde(default)]
    pub remember_approvals: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustMode {
    #[default]
    Off,
    Limited,
    AutoEdit,
    Full,
}

impl std::fmt::Display for TrustMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrustMode::Off => write!(f, "off"),
            TrustMode::Limited => write!(f, "limited"),
            TrustMode::AutoEdit => write!(f, "autoedit"),
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
            "autoedit" | "auto_edit" | "auto-edit" => Ok(TrustMode::AutoEdit),
            "full" => Ok(TrustMode::Full),
            other => Err(format!("unknown trust mode: {other} (use off, limited, autoedit, or full)")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderDef {
    pub id: &'static str,
    pub name: &'static str,
    pub env_var: &'static str,
    pub default_base_url: &'static str,
    pub api_style: &'static str,
    pub category: &'static str,
    pub supports_oauth: bool,
}

pub const BUILT_IN_PROVIDERS: &[ProviderDef] = &[
    ProviderDef { id: "openai", name: "OpenAI", env_var: "OPENAI_API_KEY",
                  default_base_url: "https://api.openai.com/v1",
                  api_style: "openai", category: "popular", supports_oauth: true },
    ProviderDef { id: "anthropic", name: "Anthropic", env_var: "ANTHROPIC_API_KEY",
                  default_base_url: "https://api.anthropic.com/v1",
                  api_style: "anthropic", category: "popular", supports_oauth: true },
    ProviderDef { id: "gemini", name: "Google Gemini", env_var: "GEMINI_API_KEY",
                  default_base_url: "https://generativelanguage.googleapis.com/v1beta",
                  api_style: "gemini", category: "popular", supports_oauth: true },
    ProviderDef { id: "cursor", name: "Cursor", env_var: "CURSOR_API_KEY",
                  default_base_url: "https://api2.cursor.sh",
                  api_style: "cursor", category: "popular", supports_oauth: true },
    ProviderDef { id: "openrouter", name: "OpenRouter", env_var: "OPENROUTER_API_KEY",
                  default_base_url: "https://openrouter.ai/api/v1",
                  api_style: "openai", category: "popular", supports_oauth: false },
    ProviderDef { id: "claude-sdk", name: "Claude Agent SDK", env_var: "ANTHROPIC_API_KEY",
                  default_base_url: "",
                  api_style: "claude-sdk", category: "agents", supports_oauth: false },
    ProviderDef { id: "codex", name: "OpenAI Codex CLI", env_var: "CODEX_API_KEY",
                  default_base_url: "",
                  api_style: "codex", category: "agents", supports_oauth: true },
    ProviderDef { id: "groq", name: "Groq", env_var: "GROQ_API_KEY",
                  default_base_url: "https://api.groq.com/openai/v1",
                  api_style: "openai", category: "other", supports_oauth: false },
    ProviderDef { id: "together", name: "Together AI", env_var: "TOGETHER_API_KEY",
                  default_base_url: "https://api.together.xyz/v1",
                  api_style: "openai", category: "other", supports_oauth: false },
    ProviderDef { id: "deepseek", name: "DeepSeek", env_var: "DEEPSEEK_API_KEY",
                  default_base_url: "https://api.deepseek.com/v1",
                  api_style: "openai", category: "other", supports_oauth: false },
    ProviderDef { id: "ollama", name: "Ollama (local)", env_var: "OLLAMA_API_KEY",
                  default_base_url: "http://localhost:11434/v1",
                  api_style: "openai", category: "other", supports_oauth: false },
    ProviderDef { id: "kimi", name: "Kimi (Moonshot)", env_var: "MOONSHOT_API_KEY",
                  default_base_url: "https://api.moonshot.ai/v1",
                  api_style: "openai", category: "other", supports_oauth: false },
    ProviderDef { id: "kimi-coding", name: "Kimi Coding Plan", env_var: "KIMI_CODING_API_KEY",
                  default_base_url: "https://api.kimi.com/coding",
                  api_style: "anthropic", category: "other", supports_oauth: false },
    ProviderDef { id: "minimax", name: "MiniMax", env_var: "MINIMAX_API_KEY",
                  default_base_url: "https://api.minimax.io/v1",
                  api_style: "openai", category: "other", supports_oauth: false },
    ProviderDef { id: "minimax-coding", name: "MiniMax Coding Plan", env_var: "MINIMAX_CODING_API_KEY",
                  default_base_url: "https://api.minimax.io/anthropic",
                  api_style: "anthropic", category: "other", supports_oauth: false },
    ProviderDef { id: "glm", name: "GLM (Z.ai)", env_var: "ZHIPU_API_KEY",
                  default_base_url: "https://api.z.ai/api/paas/v4",
                  api_style: "openai", category: "other", supports_oauth: false },
    ProviderDef { id: "glm-coding", name: "GLM Coding Plan", env_var: "ZHIPU_CODING_API_KEY",
                  default_base_url: "https://api.z.ai/api/coding/paas/v4",
                  api_style: "openai", category: "other", supports_oauth: false },
];

pub fn find_provider_def(id: &str) -> Option<&'static ProviderDef> {
    BUILT_IN_PROVIDERS.iter().find(|p| p.id == id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default = "default_provider")]
    pub default: String,
    #[serde(default, flatten)]
    pub providers: HashMap<String, ProviderEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderEntry {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_style: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputStyle {
    #[default]
    Normal,
    Verbose,
    Minimal,
    Structured,
}

impl std::fmt::Display for OutputStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputStyle::Normal => write!(f, "normal"),
            OutputStyle::Verbose => write!(f, "verbose"),
            OutputStyle::Minimal => write!(f, "minimal"),
            OutputStyle::Structured => write!(f, "structured"),
        }
    }
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
    #[serde(default)]
    pub output_style: OutputStyle,
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
        self.providers.get(name)
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            default: default_provider(),
            providers: HashMap::new(),
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
            output_style: OutputStyle::Normal,
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

    pub fn load_local(project_root: &std::path::Path) -> Result<Option<Self>> {
        let path = project_root.join(".nyzhi").join("config.local.toml");
        if path.exists() {
            let content =
                std::fs::read_to_string(&path).context("Failed to read local config")?;
            let config: Config =
                toml::from_str(&content).context("Failed to parse local config")?;
            Ok(Some(config))
        } else {
            Ok(None)
        }
    }

    pub fn merge(global: &Config, project: &Config) -> Config {
        let provider = {
            let mut merged = global.provider.providers.clone();
            for (k, proj_entry) in &project.provider.providers {
                let base = merged.remove(k).unwrap_or_default();
                merged.insert(k.clone(), merge_provider_entry(&base, proj_entry));
            }
            ProviderConfig {
                default: if project.provider.default != default_provider() {
                    project.provider.default.clone()
                } else {
                    global.provider.default.clone()
                },
                providers: merged,
            }
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
                trust: {
                    let base = if project.agent.trust.mode != TrustMode::Off {
                        project.agent.trust.clone()
                    } else {
                        global.agent.trust.clone()
                    };
                    let mut deny_tools = global.agent.trust.deny_tools.clone();
                    deny_tools.extend(project.agent.trust.deny_tools.clone());
                    deny_tools.sort();
                    deny_tools.dedup();
                    let mut deny_paths = global.agent.trust.deny_paths.clone();
                    deny_paths.extend(project.agent.trust.deny_paths.clone());
                    deny_paths.sort();
                    deny_paths.dedup();
                    TrustConfig {
                        deny_tools,
                        deny_paths,
                        ..base
                    }
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
                compact_instructions: project
                    .agent
                    .compact_instructions
                    .clone()
                    .or(global.agent.compact_instructions.clone()),
                enforce_todos: project.agent.enforce_todos || global.agent.enforce_todos,
                auto_simplify: project.agent.auto_simplify || global.agent.auto_simplify,
                verify: if !project.agent.verify.checks.is_empty() {
                    project.agent.verify.clone()
                } else {
                    global.agent.verify.clone()
                },
                agents: AgentManagerConfig {
                    max_threads: if project.agent.agents.max_threads != default_max_agents() {
                        project.agent.agents.max_threads
                    } else {
                        global.agent.agents.max_threads
                    },
                    max_depth: if project.agent.agents.max_depth != default_max_agent_depth() {
                        project.agent.agents.max_depth
                    } else {
                        global.agent.agents.max_depth
                    },
                    roles: {
                        let mut roles = global.agent.agents.roles.clone();
                        roles.extend(project.agent.agents.roles.clone());
                        roles
                    },
                },
            },
            mcp: McpConfig {
                servers: mcp_servers,
            },
            external_notify: ExternalNotifyConfig {
                webhook_url: project.external_notify.webhook_url.clone()
                    .or_else(|| global.external_notify.webhook_url.clone()),
                telegram_bot_token: project.external_notify.telegram_bot_token.clone()
                    .or_else(|| global.external_notify.telegram_bot_token.clone()),
                telegram_chat_id: project.external_notify.telegram_chat_id.clone()
                    .or_else(|| global.external_notify.telegram_chat_id.clone()),
                discord_webhook_url: project.external_notify.discord_webhook_url.clone()
                    .or_else(|| global.external_notify.discord_webhook_url.clone()),
                slack_webhook_url: project.external_notify.slack_webhook_url.clone()
                    .or_else(|| global.external_notify.slack_webhook_url.clone()),
            },
            shell: ShellConfig {
                path: project.shell.path.clone().or_else(|| global.shell.path.clone()),
                env: {
                    let mut env = global.shell.env.clone();
                    env.extend(project.shell.env.clone());
                    env
                },
                startup_commands: if !project.shell.startup_commands.is_empty() {
                    project.shell.startup_commands.clone()
                } else {
                    global.shell.startup_commands.clone()
                },
                sandbox: if project.shell.sandbox.enabled {
                    project.shell.sandbox.clone()
                } else {
                    global.shell.sandbox.clone()
                },
            },
            browser: BrowserConfig {
                enabled: project.browser.enabled || global.browser.enabled,
                executable_path: project.browser.executable_path.clone()
                    .or_else(|| global.browser.executable_path.clone()),
                headless: project.browser.headless && global.browser.headless,
            },
            memory: MemoryConfig {
                auto_memory: project.memory.auto_memory || global.memory.auto_memory,
            },
            update: UpdateConfig {
                enabled: global.update.enabled && project.update.enabled,
                check_interval_hours: if project.update.check_interval_hours != default_check_interval_hours() {
                    project.update.check_interval_hours
                } else {
                    global.update.check_interval_hours
                },
                // release_url is ONLY settable from global config — never from project config.
                // Prevents a malicious repo from redirecting updates to an attacker server.
                release_url: global.update.release_url.clone(),
            },
        }
    }
}

fn merge_provider_entry(global: &ProviderEntry, project: &ProviderEntry) -> ProviderEntry {
    ProviderEntry {
        api_key: project.api_key.clone().or_else(|| global.api_key.clone()),
        base_url: project.base_url.clone().or_else(|| global.base_url.clone()),
        model: project.model.clone().or_else(|| global.model.clone()),
        api_style: project.api_style.clone().or_else(|| global.api_style.clone()),
        max_tokens: project.max_tokens.or(global.max_tokens),
        temperature: project.temperature.or(global.temperature),
    }
}
