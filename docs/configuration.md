# Configuration

This page documents the current `nyzhi-config` schema and what the CLI actually loads today.

## File locations

- Global config: `~/.config/nyzhi/config.toml`
- Project config: `<project_root>/.nyzhi/config.toml`
- Local helper path exists in code: `<project_root>/.nyzhi/config.local.toml`

### What is loaded now

- `nyz` currently loads global config, then merges project config **if** `.nyzhi/config.toml` exists.
- `config.local.toml` has a loader in `nyzhi-config`, but is not currently wired in CLI startup.

## Top-level sections

```toml
[provider]
[models]
[tui]
[agent]
[mcp]
[external_notify]
[shell]
[browser]
[memory]
[update]
```

All sections are optional.

## Defaults (source values)

- `provider.default = "openai"`
- `models.max_tokens = 4096`
- `models.temperature = null`
- `tui.theme = "dark"` (maps to `nyzhi-dark`)
- `tui.accent = "copper"`
- `tui.markdown = true`
- `tui.streaming = true`
- `tui.notify.bell = true`
- `tui.notify.desktop = false`
- `tui.notify.min_duration_ms = 5000`
- `agent.retry.max_retries = 3`
- `agent.retry.initial_backoff_ms = 1000`
- `agent.retry.max_backoff_ms = 30000`
- `agent.agents.max_threads = 4`
- `agent.agents.max_depth = 2`
- `update.enabled = true`
- `update.check_interval_hours = 4`
- `update.release_url = "https://get.nyzhi.com"`

## Provider config

```toml
[provider]
default = "openai"

[provider.openai]
model = "gpt-5.3-codex"
# api_key = "..."
# base_url = "https://api.openai.com/v1"
# api_style = "openai"
# max_tokens = 4096
# temperature = 0.2
```

`[provider.<id>]` fields:

- `api_key`
- `base_url`
- `model`
- `api_style`
- `max_tokens`
- `temperature`

### Built-in providers

| id | env var | api_style | oauth | default base URL |
|---|---|---|---|---|
| `openai` | `OPENAI_API_KEY` | `openai` | yes | `https://api.openai.com/v1` |
| `anthropic` | `ANTHROPIC_API_KEY` | `anthropic` | yes | `https://api.anthropic.com/v1` |
| `gemini` | `GEMINI_API_KEY` | `gemini` | yes | `https://generativelanguage.googleapis.com/v1beta` |
| `cursor` | `CURSOR_API_KEY` | `cursor` | yes | `https://api2.cursor.sh` |
| `openrouter` | `OPENROUTER_API_KEY` | `openai` | no | `https://openrouter.ai/api/v1` |
| `claude-sdk` | `ANTHROPIC_API_KEY` | `claude-sdk` | no | *(empty)* |
| `codex` | `CODEX_API_KEY` | `codex` | yes | *(empty)* |
| `groq` | `GROQ_API_KEY` | `openai` | no | `https://api.groq.com/openai/v1` |
| `together` | `TOGETHER_API_KEY` | `openai` | no | `https://api.together.xyz/v1` |
| `deepseek` | `DEEPSEEK_API_KEY` | `openai` | no | `https://api.deepseek.com/v1` |
| `ollama` | `OLLAMA_API_KEY` | `openai` | no | `http://localhost:11434/v1` |
| `kimi` | `MOONSHOT_API_KEY` | `openai` | no | `https://api.moonshot.ai/v1` |
| `kimi-coding` | `KIMI_CODING_API_KEY` | `anthropic` | no | `https://api.kimi.com/coding` |
| `minimax` | `MINIMAX_API_KEY` | `openai` | no | `https://api.minimax.io/v1` |
| `minimax-coding` | `MINIMAX_CODING_API_KEY` | `anthropic` | no | `https://api.minimax.io/anthropic` |
| `glm` | `ZHIPU_API_KEY` | `openai` | no | `https://api.z.ai/api/paas/v4` |
| `glm-coding` | `ZHIPU_CODING_API_KEY` | `openai` | no | `https://api.z.ai/api/coding/paas/v4` |

## TUI config

```toml
[tui]
theme = "dark"
accent = "copper"
markdown = true
streaming = true
output_style = "normal" # normal | verbose | minimal | structured

[tui.notify]
bell = true
desktop = false
min_duration_ms = 5000

[tui.colors]
bg_page = "#000000"
accent = "#c49a6c"
```

