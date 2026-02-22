<p align="center">
  <img src="https://raw.githubusercontent.com/nyzhi-com/code/master/docs/assets/nyzhi-combined-dark.svg" alt="nyzhi" width="260" />
</p>

<p align="center">
  <strong>A performance-optimized AI coding agent for the terminal, built in Rust.</strong>
</p>

<p align="center">
  <a href="https://nyzhi.com/docs">Docs</a> &middot;
  <a href="#install">Install</a> &middot;
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="docs/">Full Documentation</a>
</p>

---

## What is nyzhi?

Nyzhi is a terminal-native AI coding agent. You give it a task, it reads your codebase, writes code, runs commands, and verifies the result -- all inside a rich TUI or as a single non-interactive command. It ships as a single static binary with no runtime dependencies.

Multi-provider by design. Nyzhi works with OpenAI, Anthropic, Google Gemini, OpenRouter, DeepSeek, Groq, and any OpenAI-compatible endpoint. Switch providers mid-session, route prompts to the right model tier automatically, and keep costs visible with built-in analytics.

---

## Install

```bash
curl -fsSL https://get.nyzhi.com | sh
```

This installs the `nyz` binary to `~/.nyzhi/bin/` and adds it to your PATH. Your config, sessions, and OAuth tokens are never touched by installs or updates.

**Self-update:**

```bash
nyz update                   # check and apply
nyz update --force           # ignore throttle
nyz update --list-backups    # list rollback points
nyz update --rollback latest # rollback to previous
```

Updates are checked automatically when the TUI starts (every 4 hours by default). Every update backs up the current binary and verifies the new one before completing. If the new binary fails, it auto-rolls back.

**Build from source:**

```bash
cargo build --release
# Binary: target/release/nyz
```

Requires Rust 1.75+. See [docs/building.md](docs/building.md) for details.

---

## Quick Start

```bash
# 1. Set your API key (or use `nyz login <provider>` for OAuth)
export OPENAI_API_KEY="sk-..."

# 2. Launch the TUI
nyz

# 3. One-shot mode (non-interactive)
nyz run "explain this codebase"

# 4. Continue the most recent session
nyz --continue

# 5. Resume a specific session
nyz --session "refactor auth"
```

---

## Features

### Core

- **Multi-provider** -- OpenAI, Anthropic, Gemini, OpenRouter, DeepSeek, Groq, Kimi, MiniMax, GLM, and any OpenAI-compatible API
- **50+ built-in tools** -- file ops, git, grep, glob, bash, sub-agents, todo, verify, notepad, LSP, AST search, web fetch, browser automation, PR workflows
- **MCP support** -- connect external tool servers via stdio or HTTP (compatible with Claude Code / Codex `.mcp.json`)
- **Streaming** -- real-time token-by-token output with thinking/reasoning display
- **Prompt caching** -- Anthropic `cache_control`, OpenAI/Gemini automatic caching for lower costs
- **Retry logic** -- exponential backoff for 429/5xx with multi-account rate-limit rotation
- **Single binary** -- no runtime dependencies, no Docker, no node_modules

### Agent

- **Autopilot** -- `/autopilot <idea>` for fully autonomous 5-phase execution (expansion, planning, execution, QA, validation)
- **Team orchestration** -- `/team N <task>` spawns coordinated sub-agents with mailbox messaging and task boards
- **Iterative planning** -- planner/critic loop for complex tasks with persistent plans
- **Smart routing** -- auto-select model tier (low/medium/high) based on task complexity
- **Verification protocol** -- auto-detect build/test/lint for Rust, Node, Go, Python projects
- **Skill learning** -- `/learn` extracts reusable patterns from sessions
- **Notepad wisdom** -- persist learnings, decisions, issues across sessions
- **Magic keywords** -- `plan:`, `persist:`, `eco:`, `tdd:`, `review:`, `parallel:` prefixes

### TUI

