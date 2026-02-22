# Architecture

Nyzhi is a Rust workspace with six crates. Each crate has a single responsibility; dependencies flow downward from the binary to leaf crates with no cycles.

---

## Crate Dependency Graph

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
       └─────┬─────┘    └─────┬─────┘    └──────┬──────┘
             │                │                  │
             │         ┌──────┴──────┐           │
             │         │             │           │
             ▼         ▼             ▼           ▼
       ┌───────────┐  ┌─────────────┐     ┌───────────┐
       │ nyzhi-auth│  │nyzhi-provider│     │ nyzhi-auth│
       └─────┬─────┘  └──────┬──────┘     └─────┬─────┘
             │               │                   │
             ▼               ▼                   ▼
       ┌─────────────────────────────────────────────┐
       │              nyzhi-config                   │
       └─────────────────────────────────────────────┘
```

### Crate Summary

| Crate | Package | Role |
|-------|---------|------|
| `crates/cli` | `nyzhi` | Binary entry point (`nyz`). CLI parsing via clap, command dispatch, MCP server setup, tool registry assembly. |
| `crates/core` | `nyzhi-core` | Agent loop, 50+ tool implementations, session management, workspace detection, MCP client, planning, teams, hooks, skills, verification, analytics, and more. |
| `crates/provider` | `nyzhi-provider` | LLM provider abstraction. Implements the `Provider` trait for OpenAI, Anthropic, and Gemini with SSE streaming, thinking/reasoning support, and a model registry. |
| `crates/tui` | `nyzhi-tui` | Terminal UI. ratatui-based app loop with theming, syntax highlighting, tab completion, input history, session export, and component rendering. |
| `crates/auth` | `nyzhi-auth` | Authentication. OAuth2 PKCE and device-code flows, API key resolution, token storage in `auth.json`, multi-account support with rate-limit rotation. |
| `crates/config` | `nyzhi-config` | Configuration. TOML loading and merging across global, project, and local config files. Defines all config types and built-in provider definitions. |

---

## nyzhi-core Module Map

The core crate contains 39 modules covering all agent logic:

### Agent Loop

| Module | Purpose |
|--------|---------|
| `agent` | Main agent turn loop. Streams LLM responses, executes tool calls, handles approval, retries, auto-compaction, and team context injection. |
| `agent_files` | File context management for agent turns. |
| `agent_manager` | Manages multiple concurrent agent instances (sub-agents, teammates). |
| `agent_roles` | Role definitions for agent specialization (worker, explorer, planner, reviewer, etc.). |

### Conversation and Sessions

| Module | Purpose |
|--------|---------|
| `conversation` | `Thread` type for message sequences. |
| `session` | Session persistence (save, load, list, delete, rename, search) in JSON format. |
| `replay` | Event-level session replay. |
| `streaming` | Stream accumulation for SSE responses. |

### Workspace and Context

| Module | Purpose |
|--------|---------|
| `workspace` | Project root detection, project type classification (Rust/Node/Python/Go), rules loading (`AGENTS.md`, `.nyzhi/rules.md`, etc.), and `.nyzhi/` scaffolding. |
| `worktree` | Git worktree management for team isolation. |
| `context` | Token estimation and context window management. |
| `context_files` | `@file` mention extraction and resolution. |
| `prompt` | System prompt construction with environment, tools, rules, skills, and MCP summaries. |

### Tools

| Module | Purpose |
|--------|---------|
| `tools` | Tool trait, `ToolRegistry`, `ToolContext`, and `ToolResult`. Deferred tool loading and role-based filtering. |
| `tools/bash` | Shell command execution with live output streaming. |
| `tools/read`, `write`, `edit` | File read, write, and edit operations. |
| `tools/glob`, `grep` | File pattern matching and content search. |
| `tools/git` | Git status, diff, log, show, branch, commit, checkout. |
| `tools/filesystem` | list_dir, directory_tree, file_info, delete, move, copy, create_dir. |
| `tools/verify` | Build/test/lint execution with structured evidence. |
| `tools/lsp` | LSP diagnostics, goto definition, find references, hover, AST search. |
| `tools/web` | web_fetch and web_search. |
| `tools/browser` | Browser automation (open, screenshot, evaluate). |
| `tools/pr` | PR creation and review via `gh`. |
| `tools/task` | Sub-agent delegation. |
| `tools/todo` | Todo list management. |
| `tools/notepad` | Notepad read/write. |
| `tools/memory` | Persistent memory read/write. |
| `tools/team` | Team creation, messaging, task management. |
| `tools/semantic_search` | Semantic code search. |
| `tools/fuzzy_find` | Fuzzy file finder. |
| `tools/apply_patch`, `batch` | Structured patch application and batch operations. |
| `tools/instrument` | Debug instrumentation injection/removal. |
| `tools/think` | Explicit thinking/reasoning tool. |
| `tools/update_plan` | Plan update tool. |
| `tools/load_skill` | Lazy skill loading. |
| `tools/tool_search` | Deferred tool discovery. |
| `tools/tail_file` | File tail for log monitoring. |

### Features

| Module | Purpose |
|--------|---------|
| `mcp` | MCP server management (stdio/HTTP), tool adaptation, hot-connect. |
| `planning` | Planner/critic loop with persistent plans in `.nyzhi/plans/`. |
| `autopilot` | 5-phase autonomous execution (expansion, planning, execution, QA, validation). |
| `teams` | Team configuration, task board with file-locking, mailbox messaging system. |
| `commands` | Custom slash command loading from `.nyzhi/commands/` and config. |
| `hooks` | After-edit, after-turn, pre/post-tool hook execution with pattern matching. |
| `skills` | Skill persistence, templates, and lazy loading. |
| `verify` | Auto-detection of build/test/lint checks per project type. |
| `routing` | Prompt complexity classification and model tier selection. |
| `analytics` | Token usage and cost tracking in JSONL format. |
| `memory` | Project-scoped and user-scoped persistent memory. |
| `notify` | External notifications (webhook, Telegram, Discord, Slack). |
| `updater` | Self-update with SHA256 verification, backup, rollback, integrity manifests. |
| `plugins` | Plugin manifest and loader. |
| `deepinit` | AGENTS.md generation from project analysis. |
| `diagnostics` | System diagnostic info collection. |
| `sandbox` | Sandboxed execution environment. |
| `index` | Semantic indexing for code search. |
| `keywords` | Keyword extraction from prompts. |
| `judging` | Quality assessment. |
| `checkpoint` | Checkpoint management. |
| `persistence` | General persistence utilities. |

---

## Data Flow

### Interactive Mode (TUI)

```
User Input
    │
    ▼
