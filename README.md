# nyzhi code

A performance-optimized AI coding agent for the terminal, built in Rust.

## Features

- **Multi-provider** -- OpenAI, Anthropic, and Google Gemini with streaming
- **Rich TUI** -- ratatui-based interface with themes, ASCII logo, animated spinner
- **30+ built-in tools** -- file ops, git, grep, glob, bash, sub-agents, todo
- **MCP support** -- connect external tool servers via stdio or HTTP
- **OAuth + API key auth** -- Google PKCE, OpenAI device code, or plain API keys
- **Session persistence** -- auto-save, resume, search, rename, delete sessions
- **Context management** -- token estimation, auto-compaction, `@file` mentions
- **Change tracking** -- undo/revert any file change, diff view on approval
- **Trust mode** -- auto-approve tools (off / limited / full)
- **Custom commands** -- user-defined slash commands from `.nyzhi/commands/` and config
- **Hooks** -- run lint, format, or tests automatically after edits or turns
- **Multi-line input** -- Alt+Enter for newlines, `/editor` for `$EDITOR`, bracketed paste
- **Input history** -- persistent across sessions, Ctrl+R reverse search
- **Syntax highlighting** -- code blocks and inline markdown via syntect
- **Tab completion** -- slash commands, `@`-mention file paths
- **Conversation export** -- `/export` to save session as markdown
- **In-session search** -- `/search` with highlighted matches and Ctrl+N/P navigation
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
nyzhi mcp add <name> -- cmd  Add stdio MCP server
nyzhi mcp add <name> --url   Add HTTP MCP server
nyzhi mcp list               List configured MCP servers
nyzhi mcp remove <name>      Remove an MCP server

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
| `/theme` | Choose theme (dark/light) |
| `/accent` | Choose accent color |
| `/trust [mode]` | Show or set trust mode |
| `/editor` | Open `$EDITOR` for multi-line input |
| `/retry` | Resend the last prompt |
| `/undo` | Undo the last file change |
| `/undo all` | Undo all file changes |
| `/changes` | List file changes in session |
| `/export [path]` | Export conversation as markdown |
| `/search <query>` | Search session (Ctrl+N/P to navigate, Esc to clear) |
| `/quit` | Exit |

**Shortcuts:** Tab (completion), Alt+Enter (newline), Ctrl+R (history search), Ctrl+N/P (search next/prev), Ctrl+T (theme), Ctrl+A (accent), Ctrl+L (clear), Ctrl+C (exit)

## Built-in Tools

### File Operations
`read`, `write`, `edit`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, `delete_file`, `move_file`, `copy_file`, `create_dir`

### Shell
`bash` -- run commands with live streaming output

### Git
`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch` (read-only), `git_commit`, `git_checkout` (require approval)

### Agent
`task` -- delegate sub-tasks to child agents, `todowrite`, `todoread`

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
theme = "dark"       # "dark" or "light"
accent = "copper"    # copper, blue, orange, emerald, violet, rose, amber,
                     # cyan, red, pink, teal, indigo, lime, monochrome

[agent]
max_steps = 100
custom_instructions = "Always write tests."

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

## Building

```bash
cargo build --release
```

The binary is at `target/release/nyzhi`.

## License

GPL-3.0-or-later