- **8 theme presets** -- Nyzhi Dark, Nyzhi Light, Tokyo Night, Catppuccin Mocha, Dracula, Solarized Dark/Light, Gruvbox Dark
- **14 accent colors** -- copper, blue, orange, emerald, violet, rose, amber, cyan, red, pink, teal, indigo, lime, monochrome
- **Per-slot color overrides** -- customize any theme token via hex values in config
- **Syntax highlighting** -- code blocks and inline markdown via syntect
- **Tab completion** -- slash commands, `@`-mention file paths
- **Input history** -- persistent across sessions with Ctrl+R reverse search
- **Multi-line input** -- Alt+Enter for newlines, `/editor` for `$EDITOR`, bracketed paste
- **In-session search** -- `/search` with highlighted matches and Ctrl+N/P navigation

### Workflow

- **Session persistence** -- auto-save, resume, search, rename, delete
- **Hooks** -- run lint, format, or tests automatically after edits or turns
- **Custom commands** -- user-defined slash commands from `.nyzhi/commands/` and config
- **Conversation export** -- `/export` to markdown
- **Session replay** -- event-level replay of past sessions
- **Token analytics** -- track costs per provider/model with daily/weekly/monthly reports
- **Notifications** -- terminal bell, desktop notifications, webhook, Telegram, Discord, Slack
- **Deep init** -- `nyz deepinit` generates AGENTS.md with project analysis

### Security

- **Trust mode** -- `off` (approve everything), `limited` (approve by tool/path), `full` (auto-approve all)
- **Change tracking** -- every file modification tracked with diffs
- **Undo** -- `/undo` reverts the last change, `/undo all` reverts everything
- **Checksum-verified updates** -- SHA256 verification, integrity manifests, automatic rollback on failure

---

## Providers

| Provider | Auth | Models |
|----------|------|--------|
| **OpenAI** | API key or OAuth (device code) | GPT-5.3 Codex, GPT-5.2 Codex, GPT-5.2, o3, o4-mini |
| **Anthropic** | API key or OAuth (PKCE) | Claude Opus 4.6, Claude Sonnet 4.6, Claude Haiku 4.5 |
| **Gemini** | API key or OAuth (Google PKCE) | Gemini 3.1 Pro, Gemini 3 Flash, Gemini 3 Pro, Gemini 2.5 Flash |
| **OpenRouter** | API key | Any model on OpenRouter |
| **DeepSeek** | API key | DeepSeek models |
| **Groq** | API key | Groq-hosted models |
| **Kimi** | API key | Moonshot models |
| **MiniMax** | API key | MiniMax models |
| **GLM** | API key | ChatGLM models |
| **Custom** | API key | Any OpenAI-compatible endpoint |

```bash
# OAuth login
nyz login gemini
nyz login openai
nyz login anthropic

# API key (environment variable)
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GEMINI_API_KEY="AI..."

# Check auth status
nyz whoami
```

See [docs/providers.md](docs/providers.md) and [docs/authentication.md](docs/authentication.md) for full details.

---

## CLI Reference

```
nyz                              Launch interactive TUI
nyz run "<prompt>"               One-shot prompt (non-interactive)
nyz run -i img.png "<prompt>"    Attach image to prompt
nyz login <provider>             OAuth login (gemini, openai, anthropic)
nyz logout <provider>            Remove stored OAuth token
nyz whoami                       Show auth status for all providers
nyz config                       Show current configuration
nyz init                         Create .nyzhi/ project directory
nyz update                       Check for updates and self-update
nyz update --force               Force update check
nyz update --list-backups        List available backups
nyz update --rollback <path>     Rollback to a backup ("latest" or path)
nyz mcp add <name> -- cmd        Add stdio MCP server
nyz mcp add <name> --url <url>   Add HTTP MCP server
nyz mcp list                     List configured MCP servers
nyz mcp remove <name>            Remove an MCP server
nyz sessions [query]             List saved sessions
nyz export <id> [-o path]        Export session to markdown
nyz session delete <id>          Delete a saved session
nyz session rename <id> <title>  Rename a saved session
nyz stats                        Show all-time session statistics
nyz cost [daily|weekly|monthly]  Show cost report by period
nyz replay <id> [--filter]       Replay session event timeline
nyz deepinit                     Generate AGENTS.md from project analysis
nyz skills                       List learned skills
nyz wait                         Check rate limit status
nyz teams list                   List agent teams
nyz teams show <name>            Show team details
nyz teams delete <name>          Delete a team
nyz ci-fix [--log-file <path>]   Auto-fix CI failures
nyz uninstall [--yes]            Uninstall nyzhi
```

