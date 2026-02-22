# Memory

Nyzhi's memory system lets the agent persist knowledge across sessions. Learnings, decisions, architecture notes, and project conventions survive session boundaries and are injected into the system prompt when relevant.

---

## Two Scopes

### User Memory

Global memory that applies across all projects:

- Stored at `~/.local/share/nyzhi/MEMORY.md`
- Contains preferences, conventions, and general knowledge

### Project Memory

Per-project memory scoped to a specific codebase:

- Stored at `~/.local/share/nyzhi/projects/<hash>/memory/`
- The hash is derived from the canonical project root path
- Contains project-specific architecture decisions, patterns, and notes

---

## Topics

Memory is organized by topic. Each topic is a markdown file:

```
~/.local/share/nyzhi/projects/<hash>/memory/
  MEMORY.md          # index of all topics
  architecture.md    # architecture decisions
  testing.md         # testing conventions
  api-design.md      # API design patterns
```

### Writing Memory

The agent uses the `memory_write` tool to persist knowledge:

```
Tool: memory_write
Args: { "topic": "architecture", "content": "We use hexagonal architecture...", "replace": false }
```

- `topic` -- the topic name (creates the file if new)
- `content` -- the content to write
- `replace` -- if true, replaces the topic entirely. If false, appends.

When a new topic is written, the `MEMORY.md` index is updated automatically.

### Reading Memory

```
Tool: memory_read
Args: { "topic": "architecture" }
```

Returns the full content of the topic. Without a topic argument, returns the index.

---

## Prompt Injection

At the start of each session, Nyzhi loads memory and injects it into the system prompt:

1. Reads user memory (`MEMORY.md`)
2. Reads project memory (all topics for the current project)
3. Combines and truncates to `MAX_INJECTION_LINES` (200 lines)
4. Injects into the system prompt under a "Memory" section

This means the agent starts every session with awareness of past decisions and conventions.

---

## Notepad

The notepad is a session-scoped scratchpad distinct from persistent memory:

- `notepad_write` -- write an entry to the current session's notepad
- `notepad_read` -- read notepad entries

Notepad entries are useful for tracking decisions, open questions, and intermediate results within a single session. They don't persist across sessions (use memory for that).

### In the TUI

```
/notepad              # list all notepad entries
/notepad architecture # view a specific entry
```

---

## Memory Count

The `memory_count()` function reports how many items are stored:

- Counts list items (`-` prefixed lines) in the index
- Counts topic files in the memory directory

---

## Clearing Memory

To reset project memory:

```
Tool: memory_write
Args: { "topic": "all", "content": "", "replace": true }
```

Or delete the memory directory manually:

```bash
rm -rf ~/.local/share/nyzhi/projects/<hash>/memory/
```

---

## Best Practices

- **Use memory for conventions**: "We use Result<T, AppError> for all API handlers"
- **Use memory for architecture decisions**: "Auth uses JWT with refresh tokens stored in httpOnly cookies"
- **Use notepad for session-specific context**: "Current task: refactor the billing module"
- **Keep topics focused**: One topic per area (testing, auth, database, etc.)
- **Let the agent manage memory**: The agent writes to memory when it discovers important patterns. You don't need to manage it manually.
