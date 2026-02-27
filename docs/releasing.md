# Releasing

Source of truth:

- `.github/workflows/release.yml`
- `Cargo.toml` (`workspace.package.version`)
- `npm/*/package.json`

## Trigger

Release workflow triggers on tag push:

```text
v*
```

Example:

```bash
git tag v1.2.9
git push origin v1.2.9
```

## Version Source

Canonical version is in workspace manifest:

- `Cargo.toml` -> `[workspace.package].version`

Release job strips `v` prefix from tag to derive publish version.

## Release Pipeline Stages

1. Build matrix binaries
2. Package tarballs and SHA256 files
3. Upload artifacts to CI release endpoint
4. Create GitHub release
5. Publish crates to crates.io in dependency order
6. Publish npm platform packages + umbrella package

## Build Targets

- `linux-x86_64`
- `linux-aarch64`
- `darwin-x86_64`
- `darwin-aarch64`

## crates.io Publish Order

From workflow:

1. `nyzhi-config`
2. `nyzhi-auth`
3. `nyzhi-index`
4. `nyzhi-provider`
5. `nyzhi-core`
6. `nyzhi-tui`
7. `nyzhi`

## npm Publish Flow

Published package set:

- `nyz-darwin-arm64`
- `nyz-darwin-x64`
- `nyz-linux-x64`
- `nyz-linux-arm64`
- `nyzhi` umbrella package

Workflow:

- extract platform binary into each package
- set package version to tag version
- publish platform packages
- update umbrella `optionalDependencies` to same version
- publish umbrella package

## Required Secrets

- `RACCOON_UPLOAD_SECRET`
- `CARGO_REGISTRY_TOKEN`
- `NPM_TOKEN` (mapped to `NODE_AUTH_TOKEN`)

## Pre-release Checklist

- run local verification:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
- confirm workspace version bump
- confirm npm package metadata matches target version
- review release notes/changelog content
