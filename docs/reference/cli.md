# CLI Reference (Hierarchy)

See `docs/commands.md` for full semantics and examples. This page is a compact hierarchy reference.

```text
nyz
  run <prompt> [--image ...] [--format text|json] [--output file]
  exec [prompt] [--image ...] [--json] [--quiet] [--ephemeral]
       [--full_auto] [--sandbox level] [--output file]
  login [provider]
  logout <provider>
  whoami
  config
  init
  mcp add <name> [--url URL] [--scope global|project] [-- <command> ...]
  mcp list
  mcp remove <name> [--scope global|project]
  sessions [query]
  export <id-or-query> [-o file]
  session delete <id-or-query>
  session rename <id-or-query> <title>
  stats
  cost [daily|weekly|monthly]
  deepinit
  teams list
  teams show <name>
  teams delete <name>
  skills
  wait
  replay <id> [--filter event_type]
  update [--force] [--rollback path|latest] [--list-backups]
  uninstall [--yes]
  ci-fix [--log-file path] [--format auto|junit|tap|plain] [--commit]
```

Global flags:

```text
-p, --provider
-m, --model
-y, --trust
-c, --continue
-s, --session
    --team-name
    --teammate-mode
```
