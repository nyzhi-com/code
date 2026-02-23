# Releasing

Release automation is defined in `.raccoon.toml` (not GitHub Actions workflow files).

## Pipeline source of truth

Two pipelines are configured:

- `check` on `push:master` and `pull_request`
- `release` on `tag:v*`

### Check pipeline

Runs:

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --all`

### Release pipeline

`Build and publish release` step runs shell script that:

1. derives `VERSION` from `RACCOON_REF`
2. builds with `cargo zigbuild --release -p nyzhi` for:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
3. packages `nyz` binary into:
   - `nyzhi-linux-x86_64.tar.gz`
   - `nyzhi-linux-aarch64.tar.gz`
   - `nyzhi-darwin-x86_64.tar.gz`
   - `nyzhi-darwin-aarch64.tar.gz`
4. writes `<name>.sha256` via `sha256sum`
5. uploads artifacts by POSTing to:
   - `$RACCOON_CALLBACK/api/jobs/$RACCOON_JOB_ID/artifact`
   - with bearer `$RACCOON_TOKEN`

## Tagging a release

```bash
git tag v1.1.3
git push origin v1.1.3
```

`tag:v*` triggers the release pipeline.

## Version source

Workspace version lives in `Cargo.toml`:

```toml
[workspace.package]
version = "1.1.2"
```

## Updater contract (consumer side)

`nyz update` expects release service endpoints:

- `GET <release_url>/version`
- `GET <release_url>/download/<os>/<arch>?version=<semver>`

Where `<os>/<arch>` maps to keys such as `darwin/aarch64`, `darwin/x86_64`, `linux/x86_64`, `linux/aarch64`, and checksum data returned by `/version`.

## Operational notes

- Ensure CI environment has `cargo-zigbuild` and zig toolchain availability.
- Release artifacts are generated under `target/<triple>/release/` and local packaging outputs; these are build artifacts, not source-of-truth.
- Docs about release infra should be updated from `.raccoon.toml` first when behavior changes.
