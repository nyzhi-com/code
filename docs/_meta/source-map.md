# Source to Docs Coverage Map

This file is the source-of-truth index for documentation coverage. Each row maps code ownership to the docs page that explains it.

## Workspace and Build Surfaces

| Source file | Primary symbols/surfaces | Covered in |
| --- | --- | --- |
| `Cargo.toml` | workspace members, shared dependencies, version | `docs/architecture.md`, `docs/building.md`, `docs/releasing.md` |
| `rust-toolchain.toml` | pinned toolchain components | `docs/building.md`, `docs/verification.md` |
| `rustfmt.toml` | formatting policy | `docs/building.md`, `docs/verification.md` |
| `.github/workflows/ci.yml` | CI checks and branch policy | `docs/verification.md`, `docs/building.md`, `CONTRIBUTING.md` |
| `.github/workflows/release.yml` | release pipeline, crates/npm publishing | `docs/releasing.md` |

## CLI and User Entry Points

| Source file | Primary symbols/surfaces | Covered in |
| --- | --- | --- |
| `crates/cli/src/main.rs` | `Cli`, `Commands`, `run_once`, `handle_mcp_command` | `docs/commands.md`, `docs/mcp.md`, `docs/sessions.md` |
| `crates/tui/src/app.rs` | `App`, `AppMode`, event loop, background tasks | `docs/tui.md`, `docs/architecture.md` |
| `crates/tui/src/input.rs` | slash command dispatch, key handling | `docs/tui.md`, `docs/autopilot.md`, `docs/teams.md` |
| `crates/tui/src/completion.rs` | `SLASH_COMMANDS`, completion contexts | `docs/tui.md` |
| `crates/tui/src/export.rs` | session markdown export | `docs/sessions.md`, `docs/commands.md` |

## Config, Auth, Providers

| Source file | Primary symbols/surfaces | Covered in |
| --- | --- | --- |
| `crates/config/src/lib.rs` | `Config`, `Config::merge`, trust/sandbox/index/update settings | `docs/configuration.md`, `docs/routing.md`, `docs/notifications.md` |
| `crates/auth/src/lib.rs` | `resolve_credential(_async)`, `auth_status`, rate-limit rotation | `docs/authentication.md` |
| `crates/auth/src/token_store.rs` | token/API key storage and account rotation | `docs/authentication.md` |
| `crates/provider/src/lib.rs` | `Provider` trait, `create_provider_async`, model registry | `docs/providers.md`, `docs/architecture.md` |
| `crates/provider/src/types.rs` | model and chat transport types | `docs/providers.md`, `docs/architecture.md` |

## Agent Runtime, Context, Sessions

| Source file | Primary symbols/surfaces | Covered in |
| --- | --- | --- |
| `crates/core/src/agent/mod.rs` | `run_turn`, streaming loop, tool-call orchestration | `docs/architecture.md`, `docs/tools.md` |
| `crates/core/src/agent_manager.rs` | `spawn_agent`, `wait_any`, status lifecycle, limits | `docs/teams.md`, `docs/architecture.md` |
| `crates/core/src/agent_roles.rs` | built-in roles, role resolution, model overrides | `docs/teams.md`, `docs/tools.md` |
| `crates/core/src/agent_files.rs` | file-based role loading from `.nyzhi/.claude` | `docs/teams.md`, `docs/skills.md` |
| `crates/core/src/context_briefing.rs` | `SharedContext`, briefing caps and injection | `docs/architecture.md`, `docs/memory.md` |
| `crates/core/src/context/mod.rs` | compaction and context management | `docs/architecture.md` |
| `crates/core/src/memory.rs` | user/project memory layout and injection | `docs/memory.md`, `docs/architecture.md` |
| `crates/core/src/session/mod.rs` | session persistence, lookup, export metadata | `docs/sessions.md`, `docs/commands.md` |
| `crates/core/src/workspace/mod.rs` | workspace detection, rule priority, scaffolding | `docs/configuration.md`, `docs/skills.md`, `docs/architecture.md` |
| `crates/core/src/prompt/mod.rs` | system prompt construction, MCP summaries, deferred tool guidance | `docs/tools.md`, `docs/mcp.md`, `docs/architecture.md` |
| `crates/core/src/routing.rs` | prompt classification and model tier routing | `docs/routing.md`, `docs/configuration.md` |
| `crates/core/src/verify.rs` | verify checks and report model | `docs/verification.md` |
| `crates/core/src/updater.rs` | update checks, URL validation, backups, rollback | `docs/self-update.md` |
| `crates/core/src/autopilot.rs` | autopilot phases and state persistence | `docs/autopilot.md`, `docs/tui.md` |
| `crates/core/src/hooks.rs` | hook lifecycle and block/feedback behavior | `docs/hooks.md`, `docs/configuration.md` |
| `crates/core/src/replay.rs` | replay timeline loading and formatting | `docs/sessions.md` |

