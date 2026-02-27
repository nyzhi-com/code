# Hooks

Source of truth:

- `crates/config/src/lib.rs` (`HookConfig`, `HookEvent`, `HookType`)
- `crates/core/src/hooks.rs`

## Hook Configuration

Hooks are configured under `[[agent.hooks]]`.

Fields:

- `event` (required)
- `command` (optional for `prompt`/`agent` types when `prompt`/`instructions` are provided)
- `hook_type` (`command`, `prompt`, `agent`; default `command`)
- `prompt`
- `instructions`
- `tools`
- `model`
- `pattern`
- `tool_name`
- `block`
- `timeout` (seconds, default `30`)

## Events

Supported `HookEvent` values:

- `session_start`
- `user_prompt_submit`
- `pre_tool_use`
- `post_tool_use`
- `post_tool_use_failure`
- `permission_request`
- `notification`
- `after_edit`
- `after_turn`
- `subagent_start`
- `subagent_end`
- `compact_context`
- `worktree_create`
- `worktree_remove`
- `config_change`
- `teammate_idle`
- `task_completed`

## Hook Types

### `command`

- executes shell command
- receives optional JSON stdin context for event-driven hooks
- returns stdout/stderr/exit code

### `prompt`

- injects prompt text (`prompt` or `instructions`) into hook output channel
- optional `command` can append command output as context
- if both prompt/instructions are empty and command exists, falls back to command execution

### `agent`

- emits agent instructions (`instructions` or `prompt`)
- optional `command` output can be appended as context
- if no instructions/prompt and no command fallback, returns non-zero error result

## Filtering

Optional filters:

- `pattern`: file pattern matching (for edit/file-related hooks)
- `tool_name`: comma-separated tool names for tool-use events

`pattern` matching supports:

- `*.ext` style suffix checks
- substring checks

## Blocking Semantics

### Pre-tool blocking

`run_pre_tool_hooks` returns `(results, blocked)`.

A pre-tool hook can block execution when:

- event is `pre_tool_use`
- hook has `block = true`
- corresponding hook execution returns non-zero exit code

### Teammate/task feedback semantics

For:

- `teammate_idle`
- `task_completed`

exit code `2` is interpreted as feedback signal:

- teammate idle: keep teammate working with stderr feedback
- task completed: reject completion with stderr feedback

## Examples

### After-edit formatter

```toml
[[agent.hooks]]
event = "after_edit"
hook_type = "command"
command = "cargo fmt --all"
pattern = "*.rs"
timeout = 60
```

### Pre-tool policy gate

```toml
[[agent.hooks]]
event = "pre_tool_use"
hook_type = "command"
tool_name = "git_commit,git_checkout"
command = "scripts/policy-check.sh"
block = true
```

### Prompt-injection quality hint

```toml
[[agent.hooks]]
event = "after_turn"
hook_type = "prompt"
prompt = "Before finalizing, confirm tests and lint were run."
```

## Operational Guidance

- keep hooks deterministic and fast
- set explicit `timeout` for network-heavy hooks
- use `block` sparingly and only for hard policy gates
- prefer event-specific filtering (`tool_name`, `pattern`) to avoid noisy global hooks
