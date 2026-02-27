# Roles and Context Briefing Internals

Source of truth:

- `crates/core/src/agent_roles.rs`
- `crates/core/src/agent_files.rs`
- `crates/core/src/context_briefing.rs`
- `crates/core/src/tools/spawn_agent.rs`

## Role Resolution Order

Role lookup (`resolve_role`) order:

1. user-defined roles (`[agent.agents.roles]`)
2. built-in roles (`built_in_roles()`)
3. fallback synthesized role with defaults

## Role Config Shape

`AgentRoleConfig` fields:

- `name`
- `description`
- `system_prompt_override`
- `model_override`
- `max_steps_override`
- `read_only`
- `allowed_tools`
- `disallowed_tools`
- `config_file`

`apply_role` mutates spawned `AgentConfig`:

- prompt override
- max steps override
- model override (`subagent_model`)
- agent name label (`sub-agent/<role>`)

`apply_model_override` then layers runtime override from `/subagent-config`.

## Built-in Roles

Built-ins include:

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

Each built-in role has dedicated prompt personality and optional tool constraints.

## File-based Roles

Loaded from:

- `.nyzhi/agents/*.md`
- `.claude/agents/*.md` (fallback)

Priority:

- `.nyzhi/agents` wins on name conflicts

Supported frontmatter keys:

- `name`
- `description`
- `model`
- `max_steps`
- `read_only`
- `allowed_tools`
- `disallowed_tools`

Role body becomes system prompt override.

## Shared Context Briefing

`SharedContext` carries parent execution context:

- `recent_changes`
- `active_todos`
- `conversation_summary`
- `project_root`

`build_briefing()` renders markdown sections and truncates with configured caps.

Injected briefing sections may include:

- recent file changes
- active todos
- conversation preview
- project memory excerpt

## Spawn-time Briefing Injection

`spawn_agent` tool attempts:

1. lock shared context
2. render briefing text
3. append briefing into child system prompt
4. append recent notepad wisdom when available

This gives subagents strong local context without re-reading full parent thread.
