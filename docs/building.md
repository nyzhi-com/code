# Building from source

## Prerequisites

- Rust `1.75`+ (`[workspace.package].rust-version`)
- Standard system linker/toolchain for your platform
- Optional for release-style cross builds: `cargo-zigbuild` + zig

## Workspace layout

```text
Cargo.toml
crates/
  cli/      (package: nyzhi, binary: nyz)
  core/     (nyzhi-core)
  provider/ (nyzhi-provider)
  auth/     (nyzhi-auth)
  tui/      (nyzhi-tui)
  config/   (nyzhi-config)
```

## Build

```bash
git clone https://github.com/nyzhi-com/code.git
cd code
cargo build --release -p nyzhi
```

Binary:

- `target/release/nyz`

Debug build:

```bash
cargo build -p nyzhi
```

## Run locally

```bash
./target/release/nyz
```

or install from workspace path:

```bash
cargo install --path crates/cli
```

## Verification commands

These align with `.raccoon.toml` check pipeline:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Common focused runs:

```bash
cargo test -p nyzhi-core
cargo test -p nyzhi-tui
cargo test -p nyzhi-config
```

## Release-style cross builds

Release pipeline uses:

```bash
cargo zigbuild --release --target x86_64-unknown-linux-gnu -p nyzhi
cargo zigbuild --release --target aarch64-unknown-linux-gnu -p nyzhi
cargo zigbuild --release --target x86_64-apple-darwin -p nyzhi
cargo zigbuild --release --target aarch64-apple-darwin -p nyzhi
```

## Artifacts and boundaries

- `target/` is generated build output.
- Any `node_modules/` in the repo tree is dependency output for JS tooling, not product behavior definition.
- `.git/` metadata drives VCS state, but not application runtime semantics.

Use source files and config (`crates/*`, `Cargo.toml`, `.raccoon.toml`) as documentation authority.
