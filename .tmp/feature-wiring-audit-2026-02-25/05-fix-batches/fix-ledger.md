# Fix Ledger

## Completed batches

### Batch A (critical)

- F001 fixed: hook prompt/agent placeholder success semantics removed.
- Outcome:
  - fail-closed behavior for unimplemented hook types.
  - command fallback support for prompt/agent hook types.
  - regression tests added in `crates/core/src/hooks.rs`.

### Batch B (major/medium)

- F002 fixed: index/runtime config wiring
  - Added `IndexOptions` in `crates/index/src/lib.rs`.
  - Wired `embedding` and `exclude` to index startup/build.
  - Wired `auto_context` and `auto_context_chunks` into AgentConfig + turn execution.
- F003 fixed: no-arg slash command fallthroughs for `/refactor`, `/team`, `/image`, `/search`.
- F004 fixed: deduped `/thinking` completion entry, explicit `/thinking toggle` added.
- F005 fixed: command selector now includes uncategorized built-ins and custom commands.

## Deferred items

- F006 (`agent.auto_simplify`): deferred pending explicit product semantics.
- F007 (`git_undo` checkpoint hardening): deferred as separate focused safety patch.

## Validation status

- Focused tests for implemented fixes: passed.
- Required workspace checks executed and archived under `06-verification/`.