Theme names accepted: `dark`, `light`, `nyzhi-dark`, `nyzhi-light`, `tokyonight`, `catppuccin-mocha`, `dracula`, `solarized-dark`, `solarized-light`, `gruvbox-dark`.

Accent names: `copper`, `blue`, `orange`, `emerald`, `violet`, `rose`, `amber`, `cyan`, `red`, `pink`, `teal`, `indigo`, `lime`, `monochrome`.

## Agent config

```toml
[agent]
max_steps = 100
max_tokens = 8192
custom_instructions = "..."
auto_compact_threshold = 0.85
compact_instructions = "..."
enforce_todos = false
auto_simplify = false

[agent.trust]
mode = "limited" # off | limited | autoedit | full
allow_tools = ["edit", "write"]
allow_paths = ["src/"]
deny_tools = []
deny_paths = []
auto_approve = []
always_ask = []
remember_approvals = false

[agent.retry]
max_retries = 3
initial_backoff_ms = 1000
max_backoff_ms = 30000

[agent.routing]
enabled = false
low_keywords = []
high_keywords = []

[agent.verify]
checks = [{ kind = "test", command = "cargo test -q" }]
```

### Trust modes

- `off`: approval required for sensitive tools.
- `limited`: read-only tools auto-run; write/exec tools still gated unless allow-lists match.
- `autoedit`: auto-approves file-editing tools (`write`, `edit`, `multi_edit`, `apply_patch`, `delete_file`, `move_file`, `copy_file`, `create_dir`).
- `full`: auto-approves all tools (except denied via deny-lists).

### Hook events (exact names)

- `session_start`
- `user_prompt_submit`
- `pre_tool_use`
- `post_tool_use`
- `post_tool_use_failure`
- `permission_request`
- `notification`
- `after_edit`
- `after_turn`
- `subagent_start`
- `subagent_end`
- `compact_context`
- `worktree_create`
- `worktree_remove`
- `config_change`
- `teammate_idle`
- `task_completed`

See [hooks.md](hooks.md) for event payload behavior.

## MCP config

```toml
[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "."]

[mcp.servers.remote]
url = "https://mcp.example.com"
headers = { Authorization = "Bearer ..." }
```

`.mcp.json` is also loaded (Claude/Codex format). On name collisions, `.mcp.json` entries win at runtime because they are merged last.

## External notify

```toml
[external_notify]
webhook_url = "https://example.com/hook"
telegram_bot_token = "..."
telegram_chat_id = "..."
discord_webhook_url = "..."
slack_webhook_url = "..."
```

## Shell, browser, memory, update

```toml
[shell]
path = "/bin/zsh"
startup_commands = ["echo ready"]
env = { FOO = "bar" }

[shell.sandbox]
enabled = false
allow_network = []
allow_read = []
allow_write = []
block_dotfiles = true

[browser]
enabled = false
executable_path = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
headless = true

[memory]
auto_memory = false

[update]
enabled = true
check_interval_hours = 4
release_url = "https://get.nyzhi.com"
```

## Merge caveats

Current merge behavior is field-specific, not a universal "project always overrides global" rule:

- Provider/model/mcp/agent/shell settings merge field-by-field.
- `tui` currently comes from global config in merge logic.
- `update.release_url` is global-only by design (project config cannot override it).
- `browser.enabled` is merged with logical OR, while `browser.headless` is logical AND.

## Environment variables

- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GEMINI_API_KEY`
- `CURSOR_API_KEY`
- `OPENROUTER_API_KEY`
- `GROQ_API_KEY`
- `TOGETHER_API_KEY`
- `DEEPSEEK_API_KEY`
- `OLLAMA_API_KEY`
- `MOONSHOT_API_KEY`
- `KIMI_CODING_API_KEY`
- `MINIMAX_API_KEY`
- `MINIMAX_CODING_API_KEY`
- `ZHIPU_API_KEY`
- `ZHIPU_CODING_API_KEY`
- `CODEX_API_KEY`

## Boundary notes

- `target/`, `node_modules/`, and `.git/` are runtime/build VCS artifacts, not configuration authority.
- Product behavior should be documented from maintained source (`crates/*`, `Cargo.toml`, `.raccoon.toml`, and docs), not generated outputs.
