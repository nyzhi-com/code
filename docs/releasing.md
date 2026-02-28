# Releasing

## Quick Reference

```bash
# 1. Bump version
#    Edit Cargo.toml: workspace.package.version and all workspace dep pins
#    Edit npm/*/package.json and npm/nyz/package.json optionalDependencies

# 2. Build
cargo build --release

# 3. Package
mkdir -p /tmp/nyzhi-release && cd /tmp/nyzhi-release
rm -f nyz nyzhi-darwin-aarch64.tar.gz latest.json
cp <repo>/target/release/nyz nyz
tar czf nyzhi-darwin-aarch64.tar.gz nyz
SHA=$(shasum -a 256 nyzhi-darwin-aarch64.tar.gz | cut -d' ' -f1)

# 4. Upload to R2
npx wrangler r2 object put nyzhi-releases/releases/v<VERSION>/nyzhi-darwin-aarch64.tar.gz \
  --file nyzhi-darwin-aarch64.tar.gz --remote
echo '{"version":"<VERSION>","date":"...","sha256":{"darwin-aarch64":"'$SHA'"}}' > latest.json
npx wrangler r2 object put nyzhi-releases/releases/latest.json \
  --file latest.json --remote

# 5. Commit, tag, push
git add -A && git commit -m "v<VERSION>: description"
git tag v<VERSION>
git push origin master --tags

# 6. GitHub release
gh release create v<VERSION> --title "v<VERSION>" --generate-notes
```

## Version Locations

All of these must match:

| File | Field |
|------|-------|
| `Cargo.toml` | `[workspace.package].version` |
| `Cargo.toml` | Each `nyzhi-*` workspace dependency pin (`"=X.Y.Z"`) |
| `npm/nyz/package.json` | `version` and `optionalDependencies` versions |
| `npm/nyz-darwin-arm64/package.json` | `version` |
| `npm/nyz-darwin-x64/package.json` | `version` |
| `npm/nyz-linux-arm64/package.json` | `version` |
| `npm/nyz-linux-x64/package.json` | `version` |

Shortcut to bump npm packages (replace OLD/NEW):

```bash
for f in npm/*/package.json; do
  sed -i '' 's/"version": "OLD"/"version": "NEW"/' "$f"
done
sed -i '' 's/"OLD"/"NEW"/g' npm/nyz/package.json
```

## Build Targets

| Platform | Archive name |
|----------|-------------|
| macOS ARM (Apple Silicon) | `nyzhi-darwin-aarch64.tar.gz` |
| macOS Intel | `nyzhi-darwin-x86_64.tar.gz` |
| Linux x86_64 | `nyzhi-linux-x86_64.tar.gz` |
| Linux ARM64 | `nyzhi-linux-aarch64.tar.gz` |

The tarball must contain a single binary named `nyz` (the install script expects this).

## R2 Bucket Structure

Bucket: `nyzhi-releases` (served via `get.nyzhi.com`)

```
releases/
  latest.json              # current version + checksums
  v3.0.0/
    nyzhi-darwin-aarch64.tar.gz
    nyzhi-darwin-x86_64.tar.gz
    nyzhi-linux-x86_64.tar.gz
    nyzhi-linux-aarch64.tar.gz
  v1.2.13/
    ...
install.sh                 # install script served at get.nyzhi.com/
```

### latest.json Format

```json
{
  "version": "3.0.0",
  "date": "2026-02-28T13:48:00Z",
  "sha256": {
    "darwin-aarch64": "<sha256>",
    "darwin-x86_64": "<sha256>",
    "linux-x86_64": "<sha256>",
    "linux-aarch64": "<sha256>"
  }
}
```

For local-only releases (single platform), only include the platform you built.

## CI Release (GitHub Actions)

When `.github/workflows/release.yml` is present, pushing a `v*` tag triggers
automated cross-platform builds. The workflow:

1. Builds all 4 platform targets
2. Computes SHA256 checksums
3. Uploads tarballs to R2
4. Updates `latest.json` in R2
5. Creates a GitHub release with auto-generated notes

## Manual Release (No CI)

When CI is disabled, releases are done locally for darwin-aarch64 and uploaded
to R2 via `npx wrangler`. Other platforms won't be available until CI is
re-enabled.

### Step-by-Step

1. **Verify the build compiles**

   ```bash
   cargo check
   ```

2. **Bump versions** in `Cargo.toml` and `npm/*/package.json`

3. **Verify the bump**

   ```bash
   cargo check  # confirms workspace dep pins resolve
   ```

4. **Build release binary**

   ```bash
   cargo build --release
   ```

5. **Package the tarball**

   ```bash
   mkdir -p /tmp/nyzhi-release && cd /tmp/nyzhi-release
   rm -f nyz nyzhi-darwin-aarch64.tar.gz latest.json
   cp /path/to/repo/target/release/nyz nyz
   tar czf nyzhi-darwin-aarch64.tar.gz nyz
   ```

   The binary inside the tarball **must** be named `nyz`.

6. **Compute checksum and create latest.json**

   ```bash
   SHA=$(shasum -a 256 nyzhi-darwin-aarch64.tar.gz | cut -d' ' -f1)
   echo '{"version":"<VER>","date":"<ISO8601>","sha256":{"darwin-aarch64":"'$SHA'"}}' > latest.json
   ```

7. **Upload to R2**

   ```bash
   npx wrangler r2 object put \
     nyzhi-releases/releases/v<VER>/nyzhi-darwin-aarch64.tar.gz \
     --file nyzhi-darwin-aarch64.tar.gz --remote

   npx wrangler r2 object put \
     nyzhi-releases/releases/latest.json \
     --file latest.json --remote
   ```

   Note: uploads can be slow (30s-3min). If wrangler auth expires mid-upload,
   retry the command.

8. **Commit, tag, push**

   ```bash
   git add -A
   git commit -m "v<VER>: brief description"
   git tag v<VER>
   git push origin master --tags
   ```

9. **Create GitHub release**

   ```bash
   gh release create v<VER> --title "v<VER>" --generate-notes
   ```

10. **Verify**

    ```bash
    curl -s https://get.nyzhi.com/version
    curl -fsSL https://get.nyzhi.com | sh
    ```

## Install Script

The install script at `infra/releases-worker/install.sh` is served at
`get.nyzhi.com` and `get.nyzhi.com/install.sh`. To update it:

```bash
npx wrangler r2 object put nyzhi-releases/install.sh \
  --file infra/releases-worker/install.sh --remote
```

## Worker

The Cloudflare Worker serving `get.nyzhi.com` lives at
`infra/releases-worker/`. To deploy changes:

```bash
cd infra/releases-worker
npm install
npx wrangler deploy
```

## Troubleshooting

**"Could not find nyz binary in archive"** — the tarball contains a binary
with the wrong name. It must be `nyz`, not `nyzhi`.

**Wrangler auth errors** — wrangler OAuth tokens expire. Just retry the
command; it will re-authenticate.

**"Already up to date"** — `latest.json` in R2 wasn't updated, or its
version doesn't match the new tag. Re-upload `latest.json`.
