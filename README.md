<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://nyzhi-hero.nyzhi.workers.dev/hero.svg" />
    <source media="(prefers-color-scheme: light)" srcset="https://nyzhi-hero.nyzhi.workers.dev/hero.svg" />
    <img src="https://nyzhi-hero.nyzhi.workers.dev/hero.svg" alt="nyzhi" width="720" />
  </picture>
</p>

<p align="center">
  <a href="https://code.nyzhi.com/docs">Docs</a> &ensp;&middot;&ensp;
  <a href="#install">Install</a> &ensp;&middot;&ensp;
  <a href="#quick-start">Quick Start</a> &ensp;&middot;&ensp;
  <a href="https://github.com/nyzhi-com/code/releases">Releases</a> &ensp;&middot;&ensp;
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

<br>

<p align="center">
Give it a task — it reads your code, writes changes, runs your tests, and verifies the result.<br>
All inside a rich terminal UI or a single non-interactive command.<br>
Built in Rust. Wired to every major LLM.
</p>

<br>

## Install

```bash
curl -fsSL https://get.nyzhi.com | sh
```

<table>
<tr>
<td>

**Cargo**
```bash
cargo install nyzhi
```

</td>
<td>

**npm**
```bash
npm install -g nyzhi
```

</td>
<td>

**Source**
```bash
git clone https://github.com/nyzhi-com/code
cd code && cargo build --release
```

</td>
</tr>
</table>

<details>
<summary>Self-update &amp; rollback</summary>

<br>

```bash
nyz update                   # check and apply
nyz update --rollback latest # revert to previous
nyz uninstall                # remove everything
```

Updates run automatically every 4h. Every update backs up the current binary, verifies the new one with SHA256, and auto-rolls back on failure.

</details>

<br>

## Quick Start

```bash
nyz                                # launch the TUI
nyz login openai                   # connect a provider (OAuth)
nyz run "explain this codebase"    # one-shot, no TUI
nyz --continue                     # pick up where you left off
```

<br>

## What makes nyzhi different

<table>
<tr>
<td width="50%" valign="top">

#### Every model, one binary

OpenAI, Anthropic, Gemini, OpenRouter, DeepSeek, Groq — and any OpenAI-compatible endpoint. Switch mid-session. Route prompts to the right cost tier automatically.

#### Autonomous execution

**Autopilot** — five-phase autonomous pipeline: expand, plan, execute, QA, validate. **Teams** — spawn coordinated sub-agents with mailbox messaging and task boards. **Planning** — planner/critic loops with persistent, replayable plans.

#### MCP native

Drop a `.mcp.json` in your project root. Same format as Claude Code and Codex. stdio and HTTP transports.

</td>
<td width="50%" valign="top">

#### 50+ tools, zero runtime deps

File ops, git, grep, glob, bash, sub-agents, LSP, AST search, browser automation, PR workflows, semantic search, debug instrumentation — all with a permission model you control.

#### A TUI worth living in

8 themes. 14 accent colors. Syntax highlighting. Tab completion. Persistent history with reverse search. Multi-line editing. Session export. Desktop notifications.

#### Your code stays yours

Every file change is tracked. `/undo` reverts anything. Trust modes gate what runs without approval. The agent learns your patterns via `/learn`.

</td>
</tr>
</table>

<br>

## Providers

| Provider | Auth | Models |
|:---|:---|:---|
| **OpenAI** | API key / OAuth | GPT-5.3 Codex, GPT-5.2, o3, o4-mini |
| **Anthropic** | API key / OAuth | Claude Opus 4.6, Sonnet 4.6, Haiku 4.5 |
| **Gemini** | API key / OAuth | Gemini 3.1 Pro, 3 Flash, 2.5 Flash |
| **OpenRouter** | API key | Any model on the platform |
| **DeepSeek** | API key | DeepSeek Chat |
| **Groq** | API key | Groq-hosted models |
| **Custom** | API key | Any OpenAI-compatible endpoint |

Multi-account per provider. Automatic token refresh. Rate-limit rotation across accounts.

<br>

## Agent Capabilities

| | |
|:---|:---|
| **Autopilot** | `/autopilot <idea>` — expand → plan → execute → QA → validate |
| **Teams** | `/team 3 <task>` — coordinated sub-agents with mailbox + task board |
| **Planning** | `plan:` prefix — planner/critic loop, persistent plans |
| **Routing** | Auto-select model tier by prompt complexity |
| **Verification** | Auto-detect build/test/lint for Rust, Node, Go, Python |
| **Skills** | `/learn` — extract reusable patterns from sessions |
| **Memory** | Persistent notepad across sessions with topic-based recall |
| **Hooks** | Run formatters/linters/tests after edits or agent turns |

<br>

## Tools

<table>
<tr>
<td valign="top">

**Files**<br>
<code>read</code> <code>write</code> <code>edit</code> <code>multi_edit</code> <code>apply_patch</code> <code>glob</code> <code>grep</code> <code>list_dir</code> <code>directory_tree</code> <code>file_info</code> <code>delete_file</code> <code>move_file</code> <code>copy_file</code> <code>create_dir</code>

**Shell**<br>
<code>bash</code> — live streaming output

**Git**<br>
<code>git_status</code> <code>git_diff</code> <code>git_log</code> <code>git_show</code> <code>git_branch</code> <code>git_commit</code> <code>git_checkout</code>

**Agent**<br>
<code>task</code> <code>todo_write</code> <code>todo_read</code> <code>notepad_write</code> <code>notepad_read</code> <code>update_plan</code> <code>think</code> <code>load_skill</code> <code>tool_search</code>

