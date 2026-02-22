# Configuration

Nyzhi loads configuration from three layers, merged in order (later values override earlier):

1. **Global**: `~/.config/nyzhi/config.toml`
2. **Project**: `.nyzhi/config.toml` (in your project root)
3. **Local**: `.nyzhi/config.local.toml` (same directory, intended for gitignored overrides)

All sections are optional. Nyzhi works with no config file at all -- just set an API key via environment variable.

---

## Provider

```toml
[provider]
default = "openai"             # which provider to use by default
```

### Per-Provider Settings

```toml
[provider.openai]
model = "gpt-5.2-codex"       # default model for this provider
# api_key = "sk-..."          # inline API key (env var preferred)
# base_url = "https://..."    # custom endpoint

[provider.anthropic]
model = "claude-sonnet-4-20250514"

[provider.gemini]
model = "gemini-2.5-flash"
```

### Custom Providers

Any OpenAI-compatible endpoint can be added:

```toml
[provider.my-llm]
base_url = "https://api.example.com/v1"
api_key = "..."
api_style = "openai"          # openai, anthropic, or gemini
env_var = "MY_LLM_API_KEY"   # environment variable name for API key
```

### Built-In Provider Definitions

Nyzhi ships with definitions for: `openai`, `anthropic`, `gemini`, `openrouter`, `deepseek`, `groq`, `kimi`, `minimax`, `glm`. Each defines a default `base_url`, `env_var`, and `api_style`.

---

## Models

```toml
[models]
max_tokens = 16384             # max output tokens per response
# temperature = 0.7            # sampling temperature (provider default if unset)
```

---

## TUI

```toml
[tui]
theme = "nyzhi-dark"           # theme preset name
accent = "copper"              # accent color name
# output_style = "streaming"   # streaming or block
```

### Theme Presets

`nyzhi-dark`, `nyzhi-light`, `tokyonight`, `catppuccin-mocha`, `dracula`, `solarized-dark`, `solarized-light`, `gruvbox-dark`

### Accent Colors

`copper`, `blue`, `orange`, `emerald`, `violet`, `rose`, `amber`, `cyan`, `red`, `pink`, `teal`, `indigo`, `lime`, `monochrome`

### Color Overrides

Override any theme slot with a hex color:

```toml
[tui.colors]
bg_page = "#1a1b26"
bg_surface = "#1f2335"
bg_elevated = "#24283b"
bg_sunken = "#16161e"
text_primary = "#c0caf5"
text_secondary = "#a9b1d6"
text_tertiary = "#565f89"
text_disabled = "#3b4261"
border_default = "#292e42"
border_strong = "#3b4261"
accent = "#7aa2f7"
accent_muted = "#3d59a1"
success = "#9ece6a"
danger = "#f7768e"
warning = "#e0af68"
info = "#7aa2f7"
```

### Notifications

```toml
[tui.notify]
bell = true                    # terminal bell on turn complete (default: true)
desktop = false                # desktop notification via notify-rust (default: false)
min_duration_ms = 5000         # only notify if turn took longer than this (default: 5000)
```

---

## Update

```toml
[update]
enabled = true                 # check for updates on TUI start (default: true)
check_interval_hours = 4       # minimum hours between checks (default: 4)
release_url = "https://get.nyzhi.com"  # override for self-hosted releases
```

---

## Agent

```toml
[agent]
max_steps = 100                # max tool-call iterations per turn (default: 100)
custom_instructions = ""       # appended to system prompt
auto_compact_threshold = 0.8   # auto-compact at this fraction of context window (default: 0.8)
enforce_todos = false           # keep running until all todos complete
auto_simplify = false           # simplify code after each turn
```

### Trust

Controls tool approval behavior:

```toml
[agent.trust]
mode = "off"                   # off | limited | full
allow_tools = ["edit", "write"]  # tools auto-approved in limited mode
allow_paths = ["src/", "tests/"] # paths auto-approved in limited mode
```

| Mode | Behavior |
|------|----------|
| `off` | Every write/execute tool requires explicit approval. |
| `limited` | Tools in `allow_tools` for files in `allow_paths` are auto-approved. All others require approval. |
| `full` | All tools are auto-approved. |

### Retry

```toml
[agent.retry]
max_retries = 3                # max retry attempts for 429/5xx (default: 3)
initial_backoff_ms = 1000      # initial backoff duration (default: 1000)
max_backoff_ms = 30000         # max backoff cap (default: 30000)
```

