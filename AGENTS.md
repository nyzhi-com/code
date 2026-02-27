# AGENTS.md

## Cursor Cloud specific instructions

This is a Rust workspace monorepo producing a single CLI binary (`nyz`). There are no external services, Docker containers, or databases to run — SQLite is bundled via `rusqlite`.

### Key commands

All standard dev commands are in `CONTRIBUTING.md` and `docs/building.md`. In short:

- **Build:** `cargo build --workspace`
- **Test:** `cargo test --workspace` (110 unit tests across core, index, tui)
- **Lint:** `cargo clippy --workspace --all-targets`
- **Format check:** `cargo fmt --all -- --check`
- **Run binary:** `./target/debug/nyz` (or `cargo run -p nyzhi -- <args>`)

### Caveats

- The `rust-toolchain.toml` pins to `stable` with `rustfmt` and `clippy` components. Rustup will auto-install the correct toolchain on first `cargo` invocation.
- The repo has pre-existing `rustfmt` diffs and `clippy` warnings (as of initial setup). CI uses `-Dwarnings` via `RUSTFLAGS` for clippy but the CI config omits `-- -D warnings` on the `clippy` step; locally `cargo clippy` will succeed with warnings.
- The `nyz` binary requires an LLM provider API key (OpenAI, Anthropic, Gemini, etc.) for interactive sessions or `nyz run`. Commands like `nyz config`, `nyz init`, `nyz sessions`, `nyz stats`, and `nyz --help` work without any API key.
- `libsqlite3-sys` is compiled from source (bundled feature) — first builds take ~1 minute, subsequent incremental builds are fast.
