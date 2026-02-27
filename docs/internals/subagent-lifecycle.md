# Subagent Lifecycle Internals

Source of truth:

- `crates/core/src/agent_manager.rs`
- `crates/core/src/tools/spawn_agent.rs`
- `crates/core/src/tools/send_input.rs`
- `crates/core/src/tools/wait_tool.rs`
- `crates/core/src/tools/close_agent.rs`
- `crates/core/src/tools/resume_agent.rs`
- `crates/tui/src/app.rs`

## Spawn Path

1. `SpawnAgentTool::execute` validates input.
2. Resolve role:
   - user role map
   - built-in role fallback
3. Build `AgentConfig`:
   - role prompt override
   - max steps override
   - optional role model override
4. Apply runtime model override (session-scoped `/subagent-config`).
5. Inject context briefing from shared state.
6. Inject latest notepad wisdom when available.
7. Compute tool filter (`allowed_tools`/`disallowed_tools`).
8. Call `AgentManager::spawn_agent`.

## Manager Responsibilities

- generate unique agent id (`uuid`)
- assign nickname from pool (Pokemon name list)
- enforce `max_threads` and `max_depth`
- create per-agent thread + status channels
- start task and forward events
- track active handles and cleanup on completion

## Status and Events

Statuses:

- `PendingInit`
- `Running`
- `Completed(Option<String>)`
- `Errored(String)`
- `Shutdown`
- `NotFound`

Parent-facing events:

- `SubAgentSpawned`
- `SubAgentStatusChanged`
- `SubAgentCompleted`

## Interaction Tools

### `send_input`

- sends follow-up instructions to active agent
- used for iterative refinement

### `wait`

- waits for one or many agents until terminal states or timeout
- intended to reduce busy polling loops

### `close_agent`

- shutdown/cancel and release slot

### `resume_agent`

- reopen completed/errored agent for further interaction

## Tool Visibility and Role Fencing

Role fencing occurs at spawn:

- explicit whitelist from `allowed_tools`
- optional blacklist subtraction from `disallowed_tools`

At execute-time, `ToolRegistry::execute` enforces `allowed_tool_names` gate.

## Context Propagation

`ToolContext` propagation includes:

- `depth + 1`
- team identity fields
- todo store handle
- index handle
- sandbox level
- runtime model override map
- shared context handle

## Runtime Registration Detail

Subagent lifecycle tools are registered in TUI startup path (`App::run`) after `AgentManager` creation. They are not part of default registry in non-interactive CLI `run`/`exec`.
