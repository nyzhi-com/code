# Verification

Nyzhi can automatically detect and run your project's build, test, and lint checks. The verification system provides structured evidence of pass/fail status, making it easy for the agent to confirm its changes are correct.

---

## Auto-Detection

When you run `/verify` or the agent calls the `verify` tool, Nyzhi scans the project root to determine the project type and applicable checks:

### Rust

| Check | Command |
|-------|---------|
| Build | `cargo check` |
| Test | `cargo test` |
| Lint | `cargo clippy --all -- -D warnings` |

### Node.js

| Check | Command |
|-------|---------|
| Build | `npm run build` (if script exists) |
| Test | `npm test` (if script exists) |
| Lint | `npx eslint .` (if eslint is configured) |

### Go

| Check | Command |
|-------|---------|
| Build | `go build ./...` |
| Test | `go test ./...` |
| Lint | `go vet ./...` |

### Python

| Check | Command |
|-------|---------|
| Test | `pytest` (if pytest is installed) |
| Lint | `ruff check .` (if ruff is installed) |

Detection is based on the presence of marker files (`Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, etc.).

---

## Evidence

Each check produces structured evidence:

```rust
struct Evidence {
    kind: CheckKind,       // Build, Test, Lint, Custom
    command: String,       // the exact command run
    exit_code: i32,        // process exit code
    stdout: String,        // standard output
    stderr: String,        // standard error
    timestamp: u64,        // when it ran (unix timestamp)
    elapsed_ms: u64,       // how long it took
}
```

- `passed()` -- true if `exit_code == 0`
- `is_fresh(max_age)` -- true if the evidence is recent enough

---

## Verify Report

Multiple checks produce a `VerifyReport`:

```
Verification Report:
  ✓ Build (cargo check) — 2.3s
  ✓ Test (cargo test) — 8.1s
  ✗ Lint (cargo clippy --all -- -D warnings) — 1.2s
    error: unused variable `x`

2/3 checks passed
```

- `all_passed()` -- true only if every check succeeded
- `summary()` -- human-readable report with timing and failure details

---

## Usage

### In the TUI

```
/verify          # show detected checks and run them
```

### As a Tool

The agent can call `verify` as a tool to check its work:

```
Tool: verify
Result:
  Build: ✓ (2.3s)
  Test: ✓ (8.1s)
  Lint: ✓ (1.2s)
  All checks passed.
```

### In Hooks

Run verification after each turn:

```toml
[[agent.hooks]]
event = "after_turn"
command = "cargo test"
timeout = 120
```

### In Persist Mode

`/persist` activates a verify/fix loop: the agent runs checks, identifies failures, fixes them, and re-runs checks until everything passes.

---

## Custom Checks

While auto-detection covers common project types, you can run arbitrary commands through hooks:

```toml
[[agent.hooks]]
event = "after_turn"
command = "make lint && make test"
timeout = 180
```

---

## Check Kinds

| Kind | Description |
|------|-------------|
| `Build` | Compilation check (e.g., `cargo check`, `go build`) |
| `Test` | Test suite (e.g., `cargo test`, `npm test`) |
| `Lint` | Linter/formatter (e.g., `clippy`, `eslint`, `ruff`) |
| `Custom` | User-defined check |
