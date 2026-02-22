# Releasing

Releases are automated via GitHub Actions. Push a version tag to trigger the full pipeline: build, checksum, publish to GitHub Releases, and upload to Cloudflare R2.

---

## Release Process

### 1. Tag the Release

```bash
git tag v0.2.1
git push origin v0.2.1
```

The `v*` tag pattern triggers the release workflow.

### 2. Build (Automated)

The workflow builds for four targets in parallel:

| Target | OS | Architecture | Runner |
|--------|----|-------------|--------|
| `x86_64-unknown-linux-gnu` | Linux | x86_64 | ubuntu-latest |
| `aarch64-unknown-linux-gnu` | Linux | ARM64 | ubuntu-latest (cross) |
| `x86_64-apple-darwin` | macOS | x86_64 | macos-latest |
| `aarch64-apple-darwin` | macOS | ARM64 | macos-latest |

Linux ARM64 uses [cross](https://github.com/cross-rs/cross) for cross-compilation.

### 3. Package

Each build produces:

- `nyzhi-<platform>-<arch>.tar.gz` -- compressed tarball containing the `nyz` binary
- `nyzhi-<platform>-<arch>.sha256` -- SHA256 checksum file

### 4. Publish

The publish job:

1. Downloads all build artifacts
2. Validates all checksums (must be 64-character hex strings)
3. Creates a GitHub Release with auto-generated release notes
4. Uploads tarballs and checksums to the release

### 5. Upload to R2

Tarballs are uploaded to Cloudflare R2 for the install script:

```
nyzhi-releases/releases/v<version>/nyzhi-<platform>-<arch>.tar.gz
```

### 6. Update latest.json

A `latest.json` manifest is uploaded to R2:

```json
{
  "version": "0.2.1",
  "date": "2025-02-22T12:00:00Z",
  "sha256": {
    "darwin-aarch64": "<hash>",
    "darwin-x86_64": "<hash>",
    "linux-x86_64": "<hash>",
    "linux-aarch64": "<hash>"
  }
}
```

This is what `nyz update` and the install script check to determine the latest version and verify downloads.

### 7. Upload Install Script

The install script (`infra/releases-worker/install.sh`) is uploaded to R2 so that `curl -fsSL https://get.nyzhi.com | sh` always gets the latest version.

---

## Required Secrets

Configure these in your GitHub repository settings:

| Secret | Purpose |
|--------|---------|
| `CLOUDFLARE_API_TOKEN` | Cloudflare API token with R2 write access |
| `CLOUDFLARE_ACCOUNT_ID` | Your Cloudflare account ID |

---

## Workflow File

The release workflow is at `.github/workflows/release.yml`. Key characteristics:

- **Pinned actions**: All GitHub Actions are pinned to specific commit SHAs for supply-chain security.
- **Checksum validation**: Checksums are validated after download and before upload to R2.
- **Stable toolchain**: Uses `dtolnay/rust-toolchain` with the stable channel.

---

## Version Bumping

The version is defined in the workspace `Cargo.toml`:

```toml
[workspace.package]
version = "0.2.1"
```

To release a new version:

1. Update the version in `Cargo.toml`
2. Run `cargo check` to update `Cargo.lock`
3. Commit: `git commit -am "release: v0.2.2"`
4. Tag: `git tag v0.2.2`
5. Push: `git push origin main v0.2.2`

---

## Install Script

The install script at `infra/releases-worker/install.sh` handles:

- Platform detection (Linux/macOS, x86_64/ARM64)
- Version checking (skips if already up to date)
- Download with progress bar
- SHA256 verification
- Backup of existing binary
- Atomic installation
- Post-install verification (runs `nyz --version`, rolls back on failure)
- PATH setup (zsh, bash, fish)
- Uninstall support (`--uninstall` flag)

The script is wrapped in a `main()` function so that partial downloads (truncated curl) cannot execute incomplete code.

---

## R2 Bucket Structure

```
nyzhi-releases/
  install.sh                              # install script
  releases/
    latest.json                           # version manifest
    v0.2.1/
      nyzhi-darwin-aarch64.tar.gz
      nyzhi-darwin-x86_64.tar.gz
      nyzhi-linux-x86_64.tar.gz
      nyzhi-linux-aarch64.tar.gz
    v0.2.0/
      ...
```
