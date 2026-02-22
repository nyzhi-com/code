# Hooks

Hooks let you run commands automatically in response to agent actions. Use them to enforce formatting, run linters, execute tests, or perform any automated check after the agent modifies files.

---

## Configuration

Hooks are defined in `config.toml` as an array:

```toml
[[agent.hooks]]
event = "after_edit"
command = "cargo fmt -- {file}"
pattern = "*.rs"
timeout = 30

[[agent.hooks]]
event = "after_turn"
command = "cargo clippy --all -- -D warnings"
timeout = 60
```

---

## Hook Events

| Event | Trigger | Available Context |
|-------|---------|-------------------|
| `after_edit` | After any file-modifying tool (write, edit, multi_edit, apply_patch) | `{file}` -- the changed file path |
| `after_turn` | After each complete agent turn | None |
| `pre_tool` | Before a tool executes | `tool_name` filter |
| `post_tool` | After a tool succeeds | `tool_name` filter |
| `post_tool_failure` | After a tool fails | `tool_name` filter |
| `teammate_idle` | When a teammate reports no work | Team context |
| `task_completed` | When a team task is marked complete | Task context |

---

## Hook Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `event` | string | yes | When to trigger (see table above) |
| `command` | string | yes | Shell command to execute |
| `pattern` | string | no | File pattern filter (for `after_edit`) |
| `timeout` | integer | no | Timeout in seconds (default: 30) |
| `block` | boolean | no | If true, non-zero exit blocks the triggering action |
| `tool_name` | string | no | Filter by tool name (for pre/post_tool events) |

---

## Placeholders

The `{file}` placeholder in the command string is replaced with the path of the changed file. Only available for `after_edit` hooks.

```toml
[[agent.hooks]]
event = "after_edit"
command = "prettier --write {file}"
pattern = "*.ts,*.tsx"
```

---

## Pattern Matching

The `pattern` field filters which files trigger the hook. Patterns support:

- **Extension matching**: `*.rs`, `*.ts`, `*.py`
- **Path substring**: `src/api/` matches any file containing that substring
- **Multiple patterns**: Comma-separated, e.g., `*.ts,*.tsx,*.js`

If no pattern is specified, the hook runs for all file changes.

---

## Blocking Hooks

When `block = true`, a non-zero exit code from the hook prevents the triggering tool from completing. This is useful for pre-tool validation:

```toml
[[agent.hooks]]
event = "pre_tool"
tool_name = "git_commit"
command = "cargo test"
block = true
timeout = 120
```

This prevents commits if tests fail.

---

## Hook Results

Each hook execution produces a result with:

- `command` -- the executed command
- `stdout` -- standard output
- `stderr` -- standard error
- `exit_code` -- process exit code
- `timed_out` -- whether the hook exceeded its timeout

Results are reported back to the agent so it can react to failures.

---

## Special Exit Codes

For team hooks (`teammate_idle`, `task_completed`):

- **Exit code 0** -- success, continue normally
- **Exit code 2** -- rejection with feedback. The hook's stdout is returned as feedback to the agent, which may adjust its approach.

---

## Hook Types

Three hook execution types exist (currently only `command` is configurable):

| Type | Description |
|------|-------------|
| **Command** | Runs a shell command via `sh -c`. Supports timeout, stdin (JSON context). |
| **Prompt** | Evaluates a prompt with context (placeholder, returns YES). |
| **Agent** | Delegates to an agent for evaluation (placeholder, returns safe). |

Prompt and Agent types are reserved for future use.

---

## Examples

### Format Rust files after edit

```toml
[[agent.hooks]]
event = "after_edit"
command = "cargo fmt -- {file}"
pattern = "*.rs"
timeout = 30
```

### Run clippy after each turn

```toml
[[agent.hooks]]
event = "after_turn"
command = "cargo clippy --all -- -D warnings"
timeout = 60
```

### Lint TypeScript on save

```toml
[[agent.hooks]]
event = "after_edit"
command = "eslint --fix {file}"
pattern = "*.ts,*.tsx"
timeout = 15
```

### Run tests before commits

```toml
[[agent.hooks]]
event = "pre_tool"
tool_name = "git_commit"
command = "npm test"
block = true
timeout = 120
```

### Format Python files

```toml
[[agent.hooks]]
event = "after_edit"
command = "black {file}"
pattern = "*.py"
timeout = 15
```

---

## Viewing Hooks

In the TUI, use `/hooks` to see all configured hooks and their settings.