## MCP, Indexing, and Search

| Source file | Primary symbols/surfaces | Covered in |
| --- | --- | --- |
| `crates/core/src/mcp/mod.rs` | server connect/list/call, `.mcp.json` load | `docs/mcp.md`, `docs/configuration.md` |
| `crates/core/src/mcp/tool_adapter.rs` | MCP tool adapter naming and execution | `docs/mcp.md`, `docs/tools.md` |
| `crates/index/src/lib.rs` | index build/search/auto-context lifecycle | `docs/configuration.md`, `docs/tools.md`, `docs/architecture.md` |
| `crates/index/src/embedder.rs` | embedding mode selection | `docs/configuration.md`, `docs/providers.md` |

## Tool Registry and Tool Implementations

| Source file | Primary symbols/surfaces | Covered in |
| --- | --- | --- |
| `crates/core/src/tools/mod.rs` | `Tool`, `ToolRegistry`, default tool registration | `docs/tools.md`, `docs/architecture.md` |
| `crates/core/src/tools/spawn_agent.rs` | `spawn_agent` tool, role tool filtering | `docs/tools.md`, `docs/teams.md` |
| `crates/core/src/tools/send_input.rs` | `send_input` tool | `docs/tools.md`, `docs/teams.md` |
| `crates/core/src/tools/wait_tool.rs` | `wait` tool | `docs/tools.md`, `docs/teams.md` |
| `crates/core/src/tools/close_agent.rs` | `close_agent` tool | `docs/tools.md`, `docs/teams.md` |
| `crates/core/src/tools/resume_agent.rs` | `resume_agent` tool | `docs/tools.md`, `docs/teams.md` |
| `crates/core/src/tools/task.rs` | legacy `task` tool behavior | `docs/tools.md`, `docs/teams.md` |
| `crates/core/src/tools/team.rs` | team/task/inbox/spawn teammate tools | `docs/teams.md`, `docs/tools.md` |
| `crates/core/src/tools/{read,write,edit,apply_patch,glob,grep,fuzzy_find}.rs` | code/file interaction tools | `docs/tools.md` |
| `crates/core/src/tools/{git,bash,verify,browser,pr}.rs` | shell, git, verification, browser, PR tools | `docs/tools.md`, `docs/verification.md` |
| `crates/core/src/tools/{semantic_search,tool_search,lsp}.rs` | semantic, deferred, and language-aware discovery | `docs/tools.md`, `docs/tui.md` |
| `crates/core/src/tools/{todo,update_plan,ask_user,notepad,memory,load_skill}.rs` | planning, user prompts, memory, skills | `docs/tools.md`, `docs/skills.md`, `docs/memory.md` |

## Teams and Collaboration State

| Source file | Primary symbols/surfaces | Covered in |
| --- | --- | --- |
| `crates/core/src/teams/config.rs` | team/member config schema and overrides | `docs/teams.md`, `docs/configuration.md` |
| `crates/core/src/teams/tasks.rs` | shared team task board semantics | `docs/teams.md` |
| `crates/core/src/teams/mailbox.rs` | inbox message model and read/broadcast behavior | `docs/teams.md` |

## Known Documentation Risk Flags

- `Config::load_local` exists in `crates/config/src/lib.rs` but is not called by CLI/TUI merge paths today.
- `README.md` previously referenced many `docs/*` files that did not exist locally; this docs overhaul closes that gap.
- Hooks and index config behavior changed recently; these pages should be reviewed whenever `crates/core/src/hooks.rs` or `crates/index/src/lib.rs` changes.
