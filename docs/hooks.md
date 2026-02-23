# Hooks

Hooks execute automation around agent events (formatting, checks, policy gates, teammate feedback).

## Config schema

```toml
[[agent.hooks]]
event = "after_edit"
command = "cargo fmt -- {file}"
pattern = "*.rs"
timeout = 30
block = false
tool_name = "write,edit"
hook_type = "command" # command | prompt | agent
```

Fields come from `HookConfig`:

- `event`
- `command`
- `hook_type`
- `prompt`
- `instructions`
- `tools`
- `model`
- `pattern`
- `tool_name`
- `block`
- `timeout`

## Event names (exact)

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

## Built-in execution paths

Primary runtime helpers in `core/hooks.rs`:

- `run_after_edit_hooks(...)`
- `run_after_turn_hooks(...)`
- `run_pre_tool_hooks(...)`
- `run_post_tool_hooks(...)`
- `run_teammate_idle_hooks(...)`
- `run_task_completed_hooks(...)`

## Pattern behavior

`pattern` matching supports:

- extension style: `*.rs`
- comma lists: `*.ts,*.tsx`
- substring path checks: `src/`

## Placeholder behavior

`after_edit` hooks replace `{file}` in command with changed file path.

## Tool filters

`tool_name` supports comma-separated values and matches against `context.tool_name`.

## Blocking semantics

`block = true` is honored for pre-tool hooks:

- non-zero hook result can block tool execution

## Hook types

- `command`: real shell command (`sh -c`), timeout, optional JSON stdin context
- `prompt`: placeholder behavior currently returns static success-like result
- `agent`: placeholder behavior currently returns static JSON `{"safe": true, ...}`

`command` is the practical production path today.

## Team hook special code

For `teammate_idle` and `task_completed` handlers:

- exit code `2` is treated as rejection/feedback signal
- feedback is taken from hook `stderr`

## Result object

Every run returns:

- `command`
- `stdout`
- `stderr`
- `exit_code`
- `timed_out`
- `hook_type`

## Examples

### Format Rust files after edits

```toml
[[agent.hooks]]
event = "after_edit"
command = "cargo fmt -- {file}"
pattern = "*.rs"
timeout = 30
```

### Gate commits with tests

```toml
[[agent.hooks]]
event = "pre_tool_use"
tool_name = "git_commit"
command = "cargo test -q"
block = true
timeout = 120
```

### Run lint after each turn

```toml
[[agent.hooks]]
event = "after_turn"
command = "cargo clippy --all-targets --all-features -- -D warnings"
timeout = 120
```
