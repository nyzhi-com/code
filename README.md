# nyzhi code

A performance-optimized AI coding agent for the terminal, built in Rust.

## Features

- **Multi-provider**: OpenAI, Anthropic, and Google Gemini out of the box
- **Rich TUI**: Built with ratatui for a smooth terminal experience
- **Streaming**: Real-time token-by-token output
- **Auth**: API key and OAuth2 (PKCE + device code) support
- **Agent loop**: Tool calling with built-in tools (bash, read, write, edit, glob, grep)
- **Single binary**: Zero-dependency install

## Quick Start

```bash
# Set your API key
export OPENAI_API_KEY="sk-..."

# Run the TUI
nyzhi

# Or run a one-shot prompt
nyzhi run "explain this codebase"
```

## Configuration

Config lives at `~/.config/nyzhi/config.toml`:

```toml
[provider]
default = "openai"

[provider.openai]
model = "gpt-4.1"

[provider.anthropic]
model = "claude-sonnet-4-20250514"

[provider.gemini]
model = "gemini-2.5-flash"
```

## Building

```bash
cargo build --release
```

## License

GPL-3.0-or-later
