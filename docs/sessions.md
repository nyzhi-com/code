# Sessions

Source of truth:

- `crates/core/src/session/mod.rs`
- `crates/cli/src/main.rs` (session commands)
- `crates/tui/src/export.rs`

## Session Data Model

Session metadata (`SessionMeta`):

- `id`
- `title`
- `created_at`
- `updated_at`
- `message_count`
- `provider`
- `model`

Serialized payload (`SessionFile`):

- `meta`
- `thread`

## Storage Path

Sessions are stored in:

- `<data_dir>/sessions/<id>.json`

`data_dir` comes from `Config::data_dir()` (typically under `~/.local/share/nyzhi/`).

## Persistence Behavior

- non-ephemeral runs persist sessions
- `nyz exec --ephemeral` disables session persistence
- title is derived from first user message (truncated to ~80 chars)

## CLI Session Commands

```bash
nyz sessions [query]
nyz session delete <id-or-title-fragment>
nyz session rename <id-or-title-fragment> "New title"
nyz export <id-or-title-fragment> [-o output.md]
nyz replay <id> [--filter <event-type>]
nyz --continue
nyz --session "<query>"
```

Lookup behavior:

- query matches id prefix or title substring
- if multiple matches are found where a single target is required, command fails with candidate list

## Export

`nyz export` loads session thread and emits markdown via `nyzhi_tui::export::export_thread_markdown`.

Default output path:

- generated timestamped path when `-o` is omitted

## Replay

`nyz replay` prints stored event timeline for session id, with optional event-type filtering.

## Session Lifecycle APIs

Core APIs:

- `save_session`
- `load_session`
- `list_sessions`
- `find_sessions`
- `latest_session`
- `delete_session`
- `rename_session`

## Operational Tips

- use `--ephemeral` for one-off CI automation where history is not needed
- use `nyz sessions <query>` to recover old work by title fragment
- use export for review artifacts and issue reports
