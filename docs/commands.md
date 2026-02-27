# Commands

Source of truth: `crates/cli/src/main.rs`.

The binary name is `nyz` (crate/package name is `nyzhi`).

## Global Options

These flags are parsed before subcommands:

| Flag | Type | Description |
| --- | --- | --- |
| `-p`, `--provider` | string | Provider id (`openai`, `anthropic`, `gemini`, etc.) |
| `-m`, `--model` | string | Model id for selected provider |
| `-y`, `--trust` | string | Trust mode override (`off`, `limited`, `autoedit`, `full`) |
| `-c`, `--continue` | bool | Resume most recent session (TUI mode) |
| `-s`, `--session` | string | Resume session by id prefix or title query (TUI mode) |
| `--team-name` | string | Run as team lead with this team context |
| `--teammate-mode` | string | Parsed values include `in-process` and `tmux`; currently parsed but not runtime-wired in CLI |

## Main Modes

| Command | Purpose |
| --- | --- |
| `nyz` | Launch interactive TUI |
| `nyz run "<prompt>"` | Non-interactive run; human-readable output by default |
| `nyz exec [prompt]` | CI/scripting mode; can read stdin and emit JSON events |

### `run` vs `exec`

- `run` always uses `SandboxLevel::FullAccess` in current implementation.
- `exec` defaults to `workspace-write` sandbox and supports explicit sandbox selection.
- `exec --full_auto` forces trust mode to `full` and sandbox to `workspace-write`.
- `exec` supports `--ephemeral` (skip session persistence); `run` does not expose this flag.

## Command Reference

### `nyz run`

```bash
nyz run "prompt text"
nyz run -i image.png "analyze this screenshot"
nyz run --format json "stream events as JSONL"
```

Options:

- `-i`, `--image <path>` (repeatable)
- `--format <text|json>` (default `text`)
- `-o`, `--output <file>` (write final response to file)

### `nyz exec`

```bash
nyz exec "fix lint errors"
cat ci.log | nyz exec --json "explain this failure"
nyz exec --sandbox read-only --ephemeral "audit this repository"
```

Options:

- `-i`, `--image <path>` (repeatable)
- `--json` (JSONL event stream)
- `-q`, `--quiet`
- `--ephemeral` (do not persist session file)
- `--full_auto` (auto-approve + workspace-write sandbox)
- `--sandbox <read-only|workspace-write|full-access>`
  - aliases accepted by parser: `readonly`, `workspace`, `full`, `danger-full-access`
- `-o`, `--output <file>`

Caveats:

- If prompt is omitted and stdin is not piped, command exits with error.
- If both stdin and prompt are provided, stdin content is prepended to prompt.

### Auth and Identity

Preferred interactive auth path:

```text
nyz
/connect
```

CLI auth commands:

```bash
nyz login [provider]
nyz logout <provider>
nyz whoami
```

- `login` without provider prompts with built-in provider list.
- OAuth is used where supported; API key fallback prompt is used otherwise.
- `/connect` is the default interactive path; `nyz login` is the CLI fallback.

### Configuration and Init

```bash
nyz config
nyz init
```

- `config` prints merged runtime config.
- `init` scaffolds `.nyzhi/` and local preference files.

### MCP

```bash
nyz mcp add local-fs -- npx @modelcontextprotocol/server-filesystem .
nyz mcp add remote --url https://example.com/mcp --scope global
nyz mcp list
nyz mcp remove local-fs --scope project
```

- `scope` defaults to `project`.
- project scope writes to `<project>/.nyzhi/config.toml`.
- global scope writes to `~/.config/nyzhi/config.toml`.
- list combines config-based servers and `.mcp.json` compatibility servers.

### Sessions

```bash
nyz sessions [query]
nyz session delete <id-or-title-fragment>
nyz session rename <id-or-title-fragment> "New title"
nyz export <id-or-title-fragment> [-o out.md]
nyz replay <id> [--filter tool]
```

Session lookup behavior:

- id prefix and title substring matching are both supported.
- ambiguous matches fail and print candidate list.

### Analytics

```bash
nyz stats
nyz cost [daily|weekly|monthly]
```

Period aliases accepted for `cost`:

- daily: `daily`, `day`
- weekly: `weekly`, `week`
- monthly: `monthly`, `month`

### Teams and Skills

```bash
nyz teams list
nyz teams show <name>
nyz teams delete <name>
nyz skills
```

### Deep Init / Wait / CI / Updates / Uninstall

```bash
nyz deepinit
nyz wait
nyz ci-fix [--log-file path] [--format auto|junit|tap|plain] [--commit]
nyz update [--force] [--rollback latest|<path>] [--list-backups]
nyz uninstall [--yes]
```

Notes:

- `ci-fix` reads from `--log-file` or stdin.
- `ci-fix --commit` stages all changes and creates a default commit message.
- `update` supports rollback and backup listing.
- `uninstall` removes binary, config, data, backups, and PATH entries.

## Defaults and Runtime Behavior

- Trust override: `--trust` parses with same semantics as config trust mode parser.
- Team context:
  - `--team-name` sets `team_name`, `agent_name=team-lead`, `is_team_lead=true` in tool context.
  - this affects tools that use team metadata and inbox/task behavior.
- Memory:
  - if `memory.auto_memory=true`, prompt injection includes recalled user/project memory.
- Auto-context:
  - `index.auto_context` and `index.auto_context_chunks` are forwarded to runtime config.

## Exit Behavior

Commands typically return:

- success (`0`) on completed operation
- non-zero on validation errors (invalid option values, missing required prompt/input, ambiguous session query)

For automation, prefer:

- `nyz exec --json` for machine-readable events
- explicit `--sandbox` and `--ephemeral` depending on safety and persistence needs
