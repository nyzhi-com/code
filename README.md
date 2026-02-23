<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/hero.svg" />
    <source media="(prefers-color-scheme: light)" srcset="docs/assets/hero.svg" />
    <img src="docs/assets/hero.svg" alt="nyzhi" width="720" />
  </picture>
</p>

<p align="center">
  <a href="https://code.nyzhi.com/docs"><strong>Docs</strong></a> &ensp;&middot;&ensp;
  <a href="#install"><strong>Install</strong></a> &ensp;&middot;&ensp;
  <a href="#quick-start"><strong>Quick Start</strong></a> &ensp;&middot;&ensp;
  <a href="https://github.com/nyzhi-com/code/releases"><strong>Releases</strong></a>
</p>

<br>

> **Single binary. 50+ built-in tools. Terminal-first.**  
> You give nyzhi a task. It reads your codebase, makes edits, runs commands, verifies changes, and reports back -- in a rich TUI or a one-shot CLI run.

<br>

## Install

```bash
curl -fsSL https://get.nyzhi.com | sh
```

<details>
<summary><strong>Other install paths</strong></summary>

```bash
# Cargo
cargo install nyzhi

# npm
npm install -g nyzhi

# from source
git clone https://github.com/nyzhi-com/code
cd code
cargo build --release
```

</details>

<details>
<summary><strong>Self-update and rollback</strong></summary>

```bash
nyz update
nyz update --force
nyz update --list-backups
nyz update --rollback latest
```

</details>

<br>

## Quick Start

```bash
nyz login openai                  # OAuth sign-in (PKCE)
nyz                               # launch TUI
nyz run "summarize this project"  # non-interactive run
nyz --continue                    # resume last session
```

<br>

## Why nyzhi

<table>
<tr>
<td width="50%" valign="top">

### Multi-provider by default

OpenAI, Anthropic, Gemini, Cursor, OpenRouter, DeepSeek, Groq, Together, Ollama, Kimi, MiniMax, GLM -- all through one interface.

### Autonomous workflows

`/autopilot` for full multi-phase execution, `/team` for coordinated sub-agents, planner/critic loops for complex tasks, and replayable session timelines.

### Real tooling, not toy demos

File ops, git, shell, LSP/AST, semantic search, browser automation, PR tools, MCP servers, verification, and debug instrumentation.

</td>
<td width="50%" valign="top">

### TUI you can actually live in

8 theme presets, 14 accents, command palette, completion, history search, in-session search, export, notifications, and background task controls.

### Strong safety model

Trust modes (`off`, `limited`, `autoedit`, `full`), hook enforcement, approval gates for risky actions, undo support, and verified self-update with rollback.

### Learns your project

Custom commands, skills, memory, rules, and notepad support keep the agent aligned with your conventions.

</td>
</tr>
</table>

<br>

## Providers

| Provider | Auth | Notes |
|:---|:---|:---|
| `openai` | API key / OAuth | GPT-5.3 Codex, GPT-5.2 Codex, GPT-5.2 |
| `anthropic` | API key / OAuth | Claude Opus 4.6, Sonnet 4.6, Haiku 4.5 |
| `gemini` | API key / OAuth | Gemini 3.1 Pro Preview, 3 Flash, 3 Pro Preview, 2.5 Flash |
| `cursor` | Cursor local auth | Reads Cursor credentials from local state DB |
| `openrouter` | API key | OpenAI-compatible |
| `deepseek` | API key | OpenAI-compatible |
| `groq` | API key | OpenAI-compatible |
| `together` | API key | OpenAI-compatible |
| `ollama` | local/base URL | OpenAI-compatible local runtime |
| `kimi` / `kimi-coding` | API key | Moonshot/Kimi endpoints |
| `minimax` / `minimax-coding` | API key | MiniMax endpoints |
| `glm` / `glm-coding` | API key | Z.ai endpoints |
| `claude-sdk` / `codex` | experimental | Stub/agent integration paths |

