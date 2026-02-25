# Master Findings

## Coverage snapshot

- Runtime source files inventoried: 138 (`00-baseline/runtime-files.csv`)
- Feature rows in wiring matrix: 150 (`02-wiring-traces/wiring-matrix.csv`)
- Post-fix behavior verdicts: 150 pass (`03-behavior-checks/behavior-verdicts.csv`)

## Findings by severity

### Critical

1. **F001 — Placeholder success in `prompt`/`agent` hooks**
   - Category: `placeholder_logic`
   - Status: resolved
   - Fix: `prompt`/`agent` hooks now fail closed when unimplemented, with optional command fallback for executable behavior.
   - Files: `crates/core/src/hooks.rs`, `docs/hooks.md`
   - Regression coverage: new hook tests in `crates/core/src/hooks.rs`

### High

2. **F002 — Index config keys merged but partially unwired**
   - Category: `config_unconsumed`
   - Status: resolved
   - Fix:
     - Added `IndexOptions` and wired `embedding` + `exclude` into index construction/build.
     - Wired `auto_context` and `auto_context_chunks` into AgentConfig and runtime usage.
   - Files: `crates/index/src/lib.rs`, `crates/core/src/agent/mod.rs`, `crates/tui/src/app.rs`, `crates/cli/src/main.rs`, `docs/configuration.md`

3. **F003 — Arg-required slash commands fell through on bare invocation**
   - Category: `missing_dispatch`
   - Status: resolved
   - Fix: `/refactor`, `/team`, `/image`, `/search` now handle bare command path explicitly.
   - File: `crates/tui/src/input.rs`

### Medium

4. **F004 — Duplicate `/thinking` metadata entry**
   - Category: `docs_runtime_drift`
   - Status: resolved
   - Fix: replaced duplicate with `/thinking toggle`; added uniqueness and multi-word classification tests.
   - File: `crates/tui/src/completion.rs`

5. **F005 — Command selector omitted many built-ins/custom commands**
   - Category: `discoverability`
   - Status: resolved
   - Fix: selector now appends uncategorized built-in commands and custom commands.
   - File: `crates/tui/src/app.rs`

### Deferred

6. **F006 — `agent.auto_simplify` merged but unconsumed**
   - Category: `config_unconsumed`
   - Status: deferred (requires product-level behavior definition)
   - File: `crates/config/src/lib.rs`

7. **F007 — `git_undo` checkpoint hardening/dead-code concerns**
   - Category: `dead_code`
   - Status: deferred (isolate in dedicated git safety patch)
   - File: `crates/core/src/git_undo.rs`

## Verification evidence

- Required command evidence: `06-verification/verification-summary.md`
- Focused regression checks:
  - `cargo test -p nyzhi-core hooks::tests`
  - `cargo test -p nyzhi-core agent::tests`
  - `cargo test -p nyzhi-tui completion::tests`
