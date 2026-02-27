# Building

Source of truth:

- `Cargo.toml`
- `rust-toolchain.toml`
- `rustfmt.toml`
- `.github/workflows/ci.yml`
- `CONTRIBUTING.md`

## Prerequisites

- Rust `stable` with:
  - `rustfmt`
  - `clippy`
- workspace rust version: `1.75+`

Pinned by:

- `rust-toolchain.toml`

## Workspace Build

From repository root:

```bash
cargo build
cargo build --release
```

Build a specific binary crate:

```bash
cargo build --release -p nyzhi
```

## Quality Checks

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Optional:

```bash
cargo check --workspace
```

## CI Reference

CI currently runs:

- format check
- clippy check
- workspace tests

Branch trigger in workflow:

- `master` for push events

## Cross-compilation Notes

Release workflow builds:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

Linux ARM64 requires cross linker:

- `gcc-aarch64-linux-gnu`
- `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc`

## Install from Source

```bash
git clone https://github.com/nyzhi-com/code
cd code
cargo build --release -p nyzhi
./target/release/nyz --version
```
