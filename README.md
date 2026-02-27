# nyzhi

`nyzhi` is a terminal-first AI coding agent.

It combines a rich TUI, a one-shot CLI mode, multi-provider model support, session persistence, MCP integration, and a large built-in toolset for reading, editing, verifying, and shipping code.

## Install

```bash
curl -fsSL https://get.nyzhi.com | sh
```

Other install paths:

```bash
# Cargo
cargo install nyzhi

# npm
npm install -g nyzhi

# from source
git clone https://github.com/nyzhi-com/code
cd code
cargo build --release -p nyzhi
```

## Quick Start

```bash
# launch interactive TUI
nyz

# connect provider (default interactive flow)
/connect

# one-shot prompt (human-friendly text output)
nyz run "summarize this repository architecture"

# one-shot prompt (automation-friendly)
nyz exec --json "run tests and summarize failures"
```

## Core Workflows

- `nyz`: interactive TUI with slash commands, history, completion, selectors, and background tasks
- `/connect`: default in-TUI provider setup (OAuth first, API key fallback)
- `nyz run "<prompt>"`: non-interactive run
- `nyz exec [prompt]`: CI/scripting mode (reads stdin if piped)
- `nyz sessions`, `nyz session rename`, `nyz export`: session lifecycle management
- `nyz mcp add|list|remove`: MCP server configuration
- `nyz teams ...`: inspect and manage team metadata

## Trust and Sandbox Model

Trust mode controls approval behavior:

- `off`
- `limited`
- `autoedit`
- `full`

Sandbox level controls tool execution boundaries:

- `read-only`
- `workspace-write`
- `full-access`

`nyz exec --full_auto` forces trust mode to `full` and sandbox to `workspace-write`.

## Configuration Model

Primary config files:

- Global: `~/.config/nyzhi/config.toml`
- Project: `.nyzhi/config.toml`

The runtime merges global + project config (`Config::merge`).  
`config.local.toml` is supported by parsing helpers but is not currently part of the default CLI/TUI merge path.

Key sections:

- `[provider]` and `[provider.<name>]`
- `[models]`
- `[tui]`
- `[agent]` (`trust`, `retry`, `routing`, `agents`, `verify`, `sharing`, `voice`)
- `[mcp]`
- `[shell]`
- `[browser]`
- `[memory]`
- `[update]`
- `[index]`
- `[external_notify]`

## Providers

Built-in provider IDs include:

- `openai`
- `anthropic`
- `gemini`
- `cursor`
- `github-copilot`
- `openrouter`
- `groq`
- `together`
- `deepseek`
- `ollama`
- `kimi`, `kimi-coding`
- `minimax`, `minimax-coding`
- `glm`, `glm-coding`
- `claude-sdk`
- `codex`

See `docs/providers.md` for auth requirements, API styles, and model notes.

## Architecture Overview

```text
                    nyzhi (CLI binary)
                           |
            +--------------+---------------+
            |                              |
        nyzhi-tui                      nyzhi-core
                                           |
                     +---------------------+---------------------+
                     |                     |                     |
                nyzhi-provider         nyzhi-auth           nyzhi-index
                     |
                nyzhi-config
```

Crate responsibilities:

- `crates/cli`: command parsing and runtime wiring
- `crates/tui`: terminal UX and command handling
- `crates/core`: agent loop, tools, sessions, teams, hooks, memory, workspace
- `crates/provider`: model/provider abstraction and streaming
- `crates/auth`: API key/OAuth/token resolution
- `crates/config`: schema/defaults/merge rules
- `crates/index`: semantic index + auto-context search

## Documentation

The full local docs set lives in `docs/`.

- [Docs index](docs/README.md)
- [Architecture](docs/architecture.md)
- [Commands](docs/commands.md)
- [Configuration](docs/configuration.md)
- [Authentication](docs/authentication.md)
- [Providers](docs/providers.md)
- [Tools](docs/tools.md)
- [TUI](docs/tui.md)
- [MCP](docs/mcp.md)
- [Sessions](docs/sessions.md)
- [Teams](docs/teams.md)
- [Autopilot](docs/autopilot.md)
- [Hooks](docs/hooks.md)
- [Skills](docs/skills.md)
- [Memory](docs/memory.md)
- [Routing](docs/routing.md)
- [Verification](docs/verification.md)
- [Notifications](docs/notifications.md)
- [Self-Update](docs/self-update.md)
- [Building](docs/building.md)
- [Releasing](docs/releasing.md)

Coverage and source tracing:

- [Source-to-doc map](docs/_meta/source-map.md)

## Contributing

- [Contributing guide](CONTRIBUTING.md)
- [Support](SUPPORT.md)
- [Security policy](SECURITY.md)
- [Code of conduct](CODE_OF_CONDUCT.md)

License: [GPL-3.0-or-later](LICENSE)
