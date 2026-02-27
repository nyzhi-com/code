# Teams and Subagents

Source of truth:

- `crates/core/src/agent_manager.rs`
- `crates/core/src/tools/spawn_agent.rs`
- `crates/core/src/tools/send_input.rs`
- `crates/core/src/tools/wait_tool.rs`
- `crates/core/src/tools/close_agent.rs`
- `crates/core/src/tools/resume_agent.rs`
- `crates/core/src/tools/team.rs`
- `crates/core/src/teams/config.rs`
- `crates/tui/src/input.rs`

## Concepts

- **subagent**: spawned worker managed by `AgentManager`
- **team**: named collection of agents with config, inboxes, and taskboard
- **team lead**: coordinator identity (usually `team-lead`)
- **teammate**: spawned agent registered as a team member

## AgentManager Lifecycle

Manager APIs:

- `spawn_agent`
- `send_input`
- `get_status`
- `subscribe_status`
- `wait_any`
- `shutdown_agent`
- `resume_agent`

Statuses:

- `pending_init`
- `running`
- `completed`
- `errored`
- `shutdown`
- `not_found`

Limits (from config):

- `agent.agents.max_threads`
- `agent.agents.max_depth`

On limit breach:

- spawn fails with explicit error

## Subagent Tools

Interactive runtime registers:

- `spawn_agent`
- `send_input`
- `wait`
- `close_agent`
- `resume_agent`
- `spawn_teammate`

`spawn_agent` features:

- role selection via `agent_type`
- role-based tool filtering (`allowed_tools` / `disallowed_tools`)
- optional runtime role->model override application
- context briefing injection from shared parent state
- optional notepad wisdom injection

## Role System

Roles come from three sources:

1. built-in roles (`agent_roles.rs`)
2. user-defined roles (`[agent.agents.roles]` in config)
3. file-based roles (`.nyzhi/agents/*.md`, fallback `.claude/agents/*.md`)

Built-in role ids include:

- `default`
- `explorer`
- `worker`
- `reviewer`
- `planner`
- `architect`
- `debugger`
- `security-reviewer`
- `quality-reviewer`
- `test-engineer`
- `build-fixer`
- `deep-executor`
- `document-specialist`
- `code-simplifier`

## Team Config Schema

`TeamConfig` fields:

- `name`
- `members: Vec<TeamMemberConfig>`
- `created_at`
- `default_model` (optional)
- `default_role` (optional)
- `max_steps` (optional)

`TeamMemberConfig` fields:

- `name`
- `agentId` (optional)
- `agentType`
- `color`
- `model` (optional)
- `role` (optional)
- `worktree_path` (optional)

## Team Storage

By default under `~/.nyzhi/`:

- `teams/<team>/config.json`
- `teams/<team>/inboxes/<member>.json`
- `tasks/<team>/*.json`
- `tasks/<team>/.highwatermark`

## Team Tools

| Tool | Purpose |
| --- | --- |
| `team_create` | Create team + inbox + taskboard files |
| `team_delete` | Delete team artifacts |
| `team_list` | List team names |
| `send_team_message` | Message teammate or broadcast |
| `read_inbox` | Read unread messages |
| `task_create` | Create team task |
| `task_update` | Update status/owner |
| `task_list` | List tasks |
| `spawn_teammate` | Spawn agent and register in team |

## CLI Team Commands

```bash
nyz teams list
nyz teams show <name>
nyz teams delete <name>
```

Global CLI team context:

- `--team-name` sets team metadata in run/exec tool context

## TUI Team Commands

Slash commands:

- `/team <N> <task>`
- `/teams-config`
- `/teams-config show <team>`
- `/teams-config set <team> model|role|max-steps <value>`
- `/teams-config member <team> <member> model|role <value>`
- `/teams-config reset <team>`
- `/subagent-config set <role> <model>`
- `/subagent-config reset [role]`

## Teammate Mode Flag

CLI parses `--teammate-mode` (`in-process`, `tmux`), but current runtime behavior is effectively in-process from the perspective of tool execution flow.

## Hook Integration

Team-specific hook events:

- `teammate_idle`
- `task_completed`

`hooks.rs` supports feedback semantics:

- hook exit code `2` can signal rejection/continue behavior in teammate/task flows

See `docs/hooks.md`.
