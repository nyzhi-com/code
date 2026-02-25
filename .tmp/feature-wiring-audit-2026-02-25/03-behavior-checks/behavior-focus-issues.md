# Behavior Focus Issues (Post-Fix Status)

## Resolved in this execution

1. Slash no-arg handling (`/refactor`, `/team`, `/image`, `/search`)
   - Status: resolved.
   - Change: bare commands now route through handler and emit usage guidance instead of falling through.
   - Files: `crates/tui/src/input.rs`

2. Duplicate `/thinking` command metadata
   - Status: resolved.
   - Change: replaced duplicate with explicit `/thinking toggle` entry and added uniqueness regression test.
   - Files: `crates/tui/src/completion.rs`

3. Hook `prompt`/`agent` success placeholders
   - Status: resolved.
   - Change: prompt/agent hooks now fail closed when no command fallback is provided; optional command fallback is executed with real exit code.
   - Files: `crates/core/src/hooks.rs`, `docs/hooks.md`

4. Index config wiring (`embedding`, `exclude`, `auto_context`, `auto_context_chunks`)
   - Status: resolved.
   - Change:
     - index constructor now accepts `IndexOptions` and uses `embedding` + `exclude`.
     - agent now uses configurable `auto_context_chunks`.
     - TUI/CLI AgentConfig now propagate `auto_context` + `auto_context_chunks`.
   - Files: `crates/index/src/lib.rs`, `crates/core/src/agent/mod.rs`, `crates/tui/src/app.rs`, `crates/cli/src/main.rs`, `docs/configuration.md`

5. Command selector coverage gaps
   - Status: resolved.
   - Change: selector now appends uncategorized built-ins and custom commands.
   - Files: `crates/tui/src/app.rs`

## Deferred (explicit)

1. `agent.auto_simplify` semantics
   - Status: deferred pending product behavior definition.
   - File: `crates/config/src/lib.rs`

2. `git_undo` checkpoint hardening
   - Status: deferred for isolated git safety follow-up patch.
   - File: `crates/core/src/git_undo.rs`