### Routing

Auto-select model tier based on prompt complexity:

```toml
[agent.routing]
enabled = false                # enable auto-routing (default: false)
low_keywords = ["typo", "rename", "format"]    # additional low-tier keywords
high_keywords = ["architect", "design", "security"]  # additional high-tier keywords
```

When enabled, prompts are classified into `Low`, `Medium`, or `High` tiers based on keyword analysis and prompt length. The provider then selects the appropriate model.

### Hooks

```toml
[[agent.hooks]]
event = "after_edit"           # when to run
command = "cargo fmt -- {file}" # shell command ({file} replaced with changed path)
pattern = "*.rs"               # only for matching files (optional)
timeout = 30                   # seconds (default: 30)
# block = false                # if true, blocks the tool on non-zero exit

[[agent.hooks]]
event = "after_turn"
command = "cargo clippy --all -- -D warnings"
timeout = 60
```

#### Hook Events

| Event | Trigger | Context |
|-------|---------|---------|
| `after_edit` | After any file-modifying tool | `{file}` placeholder available |
| `after_turn` | After each complete agent turn | No file context |
| `pre_tool` | Before a tool executes | `tool_name` filter available |
| `post_tool` | After a tool succeeds | `tool_name` filter available |
| `post_tool_failure` | After a tool fails | `tool_name` filter available |
| `teammate_idle` | When a teammate has no work | Team context |
| `task_completed` | When a team task completes | Task context |

See [hooks.md](hooks.md) for full details.

### Commands

```toml
[[agent.commands]]
name = "test"
prompt = "Write comprehensive tests for $ARGUMENTS"
description = "Generate tests for a module"
```

See [commands.md](commands.md) for full details.

---

## MCP

### In config.toml

```toml
[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]

[mcp.servers.remote-api]
url = "https://mcp.example.com"
headers = { Authorization = "Bearer token" }
```

### In .mcp.json (project root)

Compatible with Claude Code and Codex format:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    },
    "remote": {
      "url": "https://mcp.example.com",
      "headers": {
        "Authorization": "Bearer token"
      }
    }
  }
}
```

See [mcp.md](mcp.md) for full details.

---

## Shell

```toml
[shell]
# default_shell = "/bin/bash"
# timeout = 120
```

## Browser

```toml
[browser]
# headless = true
```

## Memory

```toml
[memory]
# enabled = true
```

---

## External Notifications

```toml
[notify]
# webhook = { url = "https://hooks.example.com/nyzhi" }
# telegram = { bot_token = "123:ABC", chat_id = "-100123" }
# discord = { webhook_url = "https://discord.com/api/webhooks/..." }
# slack = { webhook_url = "https://hooks.slack.com/services/..." }
```

---

## Environment Variables

| Variable | Purpose | Used by |
|----------|---------|---------|
| `OPENAI_API_KEY` | OpenAI API key | provider |
| `ANTHROPIC_API_KEY` | Anthropic API key | provider |
| `GEMINI_API_KEY` | Google Gemini API key | provider |
| `OPENROUTER_API_KEY` | OpenRouter API key | provider |
| `DEEPSEEK_API_KEY` | DeepSeek API key | provider |
| `GROQ_API_KEY` | Groq API key | provider |
| `KIMI_API_KEY` | Moonshot/Kimi API key | provider |
| `MINIMAX_API_KEY` | MiniMax API key | provider |
| `GLM_API_KEY` | ChatGLM API key | provider |
| `NYZHI_HOME` | Override install directory (default: `~/.nyzhi`) | installer, updater |
| `EDITOR` / `VISUAL` | Editor for `/editor` command | TUI |
| `RUST_LOG` | Log level filter (e.g., `nyzhi=debug`) | tracing |

---

## Config Loading Order

1. Load global config from `~/.config/nyzhi/config.toml`.
2. Detect project root (walk up from CWD looking for `.nyzhi/`, `.claude/`, or `.git/`).
3. Load project config from `<project_root>/.nyzhi/config.toml`.
4. Load local config from `<project_root>/.nyzhi/config.local.toml`.
5. Merge: project overrides global, local overrides project.
6. CLI flags override everything.

Config directories are created automatically on first run via `Config::ensure_dirs()`.
