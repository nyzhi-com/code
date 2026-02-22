# Tools

Nyzhi ships with 50+ built-in tools organized by category. Additional tools can be added via MCP servers (see [mcp.md](mcp.md)).

---

## Permission Model

Each tool has one of two permission levels:

| Permission | Behavior |
|------------|----------|
| **ReadOnly** | Always auto-approved. Can run in parallel with other read-only tools in the same turn. |
| **NeedsApproval** | Requires user confirmation before execution. Auto-approved in `full` trust mode, or selectively in `limited` mode. |

---

## File Operations

| Tool | Permission | Description |
|------|-----------|-------------|
| `read` | ReadOnly | Read file contents. Supports line offset and limit for large files. |
| `write` | NeedsApproval | Write content to a file. Creates the file if it doesn't exist. |
| `edit` | NeedsApproval | Replace a specific string in a file. The old string must be unique in the file. |
| `multi_edit` | NeedsApproval | Apply multiple edits to a file in a single operation. |
| `apply_patch` | NeedsApproval | Apply a unified diff patch to one or more files. |
| `glob` | ReadOnly | Find files matching a glob pattern. Recursive by default. |
| `grep` | ReadOnly | Search file contents using regex. Supports context lines, case-insensitive, multiline. |
| `list_dir` | ReadOnly | List directory contents with file types and sizes. |
| `directory_tree` | ReadOnly | Display a tree view of a directory structure. |
| `file_info` | ReadOnly | Get file metadata (size, permissions, modification time). |
| `delete_file` | NeedsApproval | Delete a file. |
| `move_file` | NeedsApproval | Move or rename a file. |
| `copy_file` | NeedsApproval | Copy a file to a new location. |
| `create_dir` | NeedsApproval | Create a directory (with parents). |

---

## Shell

| Tool | Permission | Description |
|------|-----------|-------------|
| `bash` | NeedsApproval | Execute a shell command. Output streams in real-time. Supports timeout and working directory. |

The bash tool runs commands via `sh -c` and captures stdout/stderr. Output is streamed back to the agent as `ToolOutputDelta` events, providing live visibility during long-running commands.

---

## Git

| Tool | Permission | Description |
|------|-----------|-------------|
| `git_status` | ReadOnly | Show working tree status. |
| `git_diff` | ReadOnly | Show changes (staged, unstaged, or between refs). |
| `git_log` | ReadOnly | Show commit history with optional count and format. |
| `git_show` | ReadOnly | Show a specific commit's contents. |
| `git_branch` | ReadOnly | List or show current branch. |
| `git_commit` | NeedsApproval | Create a commit with a message. Supports `--all` flag. |
| `git_checkout` | NeedsApproval | Switch branches or restore files. |

---

## Agent and Task Management

| Tool | Permission | Description |
|------|-----------|-------------|
| `task` | ReadOnly | Delegate a sub-task to a child agent. The child runs with its own context and tools. |
| `todo_write` | ReadOnly | Create or update todo items with id, content, and status. |
| `todo_read` | ReadOnly | Read the current todo list. |
| `notepad_write` | ReadOnly | Write an entry to the session notepad. |
| `notepad_read` | ReadOnly | Read notepad entries. |
| `update_plan` | ReadOnly | Update the current execution plan. |
| `think` | ReadOnly | Explicit thinking/reasoning step. The agent uses this to reason through complex problems without taking action. |
| `load_skill` | ReadOnly | Load a learned skill by name for the current context. |
| `tool_search` | ReadOnly | Search deferred tools by name or description. Returns matching tools and expands them for use. |

---

## Code Analysis

| Tool | Permission | Description |
|------|-----------|-------------|
| `verify` | ReadOnly | Run build, test, and lint checks. Auto-detects checks for Rust, Node, Go, and Python projects. Returns structured evidence with pass/fail, output, and timing. |
| `lsp_diagnostics` | ReadOnly | Get diagnostics from available language servers. |
| `ast_search` | ReadOnly | Structural code pattern matching using AST queries. |
| `lsp_goto_definition` | ReadOnly | Jump to the definition of a symbol. |
| `lsp_find_references` | ReadOnly | Find all references to a symbol. |
| `lsp_hover` | ReadOnly | Get hover information (type, docs) for a symbol at a position. |

