# Self-update

`core::updater` provides built-in version checking, download, checksum validation, backup, replace, and rollback.

## Commands

```bash
nyz update
nyz update --force
nyz update --list-backups
nyz update --rollback latest
nyz update --rollback <backup-path>
```

## Config

```toml
[update]
enabled = true
check_interval_hours = 4
release_url = "https://get.nyzhi.com"
```

`release_url` in effective runtime config is global-controlled in merge logic.

## Version check behavior

`check_for_update()`:

- respects `enabled`
- enforces throttle using `update-check.json` (`last_check_epoch`)
- validates release URL scheme/host policy
- fetches `GET <release_url>/version`
- compares semver against current build version
- respects skipped version state

`check_for_update_force()` clears throttle/skips and rechecks immediately.

## Expected release API contract

### Version endpoint

`GET <release_url>/version` returns JSON like:

```json
{
  "version": "1.1.3",
  "date": "2026-02-22T12:00:00Z",
  "changelog": "optional",
  "sha256": {
    "darwin-aarch64": "...",
    "darwin-x86_64": "...",
    "linux-x86_64": "...",
    "linux-aarch64": "..."
  }
}
```

### Download endpoint

Updater constructs:

- `GET <release_url>/download/<os>/<arch>?version=<version>`

where `<os>/<arch>` is derived from current platform.

## Apply flow

`download_and_apply()` pipeline:

1. snapshot integrity manifest
2. backup current executable
3. download archive and verify SHA-256
4. extract `nyz` from tarball to staging dir
5. atomically self-replace executable (`self_replace`)
6. run new binary with `--version` as post-flight check
7. rollback from backup if post-flight fails
8. re-run integrity checks and report warnings

## Backup/rollback details

- backups stored under `<data_dir>/backups`
- file naming: `nyz-v<version>-<timestamp>`
- keeps newest 3 backups
- `rollback(path)` uses `self_replace` to restore

## Integrity manifest checks

Manifest includes:

- config file hash
- existence of config/data directories
- keyring token presence snapshot for known providers

Warnings are produced if these invariants change after update.

## Startup health check

`startup_health_check()` reads the most recent manifest and surfaces recent integrity warnings (within 5 minutes).

## Security controls

- release URL must be HTTPS
- blocks known metadata/internal hosts (`169.254.169.254`, `metadata.google.internal`, `100.100.100.200`)
- allows non-default hosts with warning (self-host use case)
- checksum is mandatory; update is refused without expected SHA-256

## Data safety boundary

Update pipeline is intended to replace the binary and keep user/project state intact. Generated staging/build artifacts are temporary and not authority for runtime behavior.
