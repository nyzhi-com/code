# Architecture

Nyzhi is a Rust workspace with six crates and a single CLI binary (`nyz`).

## Crate graph

```text
                      nyzhi (crates/cli)
                     /        |        \
             nyzhi-tui   nyzhi-core   nyzhi-provider
                   |         /   \            |
                nyzhi-auth          nyzhi-config
```

## Crate responsibilities

- `crates/cli` (`nyzhi`): clap parsing, command routing, provider/mcp bootstrap, non-interactive runs.
- `crates/core` (`nyzhi-core`): agent loop, tools, sessions, workspace detection, teams, hooks, MCP manager, updater, verification, routing, planning.
- `crates/provider` (`nyzhi-provider`): provider trait + concrete provider implementations + model registry.
- `crates/tui` (`nyzhi-tui`): event loop, rendering, selectors, completion, key handling, slash command UX.
- `crates/auth` (`nyzhi-auth`): API key + OAuth resolution, token store, refresh/rotation.
- `crates/config` (`nyzhi-config`): config schema/defaults/merge + built-in provider metadata.

## Workspace and project detection

`workspace::detect_workspace()`:

- walks upward until `.nyzhi/`, `.claude/`, or `.git` is found
- infers project type (`rust`, `node`, `python`, `go`, unknown)
- loads project rules from:
  - `AGENTS.md`
  - `.nyzhi/rules.md`
  - `.nyzhi/instructions.md`
  - `CLAUDE.md`
  - `.cursorrules`

## Runtime flow (interactive)

1. Parse CLI and load config.
2. Detect workspace root/rules.
3. Build tool registry (`default_registry`).
4. Create provider from selected provider id + auth resolution.
5. Merge MCP server config and connect available servers.
6. Enter TUI loop:
   - collect input / slash commands
   - dispatch `agent::run_turn(...)`
   - stream model events
   - execute tool calls
   - save session / update UI / notifications

## Agent turn loop essentials

`agent::run_turn_with_content(...)` does:

1. append user message to thread
2. build visible tool definitions (all, read-only for plan mode, or role-filtered)
3. iterate up to `max_steps`
4. call provider streaming API
5. parse text/thinking/tool events
6. execute tools with trust and approval checks
7. retry transient provider failures using retry settings
8. track usage/cost and emit events
9. finalize when model stops issuing tool calls

## Tool system

- `ToolRegistry` holds all tools.
- tools default to `ReadOnly` permission unless overridden to `NeedsApproval`.
- supports deferred tool expansion:
  - deferred tools omitted from initial tool schema payload
  - discoverable via `tool_search`
  - marked expanded after first use

## MCP integration

- MCP servers connect via stdio or streamable HTTP (`rmcp`).
- discovered tools are wrapped and registered under `mcp__<server>__<tool>`.
- if many MCP tools are present, they can be deferred and indexed to `.nyzhi/context/tools/mcp-index.md`.

## Update architecture

`core::updater` performs:

1. version check against release endpoint
2. checksum-gated download
3. backup + atomic self-replace
4. post-flight binary check (`--version`)
5. rollback on failure
6. integrity checks for user data paths/token presence

## Boundary notes

For docs and behavior verification:

- authoritative: maintained source code and config (`crates/*`, `Cargo.toml`, `.raccoon.toml`, docs)
- non-authoritative artifacts: `target/`, `node_modules/`, `.git/` internals

Generated outputs are important operationally (build products, dependency trees, git metadata) but are not the product-spec source.
