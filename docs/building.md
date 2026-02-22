# Building from Source

Nyzhi is a standard Rust project. Build it with cargo.

---

## Prerequisites

- **Rust 1.75+** -- install via [rustup.rs](https://rustup.rs/)
- **A C linker** -- usually already present on macOS and Linux
- No other dependencies. All Rust crates are fetched from crates.io.

---

## Build

```bash
git clone https://github.com/nyzhi-com/code.git
cd code
cargo build --release
```

The binary is at `target/release/nyz`.

### Debug Build

```bash
cargo build
# Binary: target/debug/nyz
```

Debug builds are faster to compile but significantly slower at runtime.

---

## Run

```bash
# Run directly
./target/release/nyz

# Or install to PATH
cargo install --path crates/cli

# Or copy to your nyzhi install directory
cp target/release/nyz ~/.nyzhi/bin/nyz
```

---

## Workspace Structure

The project is a Cargo workspace with six crates:

```
Cargo.toml              # workspace root
crates/
  cli/                  # nyzhi (binary: nyz)
  core/                 # nyzhi-core (agent, tools, sessions, etc.)
  provider/             # nyzhi-provider (LLM abstraction)
  auth/                 # nyzhi-auth (OAuth, API keys)
  tui/                  # nyzhi-tui (terminal UI)
  config/               # nyzhi-config (configuration loading)
```

Build a specific crate:

```bash
cargo build --release -p nyzhi        # just the CLI binary
cargo build --release -p nyzhi-core   # just the core library
```

---

## Tests

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p nyzhi-core
cargo test -p nyzhi-tui
cargo test -p nyzhi-config

# Run a specific test
cargo test -p nyzhi-core -- commands::tests::expand_template
```

### Test Coverage

Tests exist in:

- `nyzhi-core`: commands, workspace, context, hooks, keywords, memory, skills
- `nyzhi-tui`: completion, export, theme, highlight
- `nyzhi-config`: config loading and parsing

Some tests use `tempfile` for filesystem operations.

---

## Linting

```bash
cargo clippy --all -- -D warnings
```

---

## Formatting

```bash
cargo fmt --all
cargo fmt --all -- --check   # check without modifying
```

---

## Cross-Compilation

For cross-compiling to other targets:

```bash
# Install the target
rustup target add aarch64-apple-darwin

# Build
cargo build --release --target aarch64-apple-darwin -p nyzhi
```

For Linux cross-compilation from macOS (or vice versa), use [cross](https://github.com/cross-rs/cross):

```bash
cargo install cross --version 0.2.5
cross build --release --target aarch64-unknown-linux-gnu -p nyzhi
```

---

## Supported Targets

| Target | OS | Architecture |
|--------|----|-------------|
| `x86_64-unknown-linux-gnu` | Linux | x86_64 |
| `aarch64-unknown-linux-gnu` | Linux | ARM64 |
| `x86_64-apple-darwin` | macOS | x86_64 (Intel) |
| `aarch64-apple-darwin` | macOS | ARM64 (Apple Silicon) |

---

## Key Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| tokio | 1.x | Async runtime |
| reqwest | 0.12 | HTTP client (rustls-tls) |
| ratatui | 0.29 | Terminal UI framework |
| crossterm | 0.28 | Terminal input/output |
| clap | 4.x | CLI argument parsing |
| serde | 1.x | Serialization |
| syntect | 5.x | Syntax highlighting |
| rmcp | 0.16 | MCP client |
| oauth2 | 5.x | OAuth2 flows |

See `Cargo.toml` for the complete dependency list.
