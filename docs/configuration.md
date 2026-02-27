# Configuration

Source of truth: `crates/config/src/lib.rs` and merge call sites in `crates/cli/src/main.rs` / `crates/tui/src/app.rs`.

## Config Files

Primary files:

- global: `~/.config/nyzhi/config.toml`
- project: `<project>/.nyzhi/config.toml`

Also supported by parser helpers:

- local: `<project>/.nyzhi/config.local.toml` (`Config::load_local`)

Important caveat:

- current CLI/TUI runtime path merges `global + project` via `Config::merge`.
- `config.local.toml` is parsed by helpers but is not currently merged by default entrypoints.

## Merge Semantics (Global + Project)

`Config::merge(global, project)` is not a naive overwrite. It applies section-specific rules.

### High-level rules

- project values override global where explicitly set
- some lists are appended and deduplicated
- some booleans use `OR` semantics
- some booleans use `AND` semantics (more restrictive)
- update `release_url` is global-only for security

### Notable section behavior

- `provider.providers`: merged per provider entry (`api_key`, `base_url`, `model`, `api_style`, `max_tokens`, `temperature`)
- `provider.default`: project wins only if project default differs from built-in default (`openai`)
- `mcp.servers`: project extends global map
- `agent.trust.deny_tools` and `agent.trust.deny_paths`: union + dedupe
- `agent.hooks` and `agent.commands`: concatenated (`global` first, then `project`)
- `agent.agents.roles`: merged map (`project` can add/override role definitions)
- `browser.headless`: `project.headless && global.headless`
- `memory.auto_memory`: `project.auto_memory || global.auto_memory`
- `update.enabled`: `global.enabled && project.enabled`
- `index.enabled`: `global.enabled && project.enabled`
- `index.auto_context`: `global.auto_context && project.auto_context`
- `update.release_url`: taken from global only (project cannot override)

## Top-level Schema

`Config` contains:

- `provider: ProviderConfig`
- `models: ModelsConfig`
- `tui: TuiConfig`
- `agent: AgentSettings`
- `mcp: McpConfig`
- `external_notify: ExternalNotifyConfig`
- `shell: ShellConfig`
- `browser: BrowserConfig`
- `memory: MemoryConfig`
- `update: UpdateConfig`
- `index: IndexConfig`

## Example Config

```toml
[provider]
default = "openai"

[provider.openai]
model = "gpt-5.3-codex"

[tui]
theme = "dark"
accent = "copper"
show_thinking = true

[agent]
max_steps = 100
auto_compact_threshold = 0.8
subagent_model = "gpt-5.3-codex-xhigh-fast"

[agent.trust]
mode = "limited"
deny_tools = ["git_commit"]
deny_paths = [".env", "secrets/"]

[agent.agents]
max_threads = 4
max_depth = 2

[index]
enabled = true
embedding = "auto"
auto_context = true
auto_context_chunks = 5
exclude = ["target/**", "node_modules/**"]
```

## Provider and Models

### `[provider]`

- `default`: default provider id
- `[provider.<id>]` entries:
  - `api_key`
  - `base_url`
  - `model`
  - `api_style`
  - `max_tokens`
  - `temperature`

Built-in providers are listed in `BUILT_IN_PROVIDERS`; see `docs/providers.md`.

### `[models]`

- `max_tokens` (default `4096`)
- `temperature` (optional)

## TUI Section

### `[tui]`

- `markdown` (default `true`)
- `streaming` (default `true`)
- `theme` (default `dark`)
- `accent` (default `copper`)
- `show_thinking` (default `true`)
- `output_style`: `normal|verbose|minimal|structured`

### `[tui.notify]`

- `bell` (default `true`)
- `desktop` (default `false`)
- `min_duration_ms` (default `5000`)

## Agent Section

### `[agent]`

- `max_steps`
- `max_tokens`
- `custom_instructions`
- `auto_compact_threshold`
- `compact_instructions`
- `enforce_todos`
- `auto_simplify`
- `auto_commit`
- `model_profile`
- `subagent_model`

### `[agent.trust]`

- `mode`: `off|limited|autoedit|full`
- `allow_tools`, `allow_paths`
- `deny_tools`, `deny_paths`
- `auto_approve`
- `always_ask`
- `remember_approvals`

Trust parser aliases accepted in CLI/config parser:

- `autoedit`, `auto_edit`, `auto-edit`

### `[agent.retry]`

- `max_retries` (default `3`)
- `initial_backoff_ms` (default `1000`)
- `max_backoff_ms` (default `30000`)

### `[agent.routing]`

- `enabled`
- `low_keywords`
- `high_keywords`

See `docs/routing.md`.

### `[agent.verify]`

- `checks`: list of `{ kind, command }`

### `[agent.agents]`

- `max_threads` (default `4`)
- `max_depth` (default `2`)
- `roles`: map of role definitions

Role definition keys (`AgentRoleToml`):

- `description`
- `config_file`
- `system_prompt`
- `model`
- `max_steps`
- `read_only`
- `allowed_tools`
- `disallowed_tools`

### `[agent.sharing]`

- `enabled`
- `pages_project`
- `domain`
- `redact_patterns`

### `[agent.voice]`

- `enabled`
- `api_key_env`
- `model`

## Hooks

Each `[[agent.hooks]]` item (`HookConfig`) supports:

- `event`
- `command`
- `hook_type`: `command|prompt|agent`
- `prompt`
- `instructions`
- `tools`
- `model`
- `pattern`
- `tool_name`
- `block`
- `timeout` (default `30`)

Supported hook events:

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

See `docs/hooks.md` for runtime behavior and block/feedback semantics.

## MCP

### `[mcp.servers.<name>]`

Two forms are supported:

- stdio:
  - `command`
  - `args`
  - `env`
- http:
  - `url`
  - `headers`

Also see `.mcp.json` compatibility in `docs/mcp.md`.

## Shell and Browser

### `[shell]`

- `path`
- `env` (map)
- `startup_commands` (array)
- `[shell.sandbox]`:
  - `enabled`
  - `allow_network`
  - `allow_read`
  - `allow_write`
  - `block_dotfiles` (default `true`)

### `[browser]`

- `enabled`
- `executable_path`
- `headless` (default `true`)

## Memory, Index, Update, Notifications

### `[memory]`

- `auto_memory` (default `true`)

### `[index]`

- `enabled` (default `true`)
- `embedding` (`auto|voyage|openai|perplexity|tfidf`)
- `embedding_model`
- `auto_context` (default `true`)
- `auto_context_chunks` (default `5`)
- `exclude` (glob-like patterns)

### `[update]`

- `enabled` (default `true`)
- `check_interval_hours` (default `4`)
- `release_url` (default `https://get.nyzhi.com`)

### `[external_notify]`

- `webhook_url`
- `telegram_bot_token`
- `telegram_chat_id`
- `discord_webhook_url`
- `slack_webhook_url`

## CLI Overrides That Affect Runtime Config

From `crates/cli/src/main.rs`:

- `--trust` mutates `config.agent.trust.mode` at runtime
- `exec --full_auto` sets `trust.mode=full` and `sandbox_level=workspace-write`
- `exec --sandbox` sets runtime `ToolContext.sandbox_level`

## Paths and Directories

- config dir: `~/.config/nyzhi/`
- data dir: `~/.local/share/nyzhi/` (platform-dependent via `dirs::data_dir()`)
- sessions: `<data_dir>/sessions/`

See also:

- `docs/memory.md`
- `docs/sessions.md`
- `docs/self-update.md`
