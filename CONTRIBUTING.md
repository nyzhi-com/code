# Contributing to nyzhi

Thanks for your interest in contributing! This document covers the process for
contributing to this repository.

## Getting Started

```bash
git clone https://github.com/nyzhi-com/code && cd code
cargo build          # debug build
cargo test           # run tests
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Requires **Rust 1.75+**. The repo includes a `rust-toolchain.toml` that pins to
stable with `rustfmt` and `clippy` components.

Need troubleshooting help before contributing? See [SUPPORT.md](SUPPORT.md).

## Development Workflow

1. Fork the repository and create a branch from `main`.
2. Make your changes in small, focused commits.
3. Add or update tests for any changed behavior.
4. Ensure all checks pass locally:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
5. Open a pull request against `main`.

## Project Layout

```
crates/
  cli/       Binary entry point, CLI parsing, command dispatch
  core/      Agent loop, tools, sessions, workspace, MCP, teams
  provider/  LLM abstraction (OpenAI, Anthropic, Gemini)
  tui/       Terminal UI (ratatui)
  auth/      OAuth2 PKCE, API keys, token store
  config/    Configuration loading and merging
```

## What to Contribute

- **Bug fixes** -- always welcome. Please include a regression test when practical.
- **Documentation** -- fixes, clarifications, new examples.
- **New tools** -- add them in `crates/core/src/tools/`. Each tool needs a
  descriptor and an implementation of the `Tool` trait.
- **Provider support** -- new LLM providers go in `crates/provider/src/`.
- **TUI improvements** -- themes, accessibility, keybindings.

For larger changes (new features, architectural shifts), please open an issue
first to discuss the approach.

## Issue and PR Workflows

- **Issue forms**: bug, feature, and documentation templates are available in
  GitHub Issues.
- **Automatic issue triage**: new and reopened issues are labeled
  `needs-triage`.
- **Automatic PR labeling**: pull requests are labeled by changed paths
  (for example: `core`, `provider`, `documentation`, `ci`).
- **Automatic stale management**: inactive issues and PRs are marked stale first
  and only closed after an additional inactivity period.
- **Automatic reviewers**: CODEOWNERS routes review requests to maintainers.

## Pull Request Guidelines

- Keep PRs focused on a single concern.
- Write a clear description of **what** changed and **why**.
- Reference any related issues with `Fixes #123` or `Closes #123`.
- Make sure CI is green before requesting review.

## Code Style

- Follow standard `rustfmt` formatting (see `rustfmt.toml`).
- No `#[allow(clippy::...)]` without a comment explaining why.
- Prefer returning `Result` over panicking.
- Keep public APIs documented with `///` doc comments.
- Avoid adding dependencies unless necessary. If you add one, explain why in
  the PR description.

## Commit Messages

Use concise, imperative-mood messages:

```
fix: handle empty response from Gemini streaming endpoint
feat: add Groq provider support
docs: clarify MCP server configuration
```

## Reporting Bugs

Open an issue using the **Bug Report** template. Include:

- nyzhi version (`nyz --version`)
- OS and architecture
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or error output

If your report is about docs quality rather than behavior, use the
**Documentation** issue template.

## Code of Conduct

This project follows [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). By participating,
you agree to uphold it.

## Security

If you discover a security vulnerability, **do not** open a public issue.
See [SECURITY.md](SECURITY.md) for responsible disclosure instructions.

## License

By contributing, you agree that your contributions will be licensed under the
[GPL-3.0-or-later](LICENSE) license.
