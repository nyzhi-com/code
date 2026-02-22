# nyzhi code

A performance-optimized AI coding agent for the terminal, built in Rust.

## Install

```bash
curl -fsSL https://get.nyzhi.com | sh
```

This installs the `nyz` binary to `~/.nyzhi/bin/` and adds it to your PATH.
Your config (`~/.config/nyzhi/`), data (`~/.local/share/nyzhi/`), and OAuth tokens are never touched by installs or updates.

**Self-update:**

```bash
nyzhi update            # check and apply updates
nyzhi update --force    # force re-check ignoring throttle
nyzhi update --list-backups   # list available rollback points
nyzhi update --rollback latest  # rollback to previous version
```

Updates are checked automatically when the TUI starts (every 4 hours by default). A banner offers `[u] Update`, `[s] Skip`, or `[i] Ignore version`. Every update backs up the current binary and verifies the new one before completing. If the new binary fails, it auto-rolls back.

## Features

- **Multi-provider** -- OpenAI, Anthropic, and Google Gemini with streaming
- **Rich TUI** -- ratatui-based interface with 8 built-in themes, persistent theme/accent selection, ASCII logo, animated spinner
- **35+ built-in tools** -- file ops, git, grep, glob, bash, sub-agents, todo, verify, notepad, LSP, AST search
- **MCP support** -- connect external tool servers via stdio or HTTP
- **OAuth + API key auth** -- Google PKCE, OpenAI device code, or plain API keys
- **Session persistence** -- auto-save, resume, search, rename, delete sessions
- **Context management** -- token estimation, configurable auto-compaction, `@file` mentions
- **Smart model routing** -- auto-select model tier (low/medium/high) based on task complexity
- **Change tracking** -- undo/revert any file change, diff view on approval
- **Trust mode** -- auto-approve tools (off / limited / full)
- **Custom commands** -- user-defined slash commands from `.nyzhi/commands/` and config
- **Hooks** -- run lint, format, or tests automatically after edits or turns
- **Magic keywords** -- `plan:`, `persist:`, `eco:`, `tdd:`, `review:`, `parallel:` prefixes
- **Verification protocol** -- auto-detect build/test/lint for Rust, Node, Go, Python projects
- **Token analytics** -- track costs per provider/model, daily/weekly/monthly reports
- **Notepad wisdom** -- persist learnings, decisions, issues across sessions
- **Session replay** -- event-level replay of past sessions
- **Iterative planning** -- planner/critic loop for complex tasks
- **LSP integration** -- detect available language servers, structural AST search
- **External notifications** -- webhook, Telegram, Discord, Slack on turn complete
- **Deep init** -- `nyzhi deepinit` generates AGENTS.md with project analysis
- **Skill learning** -- `/learn` extracts reusable patterns from sessions
- **Team orchestration** -- `/team N <task>` spawns coordinated sub-agents
- **Autopilot** -- `/autopilot <idea>` for fully autonomous 5-phase execution
- **Auto-update** -- background update checks with one-key apply, backup, rollback
- **Multi-line input** -- Alt+Enter for newlines, `/editor` for `$EDITOR`, bracketed paste
- **Input history** -- persistent across sessions, Ctrl+R reverse search
- **Syntax highlighting** -- code blocks and inline markdown via syntect
- **Tab completion** -- slash commands, `@`-mention file paths
- **Conversation export** -- `/export` to save session as markdown
- **In-session search** -- `/search` with highlighted matches and Ctrl+N/P navigation
- **Completion notifications** -- terminal bell + desktop notifications when turns finish (configurable threshold)
- **Prompt caching** -- Anthropic cache_control + OpenAI/Gemini automatic caching for lower costs
- **Retry logic** -- exponential backoff for 429/5xx errors
- **Single binary** -- no runtime dependencies

## Quick Start