### Flags

| Flag | Description |
|------|-------------|
| `-p, --provider <name>` | Provider (openai, anthropic, gemini, etc.) |
| `-m, --model <id>` | Model ID |
| `-y, --trust <mode>` | Trust mode (off, limited, full) |
| `-c, --continue` | Resume most recent session |
| `-s, --session <query>` | Resume session by ID or title |
| `--team-name <name>` | Join agent team as lead |
| `--teammate-mode <mode>` | in-process or tmux (default: in-process) |

---

## TUI Commands

| Command | Description |
|---------|-------------|
| `/help` | Show all commands and shortcuts |
| `/model` | List or switch models |
| `/image <path>` | Attach image to next prompt |
| `/login` | Show OAuth login status |
| `/init` | Initialize `.nyzhi/` project config |
| `/mcp` | List connected MCP servers |
| `/commands` | List custom commands |
| `/hooks` | List configured hooks |
| `/clear` | Clear session |
| `/compact` | Compress conversation history |
| `/sessions [q]` | List saved sessions (optionally filter) |
| `/resume <id>` | Restore a saved session |
| `/session delete <id>` | Delete a saved session |
| `/session rename <t>` | Rename current session |
| `/theme` | Choose theme preset |
| `/accent` | Choose accent color |
| `/trust [mode]` | Show or set trust mode |
| `/editor` | Open `$EDITOR` for multi-line input |
| `/retry` | Resend the last prompt |
| `/undo` | Undo the last file change |
| `/undo all` | Undo all file changes |
| `/changes` | List file changes in session |
| `/export [path]` | Export conversation as markdown |
| `/search <query>` | Search session messages |
| `/notify` | Toggle notification settings |
| `/autopilot <idea>` | Start autonomous 5-phase execution |
| `/team N <task>` | Spawn N coordinated sub-agents |
| `/plan [name]` | List or view saved plans |
| `/persist` | Activate verify/fix loop mode |
| `/qa` | Activate autonomous QA cycling |
| `/verify` | Show detected verification checks |
| `/todo` | View current todo list |
| `/learn [name]` | List or create skill templates |
| `/notepad [topic]` | List or view notepad entries |
| `/quit` | Exit |

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Tab | Completion |
| Alt+Enter | Newline |
| Ctrl+R | History search |
| Ctrl+N / Ctrl+P | Search next / prev |
| Ctrl+T | Cycle theme |
| Ctrl+A | Cycle accent |
| Ctrl+L | Clear screen |
| PageUp / PageDown | Scroll |
| Ctrl+C | Exit |

---

## Tools

### File Operations

`read`, `write`, `edit`, `multi_edit`, `apply_patch`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, `delete_file`, `move_file`, `copy_file`, `create_dir`

### Shell

`bash` -- run commands with live streaming output

### Git

`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch` (read-only) | `git_commit`, `git_checkout` (require approval)

### Agent and Task Management

`task` (sub-agents), `todo_write`, `todo_read`, `notepad_write`, `notepad_read`, `update_plan`, `think`, `load_skill`, `tool_search`

### Code Analysis

`verify` (build/test/lint), `lsp_diagnostics`, `ast_search`, `lsp_goto_definition`, `lsp_find_references`, `lsp_hover`

### Web

`web_fetch`, `web_search`

### Browser Automation

`browser_open`, `browser_screenshot`, `browser_evaluate`

### Team Orchestration

`team_create`, `team_delete`, `send_message`, `task_create`, `task_update`, `task_list`, `team_list`, `read_inbox`

### PR Workflow

`create_pr`, `review_pr`

### Search

`semantic_search`, `fuzzy_find`

### Debug

`instrument`, `remove_instrumentation`, `tail_file`, `batch_apply`

### Memory

`memory_read`, `memory_write`

See [docs/tools.md](docs/tools.md) for full parameter documentation.

---

## Configuration

Global config: `~/.config/nyzhi/config.toml`
Project config: `.nyzhi/config.toml` (merges with global)
Local overrides: `.nyzhi/config.local.toml` (merges on top, gitignored)