See [docs/providers.md](docs/providers.md) for model tables and context windows.

<br>

## CLI Overview

```text
nyz                              # interactive TUI
nyz run "<prompt>"               # one-shot run
nyz run -i image.png "<prompt>"  # one-shot with image
nyz -p openai -m gpt-5.3-codex   # set provider/model
nyz -c                            # continue most recent session
nyz -s "query"                    # resume by title/ID
```

```text
nyz login <provider>             nyz logout <provider>
nyz whoami                       nyz config
nyz init                         nyz mcp add|list|remove
nyz sessions [query]             nyz session delete|rename
nyz export <id>                  nyz replay <id> [--filter]
nyz stats                        nyz cost [daily|weekly|monthly]
nyz deepinit                     nyz skills
nyz teams list|show|delete       nyz wait
nyz ci-fix [--format] [--commit] nyz update [--rollback]
nyz uninstall [--yes]
```

<br>

## Configuration (three-layer merge)

1. Global: `~/.config/nyzhi/config.toml`
2. Project: `.nyzhi/config.toml`
3. Local: `.nyzhi/config.local.toml`

```toml
[provider]
default = "openai"

[provider.openai]
model = "gpt-5.3-codex"

[tui]
theme = "dark"      # maps to nyzhi-dark
accent = "copper"

[agent]
max_steps = 100
auto_compact_threshold = 0.8

[agent.trust]
mode = "limited"    # off | limited | autoedit | full
allow_tools = ["edit", "write"]
allow_paths = ["src/"]

[[agent.hooks]]
event = "after_edit"
command = "cargo fmt -- {file}"
pattern = "*.rs"
```

Project rules are loaded from `AGENTS.md` / `.nyzhi/rules.md` / `.nyzhi/instructions.md` / `CLAUDE.md` / `.cursorrules`.

<br>

## Architecture

```text
                      nyzhi (cli)
                     /      |      \
             nyzhi-tui  nyzhi-core  nyzhi-provider
                   |      /      \       |
                nyzhi-auth      nyzhi-config
```

| Crate | Role |
|:---|:---|
| `crates/cli` | CLI entry, command dispatch, mode selection |
| `crates/core` | Agent runtime, tools, workspace/session/mcp/teams/autopilot |
| `crates/provider` | Provider trait + model registry + provider impls |
| `crates/tui` | ratatui app loop, selectors, command UX, streaming UI |
| `crates/auth` | API key + OAuth + token store + refresh/rotation |
| `crates/config` | Config schema/defaults + global/project/local merge |

<br>

## Data locations

| What | Path | Updated by installer/update? |
|:---|:---|:---|
| Binary | `~/.nyzhi/bin/nyz` | Yes |
| Config | `~/.config/nyzhi/` | No |
| Data (sessions/tokens/analytics) | `~/.local/share/nyzhi/` | No |
| Project config | `.nyzhi/` | No |
| Backups | `~/.nyzhi/backups/` | Managed/pruned |

<br>

## Documentation

Full docs: **[code.nyzhi.com/docs](https://code.nyzhi.com/docs)**

- [Architecture](docs/architecture.md)
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
- [Commands](docs/commands.md)
- [Skills](docs/skills.md)
- [Verification](docs/verification.md)
- [Routing](docs/routing.md)
- [Notifications](docs/notifications.md)
- [Self-Update](docs/self-update.md)
- [Memory](docs/memory.md)
- [Building](docs/building.md)
- [Releasing](docs/releasing.md)

---

<p align="center">
  <a href="CONTRIBUTING.md">Contributing</a> ·
  <a href="CODE_OF_CONDUCT.md">Code of Conduct</a> ·
  <a href="SUPPORT.md">Support</a> ·
  <a href="SECURITY.md">Security</a> ·
  <a href="https://github.com/nyzhi-com/code/issues">Issues</a>
</p>

<p align="center">
  <sub><a href="LICENSE">GPL-3.0-or-later</a></sub>
</p>