```bash
# Set your API key (or use `nyzhi login <provider>` for OAuth)
export OPENAI_API_KEY="sk-..."

# Launch the TUI
nyzhi

# One-shot mode
nyzhi run "explain this codebase"

# Continue the most recent session
nyzhi --continue

# Resume a specific session
nyzhi --session "refactor"
```

## CLI Reference

```
nyzhi                        Launch interactive TUI
nyzhi run "<prompt>"         One-shot prompt (non-interactive)
nyzhi run -i img.png "<p>"   Attach image to prompt
nyzhi login <provider>       OAuth login (gemini, openai)
nyzhi logout <provider>      Remove stored OAuth token
nyzhi whoami                 Show auth status for all providers
nyzhi config                 Show current configuration
nyzhi init                   Create .nyzhi/ project directory
nyzhi update                 Check for updates and self-update
nyzhi update --force         Force update check
nyzhi update --list-backups  List available backups
nyzhi update --rollback <p>  Rollback to a backup ("latest" or path)
nyzhi mcp add <name> -- cmd  Add stdio MCP server
nyzhi mcp add <name> --url   Add HTTP MCP server
nyzhi mcp list               List configured MCP servers
nyzhi mcp remove <name>      Remove an MCP server
nyzhi sessions [query]       List saved sessions
nyzhi export <id> [-o path]  Export session to markdown
nyzhi session delete <id>    Delete a saved session
nyzhi session rename <id> <t> Rename a saved session
nyzhi stats                  Show all-time session statistics
nyzhi cost [daily|weekly|monthly] Show cost report by period
nyzhi replay <id> [--filter] Replay session event timeline
nyzhi deepinit               Generate AGENTS.md from project analysis
nyzhi skills                 List learned skills
nyzhi wait                   Check rate limit status

Flags:
  -p, --provider <name>      Provider (openai, anthropic, gemini)
  -m, --model <id>           Model ID
  -y, --trust <mode>         Trust mode (off, limited, full)
  -c, --continue             Resume most recent session
  -s, --session <query>      Resume session by ID or title
```

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
| `/search <query>` | Search session (Ctrl+N/P to navigate, Esc to clear) |
| `/notify` | Show or toggle notification settings (bell, desktop, duration) |
| `/autopilot <idea>` | Start autonomous 5-phase execution |
| `/team N <task>` | Spawn N coordinated sub-agents |
| `/plan [name]` | List/view saved plans |
| `/persist` | Activate verify/fix loop mode |
| `/qa` | Activate autonomous QA cycling |
| `/verify` | Show detected verification checks |
| `/todo` | View current todo list |
| `/learn [name]` | List/create skill templates |
| `/notepad [plan]` | List/view notepad entries |
| `/quit` | Exit |

**Shortcuts:** Tab (completion), Alt+Enter (newline), Ctrl+R (history search), Ctrl+N/P (search next/prev), Ctrl+T (theme), Ctrl+A (accent), Ctrl+L (clear), PageUp/PageDown (scroll), Ctrl+C (exit)

## Built-in Tools

### File Operations
`read`, `write`, `edit`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, `delete_file`, `move_file`, `copy_file`, `create_dir`

### Shell
`bash` -- run commands with live streaming output

### Git
`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch` (read-only), `git_commit`, `git_checkout` (require approval)

### Agent
`task` -- delegate sub-tasks to child agents, `todowrite`, `todoread`, `notepad_write`, `notepad_read`

### Verification
`verify` -- run project build/test/lint checks with structured results

### LSP / AST
`lsp_diagnostics` -- detect available language servers, `ast_search` -- structural code pattern matching

## Configuration

Global config: `~/.config/nyzhi/config.toml`
Project config: `.nyzhi/config.toml` (overrides global)