```toml
# --- Provider -----------------------------------------------------------

[provider]
default = "anthropic"

[provider.openai]
model = "gpt-5.2-codex"
# api_key = "sk-..."          # or use OPENAI_API_KEY env var
# base_url = "https://..."    # for custom endpoints

[provider.anthropic]
model = "claude-sonnet-4-20250514"

[provider.gemini]
model = "gemini-2.5-flash"

# Custom OpenAI-compatible provider
# [provider.my-provider]
# base_url = "https://api.example.com/v1"
# api_key = "..."
# api_style = "openai"
# env_var = "MY_PROVIDER_API_KEY"

# --- Models -------------------------------------------------------------

[models]
max_tokens = 16384
# temperature = 0.7

# --- TUI ----------------------------------------------------------------

[tui]
theme = "nyzhi-dark"          # nyzhi-dark, nyzhi-light, tokyonight,
                              # catppuccin-mocha, dracula, solarized-dark,
                              # solarized-light, gruvbox-dark
accent = "copper"             # copper, blue, orange, emerald, violet, rose,
                              # amber, cyan, red, pink, teal, indigo, lime,
                              # monochrome

[tui.colors]                  # optional per-slot hex overrides
# bg_page = "#1a1b26"
# text_primary = "#c0caf5"

[tui.notify]
bell = true                   # terminal bell on turn complete
desktop = false               # desktop notification on turn complete
min_duration_ms = 5000        # only notify if turn took longer than this

# --- Update -------------------------------------------------------------

[update]
enabled = true
check_interval_hours = 4
# release_url = "https://get.nyzhi.com"

# --- Agent --------------------------------------------------------------

[agent]
max_steps = 100
# custom_instructions = "Always write tests first."
auto_compact_threshold = 0.8  # auto-compact at 80% context window
# enforce_todos = false
# auto_simplify = false

[agent.routing]
enabled = false               # auto-select model tier by prompt complexity
# low_keywords = ["typo", "rename"]
# high_keywords = ["architect", "refactor"]

[agent.trust]
mode = "off"                  # off, limited, full
# allow_tools = ["edit"]      # tools to auto-approve (limited mode)
# allow_paths = ["src/"]      # paths to auto-approve (limited mode)

[agent.retry]
max_retries = 3
initial_backoff_ms = 1000
max_backoff_ms = 30000

[[agent.hooks]]
event = "after_edit"
command = "cargo fmt -- {file}"
pattern = "*.rs"
timeout = 30

[[agent.hooks]]
event = "after_turn"
command = "cargo clippy --all -- -D warnings"
timeout = 60

[[agent.commands]]
name = "test"
prompt = "Write comprehensive tests for $ARGUMENTS"
description = "Generate tests for a module"

# --- MCP ----------------------------------------------------------------

[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path"]

# [mcp.servers.remote]
# url = "https://mcp.example.com"
# headers = { Authorization = "Bearer token" }

# --- Notifications (external) ------------------------------------------

# [notify]
# webhook = { url = "https://..." }
# telegram = { bot_token = "...", chat_id = "..." }
# discord = { webhook_url = "https://..." }
# slack = { webhook_url = "https://..." }
```

See [docs/configuration.md](docs/configuration.md) for the full reference.

### Project Rules

Place an `AGENTS.md` or `.nyzhi/rules.md` in your project root to give the agent project-specific instructions. Nyzhi also reads `.cursorrules` and `CLAUDE.md` for compatibility.

### Custom Commands

Define reusable prompt templates as slash commands:

```markdown
<!-- .nyzhi/commands/review.md -->
# Review code for issues
Review $ARGUMENTS for bugs, security issues, and improvements.
```

Then use as `/review src/main.rs`. See [docs/commands.md](docs/commands.md).

---

## Architecture

```
                    ┌──────────────┐
                    │  nyzhi (cli) │
                    │   bin: nyz   │
                    └──────┬───────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
   ┌───────────┐    ┌───────────┐    ┌─────────────┐
   │ nyzhi-tui │    │ nyzhi-core│    │nyzhi-provider│
   │  ratatui  │    │ agent loop│    │  LLM API    │
   └─────┬─────┘    └─────┬─────┘    └──────┬──────┘
         │                │                  │
         └────────┬───────┴──────────┬───────┘
                  │                  │
                  ▼                  ▼
           ┌───────────┐      ┌───────────┐
           │ nyzhi-auth│      │nyzhi-config│
           │ OAuth/keys│      │ TOML load  │
           └───────────┘      └───────────┘
```

