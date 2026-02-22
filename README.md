<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/hero.svg">
    <source media="(prefers-color-scheme: light)" srcset="docs/assets/hero.svg">
    <img src="docs/assets/hero.svg" alt="nyzhi" width="720" />
  </picture>
</p>

<p align="center">
  <a href="https://code.nyzhi.com/docs"><strong>Documentation</strong></a> &ensp;&middot;&ensp;
  <a href="#install"><strong>Install</strong></a> &ensp;&middot;&ensp;
  <a href="#quick-start"><strong>Quick Start</strong></a> &ensp;&middot;&ensp;
  <a href="https://github.com/nyzhi-com/code/releases"><strong>Releases</strong></a>
</p>

<br>

> **Single binary. No runtime deps. Ships 50+ tools.** Give it a task -- it reads your code, writes changes, runs your tests, and verifies the result. All inside a rich terminal UI or a single non-interactive command. Built in Rust, wired to every major LLM.

<br>

## Install

```bash
curl -fsSL https://get.nyzhi.com | sh
```

Or via a package manager:

```bash
cargo install nyzhi           # Cargo
npm install -g nyzhi            # npm
```

<details>
<summary><strong>Self-update, rollback, uninstall</strong></summary>

```bash
nyz update                   # check and apply
nyz update --rollback latest # rollback to previous version
nyz uninstall                # remove everything (binary, config, data, tokens)
```

Updates are checked automatically every 4 hours. Every update backs up the current binary, verifies the new one, and auto-rolls back on failure.

</details>

<details>
<summary><strong>Build from source</strong></summary>

```bash
git clone https://github.com/nyzhi-com/code && cd code
cargo build --release        # Rust 1.75+
# Binary is at target/release/nyz
```

</details>

---

## Quick Start

```bash
nyz                                # launch the TUI
```

First run walks you through connecting a provider. Or connect one directly:

```bash
nyz login openai                   # OAuth — opens your browser
nyz login anthropic                # Anthropic PKCE
nyz login gemini                   # Google PKCE
```

Then just work:

```bash
nyz                                # interactive TUI
nyz run "explain this codebase"    # one-shot, no TUI
nyz --continue                     # pick up where you left off
```

---

## Why nyzhi

<table>
<tr>
<td width="50%" valign="top">

### Every model, one interface

OpenAI, Anthropic, Gemini, OpenRouter, DeepSeek, Groq -- and any OpenAI-compatible endpoint. Switch mid-session. Route prompts to the right tier automatically. Track costs with built-in analytics.

### 50+ built-in tools

File ops, git, grep, glob, bash, sub-agents, LSP, AST search, browser automation, PR workflows, semantic search, debug instrumentation. All with a permission model you control.

### MCP native

Connect external tool servers via stdio or HTTP. Drop a `.mcp.json` in your project root -- same format as Claude Code and Codex.

</td>
<td width="50%" valign="top">

### Agent-first architecture

**Autopilot** -- five-phase autonomous execution. **Teams** -- spawn coordinated sub-agents with mailbox messaging and task boards. **Planning** -- planner/critic loops with persistent plans. **Skills** -- the agent learns your patterns.

### A TUI worth living in

8 themes. 14 accent colors. Syntax highlighting. Tab completion. Persistent history with reverse search. Multi-line input. In-session search. Conversation export. Desktop notifications.

### Your code stays yours

Every file change is tracked. `/undo` reverts anything. Trust modes gate what runs without approval. Updates are SHA256-verified with automatic rollback.

</td>
</tr>
</table>

---

## Providers

| Provider | Auth | Default Models |
|:---------|:-----|:---------------|
| **OpenAI** | API key / OAuth | GPT-5.3 Codex, GPT-5.2, o3, o4-mini |
| **Anthropic** | API key / OAuth | Claude Opus 4.6, Sonnet 4.6, Haiku 4.5 |
| **Gemini** | API key / OAuth | Gemini 3.1 Pro, 3 Flash, 2.5 Flash |
| **OpenRouter** | API key | Any model on the platform |
| **DeepSeek** | API key | DeepSeek Chat |
| **Groq** | API key | Groq-hosted models |
| **Custom** | API key | Any OpenAI-compatible endpoint |

```bash
nyz login openai       # OAuth device code
nyz login gemini       # Google PKCE
nyz login anthropic    # Anthropic PKCE
nyz whoami             # check all providers
```

