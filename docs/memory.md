# Memory

Source of truth:

- `crates/core/src/memory.rs`
- `crates/core/src/context_briefing.rs`
- `crates/core/src/tools/memory.rs`

## Memory Model

`nyzhi` supports two memory scopes:

- user memory (`~/.nyzhi/MEMORY.md`)
- project memory (`<data_dir>/projects/<hash>/memory/`)

Memory can be read, written, indexed, and injected into prompts.

## Paths

### User scope

- memory index: `~/.nyzhi/MEMORY.md`
- rules directory: `~/.nyzhi/rules/`

### Project scope

Project hash is derived from canonical project root path (`SHA256`, first 8 bytes hex).

Project memory base:

- `<data_dir>/nyzhi/projects/<project_hash>/memory/`

Within memory dir:

- `MEMORY.md` index
- `<topic>.md` topic files

## Prompt Injection

`load_memory_for_prompt(root)` returns:

- `## User Memory` section (up to half line budget)
- `## Project Memory` section (remaining budget)

Injection cap:

- `MAX_INJECTION_LINES = 200`

If memory is disabled (`memory.auto_memory=false`), memory injection is skipped.

## Memory APIs

Key functions:

- `memory_dir(root)`
- `user_memory_path()`
- `load_memory_for_prompt(root)`
- `memory_count(root)`
- `read_topic(root, topic)`
- `write_topic(root, topic, content, replace)`
- `read_index(root)`
- `list_topics(root)`
- `clear_memory(root)`

## Topic Write Modes

`write_topic(..., replace)` behavior:

- `replace = true`: overwrite topic file
- `replace = false`: append with newline separator

Index behavior:

- writing a topic updates `MEMORY.md` with markdown link entry if missing

## Memory Tools

### `memory_read`

- without `topic`: returns memory index (`MEMORY.md`)
- with `topic`: returns topic file content

### `memory_write`

- writes/appends topic content
- supports replace mode
- updates index

## Shared Context Briefing Integration

`SharedContext::build_briefing()` can embed project memory excerpt under `## Project Memory` for subagent context transfer.

Briefing caps:

- total lines: `MAX_BRIEFING_LINES = 60`
- recent changes: `MAX_CHANGE_ENTRIES = 20`
- message preview: `MAX_MESSAGE_PREVIEW = 5`

## Operational Guidance

- keep memory entries concise and stable
- use topic files for durable decisions and conventions
- clear project memory when context becomes stale (`/memory clear`)