| Crate | Role |
|-------|------|
| **nyzhi** (cli) | Binary entry point. CLI parsing, command dispatch, MCP/tool setup. |
| **nyzhi-core** | Agent loop, 50+ tools, sessions, workspace, MCP, planning, teams, hooks, skills, verification, analytics. |
| **nyzhi-provider** | LLM abstraction. OpenAI, Anthropic, Gemini implementations with streaming, thinking support, and model registry. |
| **nyzhi-tui** | Terminal UI. ratatui-based app with themes, syntax highlighting, completion, history, export. |
| **nyzhi-auth** | OAuth2 PKCE/device-code flows, API key resolution, token storage, multi-account rotation. |
| **nyzhi-config** | Configuration loading and merging. Global, project, and local config with provider definitions. |

See [docs/architecture.md](docs/architecture.md) for the full deep-dive.

---

## Data Locations

| What | Path | Touched by updates? |
|------|------|---------------------|
| Binary | `~/.nyzhi/bin/nyz` | Yes (replaced, old version backed up) |
| Global config | `~/.config/nyzhi/config.toml` | Never |
| Project config | `.nyzhi/config.toml` | Never |
| Sessions and history | `~/.local/share/nyzhi/sessions/` | Never |
| Analytics | `~/.local/share/nyzhi/analytics.jsonl` | Never |
| OAuth tokens | `~/.local/share/nyzhi/auth.json` | Never |
| Backups | `~/.nyzhi/backups/` | Pruned to last 3 |
| Memory | `~/.local/share/nyzhi/MEMORY.md` | Never |
| MCP config | `.mcp.json` (project root) | Never |

---

## Building from Source

```bash
# Prerequisites: Rust 1.75+
cargo build --release
```

The binary is at `target/release/nyz`. See [docs/building.md](docs/building.md).

**Run tests:**

```bash
cargo test
```

---

## Releasing

Releases are automated via GitHub Actions. Push a version tag to trigger cross-compilation for linux/darwin x x86_64/aarch64:

```bash
git tag v0.2.1
git push origin v0.2.1
```

The workflow builds, checksums, uploads to GitHub Releases and Cloudflare R2, and updates `latest.json`. See [docs/releasing.md](docs/releasing.md).

Required secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`.

---

## Documentation

Full documentation lives in the [`docs/`](docs/) directory:

- [Architecture](docs/architecture.md) -- crate graph, module map, agent lifecycle
- [Configuration](docs/configuration.md) -- every config section with types and defaults
- [Authentication](docs/authentication.md) -- OAuth flows, API keys, multi-account
- [Providers](docs/providers.md) -- all providers, models, thinking support
- [Tools](docs/tools.md) -- all 50+ tools with parameters
- [TUI](docs/tui.md) -- commands, shortcuts, themes, accents
- [MCP](docs/mcp.md) -- stdio/HTTP setup, tool naming
- [Sessions](docs/sessions.md) -- save, resume, search, export, replay
- [Teams](docs/teams.md) -- multi-agent orchestration
- [Autopilot](docs/autopilot.md) -- autonomous 5-phase execution
- [Hooks](docs/hooks.md) -- after_edit, after_turn, pre/post tool
- [Custom Commands](docs/commands.md) -- slash command templates
- [Skills](docs/skills.md) -- pattern learning
- [Verification](docs/verification.md) -- auto-detect build/test/lint
- [Model Routing](docs/routing.md) -- tier-based model selection
- [Notifications](docs/notifications.md) -- bell, desktop, external
- [Self-Update](docs/self-update.md) -- update, backup, rollback
- [Memory](docs/memory.md) -- persistent notepad and topics
- [Building](docs/building.md) -- build from source
- [Releasing](docs/releasing.md) -- CI/CD pipeline

---

## License

[GPL-3.0-or-later](https://www.gnu.org/licenses/gpl-3.0.html)
