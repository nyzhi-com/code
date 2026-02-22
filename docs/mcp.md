# MCP (Model Context Protocol)

Nyzhi supports the [Model Context Protocol](https://modelcontextprotocol.io/) for connecting external tool servers. MCP lets you extend Nyzhi with custom tools without modifying the codebase.

---

## Overview

MCP servers expose tools (and optionally resources) over a standardized protocol. Nyzhi acts as an MCP client, connecting to servers at startup and making their tools available to the agent alongside built-in tools.

Two transport types are supported:

- **stdio** -- Nyzhi spawns a child process and communicates over stdin/stdout.
- **HTTP** -- Nyzhi connects to a remote server via Streamable HTTP.

---

## Configuration

### In config.toml

```toml
# Stdio server
[mcp.servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
# env = { NODE_ENV = "production" }  # optional environment variables

# HTTP server
[mcp.servers.remote-api]
url = "https://mcp.example.com"
headers = { Authorization = "Bearer your-token" }
```

### In .mcp.json (project root)

Nyzhi reads `.mcp.json` at the project root, using the same format as Claude Code and Codex:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."],
      "env": {
        "NODE_ENV": "production"
      }
    },
    "remote": {
      "url": "https://mcp.example.com",
      "headers": {
        "Authorization": "Bearer token"
      }
    }
  }
}
```

Both sources are merged. If the same server name appears in both, the config.toml entry takes precedence.

---

## CLI Management

```bash
# Add a stdio server
nyz mcp add my-server -- npx -y @my/mcp-server /path

# Add an HTTP server
nyz mcp add my-remote --url https://mcp.example.com

# List configured servers
nyz mcp list

# Remove a server
nyz mcp remove my-server
```

`nyz mcp add` writes to `.mcp.json` in the project root. The `--` separator separates the server name from the command and arguments.

---

## Tool Naming Convention

MCP tools are registered in Nyzhi's tool registry with a namespaced name:

```
mcp__<server-name>__<tool-name>
```

For example, a tool named `read_file` on a server named `filesystem` becomes `mcp__filesystem__read_file`.

This avoids collisions with built-in tools and between different MCP servers.

---

## Connection Lifecycle

1. **Startup**: `McpManager::start_all()` connects to all configured servers in parallel.
2. **Handshake**: Each server receives a `list_tools` call to discover available tools.
3. **Registration**: Discovered tools are wrapped in `McpTool` adapters and added to the tool registry.
4. **Runtime**: When the agent calls an MCP tool, `McpManager::call_tool()` forwards the request to the appropriate server.
5. **Hot-connect**: New servers can be added at runtime via `connect_server()`.
6. **Shutdown**: `stop_all()` terminates all connections.

### Error Handling

If a server fails to connect at startup, Nyzhi logs a warning and continues without it. The remaining servers and all built-in tools remain available.

---

## Deferred Loading

When a large number of MCP tools are available (more than 15), Nyzhi uses a summary approach in the system prompt:

- **15 or fewer tools**: Full tool descriptions are included in the system prompt.
- **More than 15 tools**: A compact summary (server name, tool name, one-line description) is included, with a reference to `tool_search` for discovery.

This keeps the system prompt within budget while still making all tools discoverable.

---

## In the TUI

Use `/mcp` to see connected servers and their tools:

```
MCP Servers:
  filesystem (3 tools): read_file, write_file, list_directory
  remote-api (2 tools): query, submit
```

---

## Writing an MCP Server

Any MCP-compatible server works with Nyzhi. Popular options:

- **@modelcontextprotocol/server-filesystem** -- file system access
- **@modelcontextprotocol/server-github** -- GitHub API
- **@modelcontextprotocol/server-postgres** -- PostgreSQL queries

Custom servers can be built in any language. The server must implement the MCP protocol over stdio (stdin/stdout JSON-RPC) or HTTP (Streamable HTTP transport).

See the [MCP specification](https://modelcontextprotocol.io/docs) for protocol details.

---

## Permissions

MCP tools inherit the `NeedsApproval` permission by default. In `full` trust mode, they are auto-approved. In `limited` mode, they can be allowed via the `allow_tools` list:

```toml
[agent.trust]
mode = "limited"
allow_tools = ["mcp__filesystem__read_file"]
```