```toml
[provider]
default = "anthropic"

[provider.openai]
model = "gpt-4.1"

[provider.anthropic]
model = "claude-sonnet-4-20250514"

[provider.gemini]
model = "gemini-2.5-flash"

[tui]
theme = "nyzhi-dark"  # nyzhi-dark, nyzhi-light, tokyonight, catppuccin-mocha,
                      # dracula, solarized-dark, solarized-light, gruvbox-dark
accent = "copper"     # copper, blue, orange, emerald, violet, rose, amber,
                      # cyan, red, pink, teal, indigo, lime, monochrome

[tui.colors]          # optional per-slot hex overrides (applied on top of preset)
# bg_page = "#1a1b26"
# text_primary = "#c0caf5"

[tui.notify]
bell = true           # terminal bell on turn complete (default: true)
desktop = false       # desktop notification on turn complete (default: false)
min_duration_ms = 5000  # only notify if turn took longer than this (default: 5000)

[update]
enabled = true             # check for updates on TUI start (default: true)
check_interval_hours = 4   # minimum hours between checks (default: 4)
release_url = "https://get.nyzhi.com"  # override for self-hosted releases

[agent]
max_steps = 100
custom_instructions = "Always write tests."
auto_compact_threshold = 0.8  # auto-compact at 80% context window
enforce_todos = false          # continue until all todos complete
auto_simplify = false          # simplify code after each turn

[agent.routing]
enabled = false                # auto-select model tier by prompt complexity

[agent.trust]
mode = "off"              # off, limited, full
allow_tools = ["edit"]    # tools to auto-approve (limited mode)
allow_paths = ["src/"]    # paths to auto-approve (limited mode)

[agent.retry]
max_retries = 3
initial_backoff_ms = 1000
max_backoff_ms = 30000

[[agent.hooks]]
event = "after_edit"               # run after file-modifying tools
command = "cargo fmt -- {file}"    # {file} is replaced with the changed path
pattern = "*.rs"                   # only for Rust files
timeout = 30                       # seconds

[[agent.hooks]]
event = "after_turn"               # run after each agent turn
command = "cargo clippy --all -- -D warnings"
timeout = 60
```

### Custom Commands

Define reusable prompt templates as slash commands. Two methods:

**Markdown files** in `.nyzhi/commands/`:

```markdown
<!-- .nyzhi/commands/review.md -->
# Review code for issues
Review $ARGUMENTS for bugs, security issues, and improvements.
```

Then use as `/review src/main.rs` -- `$ARGUMENTS` is replaced with everything after the command name.

**Inline in config:**

```toml
[[agent.commands]]
name = "test"
prompt = "Write comprehensive tests for $ARGUMENTS"
description = "Generate tests for a module"
```

Config entries override file-based commands with the same name. List all with `/commands`.

### MCP Servers

Configure in `config.toml` or `.mcp.json` at project root:

```toml
[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path"]

[mcp.servers.remote]
url = "https://mcp.example.com"
headers = { Authorization = "Bearer token" }
```

### Project Rules

Place an `AGENTS.md` or `.nyzhi/rules.md` in your project root to give the agent project-specific instructions.

## Data Locations

| What | Path | Touched by updates? |
|------|------|---------------------|
| Binary | `~/.nyzhi/bin/nyz` | Yes (replaced, old version backed up) |
| Config | `~/.config/nyzhi/config.toml` | **Never** |
| Project config | `.nyzhi/config.toml` | **Never** |
| Sessions & history | `~/.local/share/nyzhi/` | **Never** |
| OAuth tokens | OS keyring | **Never** |
| Backups | `~/.local/share/nyzhi/backups/` | Pruned to last 3 |

## Building from Source

```bash
cargo build --release
```

The binary is at `target/release/nyz`.

## Releasing

Releases are automated via GitHub Actions. Push a version tag to trigger:

```bash
git tag v0.2.0
git push origin v0.2.0
```

The workflow cross-compiles for linux/darwin x x86_64/aarch64, uploads to R2, and creates a GitHub release.

Required GitHub secrets: `CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`.

## License

GPL-3.0-or-later