</td>
<td valign="top">

**Analysis**<br>
<code>verify</code> <code>lsp_diagnostics</code> <code>ast_search</code> <code>lsp_goto_definition</code> <code>lsp_find_references</code> <code>lsp_hover</code>

**Web &amp; Browser**<br>
<code>web_fetch</code> <code>web_search</code> <code>browser_open</code> <code>browser_screenshot</code> <code>browser_evaluate</code>

**Teams**<br>
<code>team_create</code> <code>team_delete</code> <code>send_message</code> <code>task_create</code> <code>task_update</code> <code>task_list</code> <code>team_list</code> <code>read_inbox</code>

**PR &amp; Search &amp; Debug**<br>
<code>create_pr</code> <code>review_pr</code> <code>semantic_search</code> <code>fuzzy_find</code> <code>instrument</code> <code>remove_instrumentation</code> <code>tail_file</code> <code>batch_apply</code>

</td>
</tr>
</table>

<br>

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
<summary>All commands</summary>

<br>

| Command | |
|:---|:---|
| `nyz login <provider>` | OAuth login |
| `nyz logout <provider>` | Remove stored token |
| `nyz whoami` | Auth status for all providers |
| `nyz config` | Show merged config |
| `nyz init` | Create `.nyzhi/` in current project |
| `nyz mcp add\|list\|remove` | Manage MCP servers |
| `nyz sessions [query]` | List / search sessions |
| `nyz export <id> [-o path]` | Export session to markdown |
| `nyz stats` | Session statistics |
| `nyz cost [daily\|weekly\|monthly]` | Cost report |
| `nyz replay <id>` | Replay session events |
| `nyz deepinit` | Generate AGENTS.md from project analysis |
| `nyz skills` | List learned skills |
| `nyz teams list\|show\|delete` | Team management |
| `nyz ci-fix` | Auto-fix CI failures |
| `nyz update [--rollback]` | Self-update or rollback |
| `nyz uninstall` | Uninstall nyzhi |

</details>

<br>

## Configuration

Three layers, merged in order: **global** `~/.config/nyzhi/config.toml` → **project** `.nyzhi/config.toml` → **local** `.nyzhi/config.local.toml`

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

| File | Purpose |
|:---|:---|
| `AGENTS.md` or `.nyzhi/rules.md` | Project rules the agent follows |
| `.nyzhi/commands/` | Custom slash commands |
| `.mcp.json` | MCP server definitions |

<br>

## Architecture

Six crates, zero cycles, one binary.

```
                      nyzhi (cli)
                     ╱     │     ╲
              nyzhi-tui  nyzhi-core  nyzhi-provider
                    │    ╱         ╲      │
                nyzhi-auth      nyzhi-config
```

| Crate | |
|:---|:---|
| **nyzhi** | Binary entry point. CLI parsing, command dispatch, tool and MCP assembly. |
| **nyzhi-core** | Agent loop, 50+ tools, sessions, workspace, MCP, planning, teams, hooks, skills, verification, analytics. |
| **nyzhi-provider** | LLM abstraction. OpenAI, Anthropic, Gemini with streaming and thinking support. |
| **nyzhi-tui** | Terminal UI. ratatui, themes, syntax highlighting, completion, history, export. |
| **nyzhi-auth** | OAuth2 PKCE + device code, API keys, token store, multi-account rotation. |
| **nyzhi-config** | Config loading and merging across global, project, and local layers. |

<br>

## Data

| What | Where | Touched by updates? |
|:---|:---|:---|
| Binary | `~/.nyzhi/bin/nyz` | Yes (backed up first) |
| Config | `~/.config/nyzhi/` | Never |
| Sessions | `~/.local/share/nyzhi/sessions/` | Never |
| Tokens | `~/.local/share/nyzhi/auth.json` | Never |
| Analytics | `~/.local/share/nyzhi/analytics.jsonl` | Never |
| Memory | `~/.local/share/nyzhi/MEMORY.md` | Never |
| Backups | `~/.nyzhi/backups/` | Pruned to 3 |

<br>

## Docs

Full documentation at **[code.nyzhi.com/docs](https://code.nyzhi.com/docs)** — or browse the [`docs/`](docs/) directory:

**Core** &ensp; [Architecture](docs/architecture.md) · [Configuration](docs/configuration.md) · [Authentication](docs/authentication.md)

**Features** &ensp; [Providers](docs/providers.md) · [Tools](docs/tools.md) · [TUI](docs/tui.md) · [MCP](docs/mcp.md) · [Sessions](docs/sessions.md) · [Teams](docs/teams.md) · [Autopilot](docs/autopilot.md)

**Workflow** &ensp; [Hooks](docs/hooks.md) · [Commands](docs/commands.md) · [Skills](docs/skills.md) · [Verification](docs/verification.md) · [Routing](docs/routing.md) · [Notifications](docs/notifications.md) · [Memory](docs/memory.md)

**System** &ensp; [Self-Update](docs/self-update.md) · [Building](docs/building.md) · [Releasing](docs/releasing.md)

<br>

---

<p align="center">
  <a href="CONTRIBUTING.md">Contributing</a> · <a href="CODE_OF_CONDUCT.md">Code of Conduct</a> · <a href="SUPPORT.md">Support</a> · <a href="SECURITY.md">Security</a> · <a href="https://github.com/nyzhi-com/code/issues">Issues</a>
</p>

<p align="center">
  <sub><a href="https://github.com/nyzhi-com/code/blob/main/LICENSE">GPL-3.0-or-later</a></sub>
</p>
