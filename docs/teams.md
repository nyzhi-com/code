# Team Orchestration

Nyzhi can spawn multiple coordinated agents that work together on complex tasks. Teams have a lead agent (the coordinator) and one or more member agents, each with their own conversation context, tools, and optionally isolated git worktrees.

---

## Quick Start

```
/team 3 Build a REST API with auth, database, and tests
```

This spawns 3 sub-agents coordinated by the lead (your main agent). The lead breaks the task into sub-tasks, assigns them, and coordinates results.

---

## Architecture

### Lead Agent

The lead agent (your main TUI session) acts as the coordinator:

- Breaks the high-level task into sub-tasks
- Creates team members with specific roles
- Assigns tasks via the task board
- Monitors progress through the mailbox
- Resolves conflicts and merges results

### Member Agents

Each member runs its own agent loop with:

- Its own conversation thread and context
- Access to all tools (or a filtered subset based on role)
- Its own worktree (in tmux mode) or shared project directory (in-process mode)
- A mailbox for receiving instructions and sending status updates

---

## Modes

### In-Process (default)

```bash
nyz --teammate-mode in-process
```

All agents run as async tasks within the same process. They share the project directory and must coordinate file access through the mailbox.

### Tmux

```bash
nyz --teammate-mode tmux
```

Each agent gets its own tmux pane and git worktree. This provides true isolation -- agents can work on different branches simultaneously without conflicts. Requires `tmux` and `git` on PATH.

---

## Team Configuration

Teams are stored in `~/.nyzhi/teams/<team-name>/config.json`:

```json
{
  "name": "api-build",
  "members": [
    {
      "name": "auth-agent",
      "agent_id": "uuid",
      "agent_type": "worker",
      "color": "#3B82F6",
      "model": "claude-sonnet-4-20250514",
      "role": "Authentication module"
    }
  ],
  "created_at": "2025-01-15T10:00:00Z"
}
```

Each member is assigned a distinct color for visual identification in the TUI.

---

## Mailbox System

Agents communicate through a file-based mailbox system. Messages are JSON files stored in the team directory.

### Message Types

| Type | Description |
|------|-------------|
| `Message` | General message between agents |
| `Broadcast` | Message to all team members |
| `DirectMessage` | Point-to-point message |
| `Request` | Request with expected response |
| `Response` | Response to a request |
| `TaskAssignment` | New task assignment |
| `TaskCompleted` | Task completion notification |
| `IdleNotification` | Agent has no more work |
| `ShutdownRequest` | Request to shut down |
| `ShutdownResponse` | Acknowledgment of shutdown |
| `ConflictDetected` | File conflict between agents |
| `MergeRequest` | Request to merge changes |
| `PlanApprovalRequest` | Plan needs lead approval |
| `PlanApprovalResponse` | Lead's plan approval/rejection |

### Message Injection

At the start of each agent turn, unread messages are injected into the conversation context. This ensures agents stay aware of team activity without polling.

---

## Task Board

Teams have a shared task board for work coordination.

### Task Structure

```json
{
  "id": 1,
  "subject": "Implement JWT auth middleware",
  "description": "Create middleware that validates JWT tokens...",
  "active_form": "detailed description with context",
  "status": "in_progress",
  "owner": "auth-agent",
  "blocks": [],
  "blocked_by": [],
  "created_at": "2025-01-15T10:05:00Z",
  "updated_at": "2025-01-15T10:30:00Z"
}
```

### Task Statuses

| Status | Description |
|--------|-------------|
| `Pending` | Created but not started |
| `InProgress` | Actively being worked on |
| `Completed` | Finished successfully |
| `Blocked` | Waiting on another task |
| `Deleted` | Removed |

### Dependencies

Tasks can declare dependencies via `blocks` and `blocked_by` fields. When a task completes, `unblock_dependents()` automatically moves dependent tasks from `Blocked` to `Pending`.

### Concurrency

Task files use `fs2` file locking to prevent race conditions when multiple agents update tasks simultaneously.

---

## Tools

The following tools are available for team operations:

| Tool | Description |
|------|-------------|
| `team_create` | Create a new team with members |
| `team_delete` | Delete a team |
| `team_list` | List all teams |
| `send_team_message` | Send a message to a teammate |
| `read_inbox` | Read unread messages |
| `task_create` | Create a task on the board |
| `task_update` | Update task status, owner, etc. |
| `task_list` | List tasks with optional filter |

---

## CLI Management

```bash
nyz teams list              # list all teams
nyz teams show <name>       # show team details and status
nyz teams delete <name>     # delete a team and its data
```

---

## Conflict Detection

When agents modify the same file, the mailbox system can detect and report conflicts via `ConflictDetected` messages. The lead agent is responsible for resolving conflicts, typically by reviewing changes and choosing which to keep.

---

## Hooks

Two hook events are specific to teams:

- **`teammate_idle`** -- fires when a teammate reports it has no more work. The hook can return feedback (exit code 2) to assign new work.
- **`task_completed`** -- fires when a task is marked complete. The hook can reject the completion (exit code 2) with feedback.

See [hooks.md](hooks.md) for configuration.
