# Self-Update

Nyzhi includes a built-in update system. It checks for new versions, downloads them with SHA256 verification, creates backups, and can roll back if something goes wrong.

---

## Update Commands

```bash
nyz update                   # check and apply if available
nyz update --force           # ignore throttle, check now
nyz update --list-backups    # list available rollback points
nyz update --rollback latest # rollback to previous version
nyz update --rollback <path> # rollback to a specific backup
```

---

## Automatic Checks

When the TUI starts, Nyzhi checks for updates in the background if:

- `[update] enabled = true` (default)
- At least `check_interval_hours` have passed since the last check (default: 4)
- The current version hasn't been explicitly skipped

If an update is available, a banner appears in the TUI:

- **`[u]`** -- Apply the update now
- **`[s]`** -- Skip for this session
- **`[i]`** -- Ignore this version permanently

---

## Update Flow

When an update is applied, the following steps execute in order:

### 1. Integrity Snapshot

Before changing anything, Nyzhi captures a snapshot of the current state:

- Hash of the config directory
- Keyring state
- Data directory presence

This snapshot is used for post-update verification.

### 2. Backup

The current binary is copied to `~/.nyzhi/backups/nyz-v<version>-<timestamp>`. The last 3 backups are kept; older ones are pruned.

### 3. Download

The new binary is downloaded from the release URL (default: `https://get.nyzhi.com`). The download URL is validated to prevent redirects to untrusted hosts.

### 4. SHA256 Verification

The downloaded file's SHA256 hash is compared against the expected hash from `latest.json`. If they don't match, the update is aborted.

### 5. Atomic Replacement

The binary is replaced using `self-replace`, which handles the OS-specific details of replacing a running executable:

- On Unix: rename + write
- On Windows: self-delete scheduling

### 6. Post-Flight Check

The new binary is executed with `--version` to verify it runs correctly. If it fails to produce output:

- The backup is restored automatically
- An error is reported

### 7. Integrity Verification

The integrity snapshot from step 1 is re-checked. If anything changed unexpectedly (config, keyring, data), the update is rolled back as a precaution.

---

## Rollback

If an update causes problems, roll back to a previous version:

```bash
# Rollback to the most recent backup
nyz update --rollback latest

# List available backups
nyz update --list-backups

# Rollback to a specific backup
nyz update --rollback ~/.nyzhi/backups/nyz-v0.2.0-1705312800
```

Backups are stored newest-first in `~/.nyzhi/backups/`.

---

## Version Skipping

If you don't want a specific version:

```
[i] Ignore this version
```

The skipped version is recorded and won't trigger the update banner again. Future versions will still be offered.

---

## Startup Health Check

On each launch, `startup_health_check()` runs:

- Verifies the integrity of the current binary
- Checks for signs of failed updates
- Reports any issues

---

## Configuration

```toml
[update]
enabled = true                 # enable update checks (default: true)
check_interval_hours = 4       # minimum hours between checks (default: 4)
release_url = "https://get.nyzhi.com"  # release endpoint
```

### Self-Hosted Releases

For air-gapped or enterprise environments, point to your own release server:

```toml
[update]
release_url = "https://releases.internal.example.com"
```

The server must serve a `latest.json` at `/releases/latest.json` and tarballs at `/releases/v<version>/nyzhi-<platform>-<arch>.tar.gz`.

---

## Security

- **Host validation**: Release URLs are checked against a trusted host list. Private IPs and localhost are blocked.
- **SHA256 required**: Downloads are never installed without hash verification.
- **Atomic replacement**: The old binary is never deleted until the new one is verified.
- **Automatic rollback**: If the new binary fails its post-flight check, the previous version is restored.
- **Integrity manifests**: Config and data directories are checked before and after updates.

---

## Data Safety

Updates only touch `~/.nyzhi/bin/nyz` (the binary). The following are never modified:

| Path | Content |
|------|---------|
| `~/.config/nyzhi/` | User configuration |
| `~/.local/share/nyzhi/` | Sessions, history, analytics, tokens |
| `.nyzhi/` | Project-level config and rules |
| OS keyring | OAuth tokens |