nyzhi (cli) ── parse CLI args ── load config ── detect workspace
    │
    ▼
nyzhi-tui::App::run()
    │
    ├─ Build ToolRegistry (50+ tools + MCP tools)
    ├─ Create Provider (resolve credentials via nyzhi-auth)
    ├─ Start MCP servers (nyzhi-core::mcp)
    │
    ▼
Event Loop
    │
    ├─ User types message
    │   │
    │   ▼
    │   nyzhi-core::agent::run_turn()
    │       │
    │       ├─ Build system prompt (workspace, rules, tools, skills)
    │       ├─ Check context window, auto-compact if needed
    │       ├─ Provider::chat_stream() ── HTTP/SSE to LLM API
    │       │   │
    │       │   ├─ ThinkingDelta events ── displayed in TUI
    │       │   ├─ TextDelta events ── displayed in TUI
    │       │   └─ ToolCall events ── execute tools
    │       │       │
    │       │       ├─ ReadOnly tools: execute in parallel
    │       │       ├─ NeedsApproval tools: prompt user
    │       │       └─ Results fed back for next LLM turn
    │       │
    │       ├─ Retry on 429/5xx (exponential backoff)
    │       ├─ Run hooks (after_edit, after_turn)
    │       └─ Log analytics (token counts, cost)
    │
    ├─ Auto-save session
    └─ Render updated UI
```

### Non-Interactive Mode (`nyz run`)

Same flow but skips the TUI event loop. Output streams directly to stdout. Trust mode defaults apply for tool approval.

---

## Tool Registry Design

The tool registry uses a deferred loading pattern to keep initial prompt size small:

1. **Core tools** are registered normally and their full schemas are sent to the LLM in every request.
2. **Deferred tools** are registered but only indexed (name + description). They are not included in the ChatRequest tool definitions.
3. When the LLM needs a deferred tool, it calls `tool_search` to discover it by name/description.
4. On first use, the deferred tool is **expanded** -- its full schema is included in subsequent requests.

This keeps the prompt under budget while still making 50+ tools available.

### Permission Model

Each tool declares a `ToolPermission`:

| Level | Behavior |
|-------|----------|
| `ReadOnly` | Always auto-approved. Can run in parallel with other read-only tools. |
| `NeedsApproval` | Requires user confirmation (or auto-approved in `full` trust mode, or if matching `allow_tools`/`allow_paths` in `limited` mode). |

---

## Agent Turn Lifecycle

A single agent turn (`run_turn`) follows this sequence:

1. Push user message onto the conversation thread.
2. Build tool definitions (filtered by role if applicable).
3. For up to `max_steps` iterations:
   a. Inject unread teammate messages (if in a team).
   b. Micro-compact the thread if any individual message is oversized.
   c. If context usage exceeds `auto_compact_threshold` (default 85%), run full auto-compaction: summarize history, keep recent messages.
   d. Build `ChatRequest` with thinking config.
   e. Stream response via `Provider::chat_stream()`.
   f. On retryable error (429, 5xx): exponential backoff, try rate-limit account rotation.
   g. Execute tool calls: read-only in parallel, others sequentially.
   h. For `NeedsApproval` tools: emit `ApprovalRequest`, wait for user response.
   i. Offload large tool results to context files.
   j. Accumulate token usage, emit `Usage` event.
4. If the LLM stops calling tools (no tool_use in response), the turn is complete.
5. Run after-turn hooks.
6. Emit `TurnComplete`.
