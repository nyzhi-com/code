# Fix Batch Plan

## Batch A (critical)

- F001: Hook prompt/agent placeholder success semantics.
- Target: make unsupported hook types fail closed unless explicit command fallback is configured.
- Required evidence: unit tests in `crates/core/src/hooks.rs`.

## Batch B (major/medium)

- F002: Wire index config fields (`embedding`, `exclude`, `auto_context`, `auto_context_chunks`).
- F003: Handle bare slash commands for arg-required commands.
- F004: Resolve duplicate `/thinking` metadata.
- F005: Expand command selector coverage.
- Required evidence: targeted tests in `crates/tui/src/completion.rs` and `crates/core/src/agent/mod.rs`.

## Batch C (deferred with rationale)

- F006: `agent.auto_simplify` needs product-level semantics before safe implementation.
- F007: `git_undo` checkpoint hardening should be isolated in dedicated git safety patch.
