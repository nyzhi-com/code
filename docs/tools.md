# Tools

Nyzhi registers a large built-in toolset in `nyzhi-core::tools::default_registry()`, then adds agent-management tools dynamically when an `AgentManager` is available.

## Permission model

- `ReadOnly`: default for tools unless overridden.
- `NeedsApproval`: explicit override per tool.

Trust mode then decides whether approval prompts are skipped.

## Built-in tool names

### File and filesystem

- `read`
- `write`
- `edit`
- `apply_patch`
- `multi_edit`
- `glob`
- `grep`
- `list_dir`
- `directory_tree`
- `file_info`
- `delete_file`
- `move_file`
- `copy_file`
- `create_dir`

### Shell and git

- `bash`
- `git_status`
- `git_diff`
- `git_log`
- `git_show`
- `git_branch`
- `git_commit`
- `git_checkout`

### Agent workflow and planning

- `task`
- `todowrite`
- `todoread`
- `notepad_write`
- `notepad_read`
- `update_plan`
- `think`
- `load_skill`
- `tool_search`
- `ask_user`

### Code intelligence

- `verify`
- `lsp_diagnostics`
- `ast_search`
- `lsp_goto_definition`
- `lsp_find_references`
- `lsp_hover`
- `semantic_search`
- `fuzzy_find`

### Web and browser

- `web_fetch`
- `web_search`
- `browser_open`
- `browser_screenshot`
- `browser_evaluate`

### PR and debugging

- `create_pr`
- `review_pr`
- `instrument`
- `remove_instrumentation`
- `tail_file`
- `batch_apply`

### Memory and team ops

- `memory_read`
- `memory_write`
- `team_create`
- `team_delete`
- `send_team_message`
- `task_create`
- `task_update`
- `task_list`
- `team_list`
- `read_inbox`

### Runtime-added agent control tools

When TUI initializes agent manager support, it also registers:

- `spawn_agent`
- `send_input`
- `wait`
- `close_agent`
- `resume_agent`
- `spawn_teammate`

## Tools with explicit `NeedsApproval`

The following built-ins explicitly request approval in code:

- `write`, `edit`, `apply_patch`, `multi_edit`
- `delete_file`, `move_file`, `copy_file`, `create_dir`
- `bash`
- `git_commit`, `git_checkout`
- `instrument`, `remove_instrumentation`
- `browser_open`, `browser_screenshot`, `browser_evaluate`
- `create_pr`
- `team_create`, `team_delete`, `spawn_teammate`
- `web_fetch`, `web_search`

Everything else is read-only unless changed in code.

## Trust interaction

- `off`: approval prompts for sensitive tools.
- `limited`: read-only tools generally auto-run; write/exec tools still gated unless allow rules match.
- `autoedit`: file-editing tools auto-approved (`write`, `edit`, `multi_edit`, `apply_patch`, and file mutation filesystem tools).
- `full`: all tools auto-approved except denied tools/paths.

## Deferred tool behavior

Tool registry supports deferred expansion:

- deferred tool schemas are omitted from initial LLM tool payload
- `tool_search` helps discover deferred tools
- once used, deferred tool is marked expanded and included in later turns

This is used to keep prompt size manageable with large toolsets.

## MCP tool naming

MCP tools are wrapped as:

- `mcp__<server_name>__<tool_name>`

Example: `mcp__filesystem__read_file`.

See [mcp.md](mcp.md) for merge, transport, and runtime details.

## Tool context fields

Every tool receives `ToolContext` with:

- `session_id`
- `cwd`
- `project_root`
- `depth`
- `event_tx`
- `change_tracker`
- `allowed_tool_names`
- `team_name`
- `agent_name`
- `is_team_lead`
- `todo_store`
