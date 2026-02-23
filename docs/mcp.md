# MCP (Model Context Protocol)

Nyzhi loads MCP servers from both config and `.mcp.json`, connects with `rmcp`, and exposes remote tools as first-class tools.

## Supported transports

- `Stdio` (spawn process and speak MCP over stdio)
- `Http` (streamable HTTP transport)

## Configuration sources

### `config.toml`

```toml
[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "."]
env = { NODE_ENV = "production" }

[mcp.servers.remote]
url = "https://mcp.example.com"
headers = { Authorization = "Bearer ..." }
```

### `.mcp.json` at project root

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    },
    "remote": {
      "url": "https://mcp.example.com"
    }
  }
}
```

`load_mcp_json()` accepts either `mcpServers` or `mcp_servers` (serde alias).

## Merge precedence

Runtime merge order is:

1. `config.mcp.servers`
2. `.mcp.json` servers via `extend(...)`

If names collide, `.mcp.json` wins.

## CLI commands

```bash
nyz mcp add <name> -- <command> [args...]
nyz mcp add <name> --url https://...
nyz mcp list
nyz mcp remove <name>
```

Scopes for add/remove:

- `--scope global` -> `~/.config/nyzhi/config.toml`
- `--scope project` (default) -> `.nyzhi/config.toml`

Important: CLI add/remove edits `config.toml`, not `.mcp.json`.

## Runtime lifecycle

1. Startup reads merged server config.
2. `McpManager::start_all()` attempts connection per server.
3. `list_tools` is called; discovered tools are registered.
4. Tool calls route via `McpManager::call_tool(server, tool, args)`.
5. Failed servers are warned and skipped (startup continues).

## Tool naming

MCP tools are wrapped as:

- `mcp__<server_name>__<tool_name>`

Example: `mcp__filesystem__read_file`.

## Deferred behavior for large MCP sets

When total MCP tools > 15:

- MCP tools are registered deferred.
- an index file is written to `.nyzhi/context/tools/mcp-index.md`.
- agent can discover them through `tool_search`.

## Header caveat

`McpServerConfig::Http` includes `headers`, but current connection code does not yet apply custom headers to transport requests.

## TUI view

`/mcp` displays connected server names and tool counts/tool names (`server_info_list()` output).
