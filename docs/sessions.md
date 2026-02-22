# Sessions

Nyzhi automatically saves conversations as sessions. You can resume, search, rename, delete, export, and replay them.

---

## Auto-Save

Every conversation is automatically saved to disk. Sessions are stored as JSON files in:

```
~/.local/share/nyzhi/sessions/<session-id>.json
```

Each session file contains:

- **Metadata**: ID, title, creation time, last update time, message count, provider, model.
- **Thread**: The full conversation (all user messages, agent responses, and tool calls).

The session title is derived from the first user message (truncated to 80 characters). Untitled sessions are labeled `"untitled"`.

---

## Resume

### From the CLI

```bash
# Resume the most recent session
nyz --continue
nyz -c

# Resume by ID or title search
nyz --session "refactor"
nyz -s "abc123"
```

### From the TUI

```
/sessions              # list all sessions
/sessions refactor     # filter by title
/resume <id>           # restore a session
```

When you resume a session, the full conversation thread is restored and you can continue where you left off.

---

## Search

Sessions can be searched by ID prefix or title (case-insensitive):

```bash
# CLI
nyz sessions "auth"

# TUI
/sessions auth
```

---

## Rename

```bash
# CLI
nyz session rename <id> "new title"

# TUI
/session rename "new title"    # renames the current session
```

---

## Delete

```bash
# CLI
nyz session delete <id>

# TUI
/session delete <id>
```

---

## Export

Export a session to a readable markdown file:

```bash
# CLI
nyz export <session-id>
nyz export <session-id> -o conversation.md

# TUI
/export
/export ~/Desktop/conversation.md
```

The exported markdown includes:

- Session metadata (title, date, provider, model)
- All user and assistant messages
- Tool call summaries with outputs
- Properly formatted code blocks

Default export path: `<session-title>.md` in the current directory.

---

## Replay

Replay a session's events in timeline order:

```bash
nyz replay <session-id>
nyz replay <session-id> --filter tool    # only tool events
```

This shows the sequence of events (text, thinking, tool calls, approvals, errors) as they originally occurred, useful for reviewing what the agent did and why.

---

## Statistics

View aggregate session statistics:

```bash
nyz stats
```

Shows total sessions, total messages, total time, and other metrics across all saved sessions.

---

## Storage Format

Sessions are stored as JSON with this structure:

```json
{
  "meta": {
    "id": "uuid-v4",
    "title": "first user message truncated",
    "created_at": "2025-01-15T10:30:00Z",
    "updated_at": "2025-01-15T10:45:00Z",
    "message_count": 12,
    "provider": "anthropic",
    "model": "claude-sonnet-4-20250514"
  },
  "thread": {
    "messages": [
      {
        "role": "user",
        "content": "..."
      },
      {
        "role": "assistant",
        "content": "..."
      }
    ]
  }
}
```

---

## Data Safety

Sessions are stored in `~/.local/share/nyzhi/` which is:

- **Never** touched by installs or updates
- **Never** modified by the agent (read-only from the agent's perspective)
- Safe to back up with standard file tools