---

## Search

| Tool | Permission | Description |
|------|-----------|-------------|
| `semantic_search` | ReadOnly | Search code by meaning, not exact text. Uses a semantic index to find relevant code chunks. |
| `fuzzy_find` | ReadOnly | Fuzzy file name search across the project. |

---

## Web

| Tool | Permission | Description |
|------|-----------|-------------|
| `web_fetch` | ReadOnly | Fetch a URL and return its content as readable text/markdown. |
| `web_search` | ReadOnly | Search the web and return summarized results with URLs. |

---

## Browser Automation

| Tool | Permission | Description |
|------|-----------|-------------|
| `browser_open` | NeedsApproval | Open a URL in a headless browser. |
| `browser_screenshot` | ReadOnly | Take a screenshot of the current browser page. |
| `browser_evaluate` | NeedsApproval | Execute JavaScript in the browser context. |

---

## Team Orchestration

| Tool | Permission | Description |
|------|-----------|-------------|
| `team_create` | NeedsApproval | Create a new agent team with configuration. |
| `team_delete` | NeedsApproval | Delete an existing team. |
| `team_list` | ReadOnly | List all teams and their members. |
| `send_message` | ReadOnly | Send a message to a teammate or broadcast to the team. |
| `read_inbox` | ReadOnly | Read unread messages from the team mailbox. |
| `task_create` | ReadOnly | Create a task on the team task board. |
| `task_update` | ReadOnly | Update a task's status, owner, or details. |
| `task_list` | ReadOnly | List tasks with optional status filter. |

---

## PR Workflow

| Tool | Permission | Description |
|------|-----------|-------------|
| `create_pr` | NeedsApproval | Create a GitHub pull request using `gh`. |
| `review_pr` | ReadOnly | Review an existing pull request. |

---

## Debug and Instrumentation

| Tool | Permission | Description |
|------|-----------|-------------|
| `instrument` | NeedsApproval | Inject debug instrumentation (logging, timing) into source files. Tracked for later removal. |
| `remove_instrumentation` | NeedsApproval | Remove all previously injected instrumentation. |
| `tail_file` | ReadOnly | Read the last N lines of a file, useful for monitoring logs. |
| `batch_apply` | NeedsApproval | Apply multiple file operations in a single batch. |

---

## Memory

| Tool | Permission | Description |
|------|-----------|-------------|
| `memory_read` | ReadOnly | Read persistent memory entries (project-scoped or user-scoped). |
| `memory_write` | ReadOnly | Write a memory entry. Persists across sessions. |

---

## Deferred Tool Loading

To keep the LLM's context budget manageable, some tools are registered as **deferred**. They appear in a compact index but their full JSON schemas are not sent to the LLM until they are first used.

The `tool_search` tool lets the agent discover deferred tools:

1. Agent calls `tool_search` with a query (e.g., "browser").
2. Returns matching deferred tool names and descriptions.
3. On first invocation of a deferred tool, it is **expanded** -- its full schema is included in all subsequent requests for the session.

This pattern allows Nyzhi to offer 50+ tools without overwhelming the context window.

---

## MCP Tools

Tools from MCP servers are registered with the naming convention `mcp__<server>__<tool>`. For example, a tool named `read_file` on a server named `filesystem` becomes `mcp__filesystem__read_file`.

MCP tools inherit the `NeedsApproval` permission by default. See [mcp.md](mcp.md) for setup.

---

## Tool Context

Every tool receives a `ToolContext` when executed:

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | String | Current session ID |
| `cwd` | PathBuf | Current working directory |
| `project_root` | PathBuf | Detected project root |
| `depth` | u32 | Agent depth (0 = main, 1 = sub-agent, etc.) |
| `change_tracker` | ChangeTracker | Tracks all file modifications for undo |
| `allowed_tool_names` | Option | Role-based tool filtering |
| `team_name` | Option | Team name if in a team |
| `agent_name` | Option | Agent name within team |
| `is_team_lead` | bool | Whether this agent is the team coordinator |
