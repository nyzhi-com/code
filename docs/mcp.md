# MCP (Model Context Protocol)

Source of truth:

- `crates/core/src/mcp/mod.rs`
- `crates/core/src/mcp/tool_adapter.rs`
- `crates/cli/src/main.rs` (`nyz mcp ...`)
- `crates/config/src/lib.rs` (`McpConfig`, `McpServerConfig`)

## What nyzhi Supports

`nyzhi` supports MCP servers over:

- stdio transport
- streamable HTTP transport

MCP tools are discovered at startup and registered into the tool registry.

## Config-based Server Definitions

In config:

```toml
[mcp.servers.localfs]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "."]

[mcp.servers.remote]
url = "https://example.com/mcp"
```

Schema forms:

- stdio:
  - `command`
  - `args` (optional)
  - `env` (optional)
- http:
  - `url`
  - `headers` (optional)

## `.mcp.json` Compatibility

`nyzhi` also reads `<project>/.mcp.json` using Claude/Codex-style schema.

Loader behavior (`load_mcp_json`):

- supports `mcpServers` alias
- maps `url` entries to HTTP servers
- maps `command` entries to stdio servers
- ignores invalid entries with warnings

## CLI Management Commands

```bash
nyz mcp add <name> --url <url> [--scope global|project]
nyz mcp add <name> [--scope global|project] -- <command> [args...]
nyz mcp list
nyz mcp remove <name> [--scope global|project]
```

Scope behavior:

- `project` (default): writes to `<project>/.nyzhi/config.toml`
- `global`: writes to `~/.config/nyzhi/config.toml`

## Runtime Connection Flow

At startup:

1. merge configured MCP servers from config
2. merge `.mcp.json` discovered servers
3. connect each server via `McpManager::start_all`
4. call `tools/list` on each connected server
5. register each MCP tool into local `ToolRegistry`

## Deferred MCP Tool Mode

When many MCP tools are present (`> 15`):

- tools are registered as deferred
- deferred index is written to `.nyzhi/context/tools/mcp-index.md`
- model can discover available deferred tools through `tool_search`

This avoids bloating initial tool definition payloads.

## MCP Tool Naming

MCP tools are exposed with adapter naming pattern:

- `mcp__<server_name>__<tool_name>`

Adapter wiring occurs in `tool_adapter::McpTool`.

## Calling MCP Tools

`McpManager::call_tool` executes:

- server lookup by name
- tool call by name + JSON argument map
- textual output extraction from MCP response content

## Server Introspection

Manager APIs include:

- `all_tools()`
- `tool_summaries()`
- `server_info_list()`
- `connect_server()` (hot-add)
- `stop_all()`

## Operational Notes

- failing servers are logged and skipped; startup continues
- MCP servers can materially increase tool surface area
- for large MCP fleets, use `tool_search` and narrower prompts
