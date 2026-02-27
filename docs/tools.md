# Tools

Source of truth:

- `crates/core/src/tools/mod.rs` (registry)
- `crates/core/src/tools/*.rs` (tool implementations)
- `crates/tui/src/app.rs` (interactive-only tool registration)

## Tool Runtime Model

- Each tool implements the `Tool` trait:
  - `name()`
  - `description()`
  - `parameters_schema()`
  - `permission()` (defaults to read-only)
  - `execute()`
- Tools are registered in `ToolRegistry`.
- Definitions sent to the model are built from the registry.
- Deferred tools are hidden initially and discoverable via `tool_search`.

## Permission Model

Tool permission levels:

- `ReadOnly`
- `NeedsApproval`

Execution behavior in agent loop:

- read-only tool calls can run in parallel
- mutating/approval-required calls run sequentially
- trust and sandbox rules can still deny execution even if tool exists

## Availability by Runtime

| Surface | Notes |
| --- | --- |
| TUI (`nyz`) | Registers default tools + subagent lifecycle tools (`spawn_agent`, `send_input`, `wait`, `close_agent`, `resume_agent`, `spawn_teammate`) |
| CLI `run` / `exec` | Uses default registry; subagent lifecycle tools are not currently registered in this path |

## Tool Inventory

### Core code and file tools

| Tool | Permission | Purpose |
| --- | --- | --- |
| `bash` | approval | Run shell commands |
| `read` | read-only | Read file contents |
| `write` | approval | Write or overwrite files |
| `edit` | approval | Single-location string replace |
| `apply_patch` | approval | Apply unified diff atomically |
| `multi_edit` | approval | Transactional multi-file string replacements |
| `glob` | read-only | File search by glob pattern |
| `grep` | read-only | Regex search in files |
| `fuzzy_find` | read-only | Fuzzy filename search |
| `tail_file` | read-only | Read last N lines of file |
| `batch_apply` | approval | Apply operation across many files |

### Git tools

| Tool | Permission | Purpose |
| --- | --- | --- |
| `git_status` | read-only | Working tree status |
| `git_diff` | read-only | Staged/unstaged diff |
| `git_log` | read-only | Commit history |
| `git_show` | read-only | Single commit details |
| `git_branch` | read-only | Branch listing |
| `git_commit` | approval | Stage and commit |
| `git_checkout` | approval | Switch/create branch |

### Filesystem metadata and path operations

| Tool | Permission | Purpose |
| --- | --- | --- |
| `list_dir` | read-only | List directory entries |
| `directory_tree` | read-only | Recursive tree view |
| `file_info` | read-only | File metadata |
| `delete_file` | approval | Delete file/empty dir |
| `move_file` | approval | Move/rename path |
| `copy_file` | approval | Copy file |
| `create_dir` | approval | Create directory |

### Code intelligence tools

| Tool | Permission | Purpose |
| --- | --- | --- |
| `verify` | read-only | Run project verification checks |
| `lsp_diagnostics` | read-only | LSP diagnostics/capability discovery |
| `ast_search` | read-only | Structural pattern search |
| `lsp_goto_definition` | read-only | Symbol definition lookup |
| `lsp_find_references` | read-only | Symbol references lookup |
| `lsp_hover` | read-only | Type/docs at position |
| `semantic_search` | read-only | Embedding-based code retrieval (when index is enabled) |

### Planning, orchestration, and user interaction

| Tool | Permission | Purpose |
| --- | --- | --- |
| `todowrite` | read-only | Create/update structured todo list |
| `todoread` | read-only | Read current todo list |
| `create_plan` | read-only | Create/update session plan markdown |
| `think` | read-only | Side-effect-free reasoning note |
| `ask_user` | read-only | Ask user structured multiple-choice question |
| `tool_search` | read-only | Discover deferred/MCP tools |

### Memory and knowledge tools

| Tool | Permission | Purpose |
| --- | --- | --- |
| `memory_read` | read-only | Read user/project memory index or topic |
| `memory_write` | approval | Persist memory topics/index entries |
| `notepad_read` | read-only | Read plan notepad |
| `notepad_write` | read-only | Record learning/decision/issue |
| `load_skill` | read-only | Load skill content by name |

### Web, browser, and PR tools

| Tool | Permission | Purpose |
| --- | --- | --- |
| `web_fetch` | approval | Fetch URL text content |
| `web_search` | approval | Search web and return snippets |
| `browser_open` | approval | Open URL in headless browser |
| `browser_screenshot` | approval | Capture screenshot |
| `browser_evaluate` | approval | Evaluate JS in page context |
| `create_pr` | approval | Create PR (GitHub/GitLab CLI) |
| `review_pr` | read-only | Retrieve/review PR diff |

### Team and taskboard tools

| Tool | Permission | Purpose |
| --- | --- | --- |
| `team_create` | approval | Create team + config + inbox/taskboard |
| `team_delete` | approval | Delete team artifacts |
| `team_list` | read-only | List teams |
| `send_team_message` | read-only | Direct/broadcast team message |
| `read_inbox` | read-only | Read unread team inbox messages |
| `task_create` | read-only | Create shared team task |
| `task_update` | read-only | Update task status/owner |
| `task_list` | read-only | List team tasks |
| `spawn_teammate` | approval | Spawn agent and register as team member (interactive runtime registration) |

### Subagent lifecycle tools (interactive runtime)

| Tool | Permission | Purpose |
| --- | --- | --- |
| `spawn_agent` | read-only | Spawn sub-agent by role and message |
| `send_input` | read-only | Send follow-up message to sub-agent |
| `wait` | read-only | Wait for one/all agents to reach terminal status |
| `close_agent` | read-only | Close sub-agent and free slot |
| `resume_agent` | read-only | Resume completed/errored agent for new work |

## Legacy `task` Tool

`crates/core/src/tools/task.rs` implements a legacy `task` tool that runs a child agent synchronously. It is currently not part of the default CLI/TUI registration path and has largely been superseded by the explicit lifecycle tools above.

## Deferred and MCP Tools

When many MCP tools are present:

- tools can be registered as deferred
- the model uses `tool_search` to discover them
- deferred index may be written to `.nyzhi/context/tools/mcp-index.md`

See `docs/mcp.md` for MCP wiring details.

## ToolContext Fields

Important runtime context passed to each tool:

- `session_id`
- `cwd`
- `project_root`
- `depth` (0 for main agent, increments for subagents)
- `allowed_tool_names` (role filtering)
- `team_name`, `agent_name`, `is_team_lead`
- `sandbox_level`
- `todo_store`
- `index`
- `subagent_model_overrides`
- `shared_context`

These fields are critical for role-scoped behavior, team messaging, and subagent context briefing.
