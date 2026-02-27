# Verification

Source of truth:

- `crates/core/src/verify.rs`
- `.github/workflows/ci.yml`
- `CONTRIBUTING.md`

## Verification Philosophy

Verification combines:

- build checks
- tests
- lint checks
- optional custom project checks

## Verify Runtime Model

Core types:

- `CheckKind`: `build|test|lint|custom`
- `VerifyCheck`: `{ kind, command }`
- `Evidence`: command result + stdout/stderr + timestamp + duration
- `VerifyReport`: collection of evidence with summary rendering

## Auto-detected Checks

`detect_checks(project_root)` defaults by project type:

### Rust

- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`

### Node

- `npm run build`
- `npm test`
- optional `npx eslint .` when local eslint binary exists

### Go

- `go build ./...`
- `go test ./...`
- `go vet ./...`

### Python

- `python -m pytest`
- `python -m ruff check .`

## CI Baseline in This Repository

From `.github/workflows/ci.yml`:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets` (with `RUSTFLAGS=-Dwarnings`)
- `cargo test --workspace`

## Recommended Local Commands (This Repo)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Optional fast type-check:

```bash
cargo check --workspace
```

## CLI and Agent-facing Verification

- `verify` tool runs checks and returns structured pass/fail evidence
- `nyz ci-fix` can use CI logs to propose and apply fixes

## Evidence Freshness

Evidence includes timestamps and can be freshness-checked via `Evidence::is_fresh(max_age)`.

## Failure Reporting

`VerifyReport::summary()` prints:

- pass/fail status per check
- command
- elapsed time
- tail of relevant stdout/stderr on failures

## Best Practices

- run checks before commit and before release
- keep custom checks deterministic and non-interactive
- for large changes, include both local command output and CI result links in PR description
