# Self-Update

Source of truth:

- `crates/core/src/updater.rs`
- `crates/cli/src/main.rs` (`update`, `uninstall`)
- `crates/config/src/lib.rs` (`UpdateConfig`)

## Commands

```bash
nyz update
nyz update --force
nyz update --list-backups
nyz update --rollback latest
nyz update --rollback /path/to/backup
nyz uninstall [--yes]
```

## Update Config

```toml
[update]
enabled = true
check_interval_hours = 4
release_url = "https://get.nyzhi.com"
```

Security note:

- `release_url` is global-config-only in merge logic (project config cannot override it).

## Update Flow

1. Check interval throttle via `update-check.json`.
2. Validate release URL (HTTPS required, blocked metadata hosts).
3. Fetch remote version metadata (`<release_url>/version`).
4. Compare semantic versions.
5. Download platform artifact URL.
6. Backup current binary.
7. Replace binary.
8. Verify new binary runs (`--version` check).
9. Run post-update integrity checks.

## Backups

Backups are stored under:

- `<data_dir>/backups/`

Behavior:

- auto-prunes to `MAX_BACKUPS = 3`
- rollback uses backup path and binary self-replace
- `list_backups()` returns newest-first

## Integrity and Safety Checks

Updater snapshots:

- config file hash
- data/config dir existence
- selected provider token presence

Post-flight validation checks for:

- missing config/data dirs
- config hash drift
- missing migrated tokens

## URL Validation Rules

- must be `https`
- blocks known cloud metadata hosts
- warns (but does not hard-fail) on non-default host for self-hosted setups

## Uninstall

`nyz uninstall` removes:

- binary
- config
- data
- backups
- shell PATH entries installed by nyzhi scripts

Use `--yes` to skip confirmation prompt.