Multi-account per provider. Automatic token refresh. Rate-limit rotation across accounts. [Full provider docs &rarr;](https://code.nyzhi.com/docs/providers)

---

## Features

<details open>
<summary><strong>Agent capabilities</strong></summary>

| Feature | How |
|:--------|:----|
| **Autopilot** | `/autopilot <idea>` -- 5 phases: expand, plan, execute, QA, validate |
| **Teams** | `/team 3 <task>` -- coordinated sub-agents with mailbox + task board |
| **Planning** | `plan: <task>` prefix -- planner/critic loop, persistent plans |
| **Routing** | Auto-select model tier (low/medium/high) by prompt complexity |
| **Verification** | Auto-detect build/test/lint for Rust, Node, Go, Python |
| **Skills** | `/learn` -- extract reusable patterns from sessions |
| **Memory** | Persistent notepad across sessions with topic-based recall |
| **Magic keywords** | `plan:`, `persist:`, `eco:`, `tdd:`, `review:`, `parallel:` |

</details>

<details>
<summary><strong>TUI</strong></summary>

| Feature | Details |
|:--------|:--------|
| **Themes** | Nyzhi Dark/Light, Tokyo Night, Catppuccin Mocha, Dracula, Solarized, Gruvbox |
| **Accents** | copper, blue, orange, emerald, violet, rose, amber, cyan, red, pink, teal, indigo, lime, monochrome |
| **Input** | Multi-line (Alt+Enter), `/editor` for $EDITOR, bracketed paste |
| **History** | Persistent across sessions, Ctrl+R reverse search |
| **Completion** | Tab for commands, `@file` mentions, file paths |
| **Search** | `/search` with Ctrl+N/P navigation |
| **Shortcuts** | Ctrl+T theme, Ctrl+A accent, Ctrl+L clear, PageUp/Down scroll |

</details>

<details>
<summary><strong>Workflow</strong></summary>

| Feature | Details |
|:--------|:--------|
| **Sessions** | Auto-save, resume (`--continue`), search, rename, delete, export |
| **Hooks** | Run formatters/linters/tests after edits or turns |
| **Commands** | Custom slash commands from `.nyzhi/commands/` or config |
| **Export** | `/export` to markdown |
| **Replay** | Event-level session replay |
| **Analytics** | Cost tracking per provider/model, daily/weekly/monthly reports |
| **Notifications** | Terminal bell, desktop, webhook, Telegram, Discord, Slack |
| **Deep init** | `nyz deepinit` generates AGENTS.md from project analysis |

</details>

<details>
<summary><strong>Tools (50+)</strong></summary>

**Files** &ensp; `read` `write` `edit` `multi_edit` `apply_patch` `glob` `grep` `list_dir` `directory_tree` `file_info` `delete_file` `move_file` `copy_file` `create_dir`

**Shell** &ensp; `bash` -- live streaming output

**Git** &ensp; `git_status` `git_diff` `git_log` `git_show` `git_branch` `git_commit` `git_checkout`

**Agent** &ensp; `task` `todo_write` `todo_read` `notepad_write` `notepad_read` `update_plan` `think` `load_skill` `tool_search`

**Analysis** &ensp; `verify` `lsp_diagnostics` `ast_search` `lsp_goto_definition` `lsp_find_references` `lsp_hover`

**Web** &ensp; `web_fetch` `web_search`

**Browser** &ensp; `browser_open` `browser_screenshot` `browser_evaluate`

**Teams** &ensp; `team_create` `team_delete` `send_message` `task_create` `task_update` `task_list` `team_list` `read_inbox`

**PR** &ensp; `create_pr` `review_pr`

**Search** &ensp; `semantic_search` `fuzzy_find`

**Debug** &ensp; `instrument` `remove_instrumentation` `tail_file` `batch_apply`

**Memory** &ensp; `memory_read` `memory_write`

[Full tool reference &rarr;](https://code.nyzhi.com/docs/tools)

</details>

---

## CLI

```
nyz                              interactive TUI
nyz run "<prompt>"               one-shot (non-interactive)
nyz run -i img.png "<prompt>"    with image
nyz -c / --continue              resume last session
nyz -s / --session "<query>"     resume by title or ID
nyz -p openai -m gpt-5.2        provider and model flags
```

<details>
<summary><strong>All commands</strong></summary>

```
nyz login <provider>             OAuth login
nyz logout <provider>            remove stored token
nyz whoami                       auth status
nyz config                       show config
nyz init                         create .nyzhi/
nyz mcp add|list|remove          MCP servers
nyz sessions [query]             list sessions
nyz export <id> [-o path]        export to markdown
nyz session delete|rename        manage sessions
nyz stats                        session statistics
nyz cost [daily|weekly|monthly]  cost report
nyz replay <id>                  replay session
nyz deepinit                     generate AGENTS.md
nyz skills                       list skills
nyz wait                         rate limit status
nyz teams list|show|delete       team management
nyz ci-fix                       auto-fix CI failures
nyz update [--rollback]          self-update
nyz uninstall                    uninstall
```

</details>

---

## Configuration

Three layers, merged in order: **global** `~/.config/nyzhi/config.toml` &rarr; **project** `.nyzhi/config.toml` &rarr; **local** `.nyzhi/config.local.toml`

```toml
[provider]
default = "anthropic"

[provider.anthropic]
model = "claude-sonnet-4-20250514"

[tui]
theme = "nyzhi-dark"
accent = "copper"

[agent]
max_steps = 100
auto_compact_threshold = 0.8

[agent.trust]
mode = "limited"
allow_tools = ["edit", "write"]
allow_paths = ["src/"]

[[agent.hooks]]
event = "after_edit"
command = "cargo fmt -- {file}"
pattern = "*.rs"
```

Project rules go in `AGENTS.md` or `.nyzhi/rules.md`. Custom commands go in `.nyzhi/commands/`. MCP servers go in `.mcp.json`.

[Full configuration reference &rarr;](https://code.nyzhi.com/docs/configuration)

---

## Architecture

Six crates, zero cycles, one binary.

```
                      nyzhi (cli)
                     ╱     │     ╲
              nyzhi-tui  nyzhi-core  nyzhi-provider
                    │    ╱         ╲      │
                nyzhi-auth      nyzhi-config
```

| Crate | What it does |
|:------|:-------------|
| **nyzhi** | Binary. CLI parsing, command dispatch, tool/MCP assembly. |
| **nyzhi-core** | Agent loop, 50+ tools, sessions, workspace, MCP, planning, teams, hooks, skills, verification, analytics. |
| **nyzhi-provider** | LLM abstraction. OpenAI, Anthropic, Gemini with streaming and thinking support. |
| **nyzhi-tui** | Terminal UI. ratatui, themes, highlighting, completion, history, export. |
| **nyzhi-auth** | OAuth2 PKCE + device code, API keys, token store, multi-account rotation. |
| **nyzhi-config** | Config loading and merging. Provider definitions. |

[Architecture deep-dive &rarr;](https://code.nyzhi.com/docs/architecture)

---

## Data locations

| What | Where | Touched by updates? |
|:-----|:------|:--------------------|
| Binary | `~/.nyzhi/bin/nyz` | Yes (backed up first) |
| Config | `~/.config/nyzhi/` | Never |
| Sessions | `~/.local/share/nyzhi/sessions/` | Never |
| Tokens | `~/.local/share/nyzhi/auth.json` | Never |
| Analytics | `~/.local/share/nyzhi/analytics.jsonl` | Never |
| Memory | `~/.local/share/nyzhi/MEMORY.md` | Never |
| Backups | `~/.nyzhi/backups/` | Pruned to 3 |

---

## Documentation

Full docs at **[code.nyzhi.com/docs](https://code.nyzhi.com/docs)** or in the [`docs/`](docs/) directory:

<table>
<tr><td>

**Core** &ensp; [Architecture](docs/architecture.md) &middot; [Configuration](docs/configuration.md) &middot; [Authentication](docs/authentication.md)

**Features** &ensp; [Providers](docs/providers.md) &middot; [Tools](docs/tools.md) &middot; [TUI](docs/tui.md) &middot; [MCP](docs/mcp.md) &middot; [Sessions](docs/sessions.md) &middot; [Teams](docs/teams.md) &middot; [Autopilot](docs/autopilot.md)

**Workflow** &ensp; [Hooks](docs/hooks.md) &middot; [Commands](docs/commands.md) &middot; [Skills](docs/skills.md) &middot; [Verification](docs/verification.md) &middot; [Routing](docs/routing.md) &middot; [Notifications](docs/notifications.md) &middot; [Memory](docs/memory.md)

**System** &ensp; [Self-Update](docs/self-update.md) &middot; [Building](docs/building.md) &middot; [Releasing](docs/releasing.md)

</td></tr>
</table>

---

<p align="center">
  <a href="https://github.com/nyzhi-com/code/blob/main/LICENSE">GPL-3.0-or-later</a>
</p>
