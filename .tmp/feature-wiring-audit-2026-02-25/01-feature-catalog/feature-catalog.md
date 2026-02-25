# Feature Catalog

## Generated inventories
- Slash command features: 74
- Tool registry features: 56
- CLI command features: 28

## Files
- `slash-command-features.csv`
- `tool-registry-features.csv`
- `cli-command-features.csv`

## Cross-crate wiring seams (primary)
- CLI -> TUI/Core/Provider/Auth/Config via `crates/cli/src/main.rs`.
- TUI -> Core agent loop/tools via `crates/tui/src/app.rs` and `crates/tui/src/input.rs`.
- Core -> Provider stream + tool runtime via `crates/core/src/agent/mod.rs` and `crates/core/src/tools/mod.rs`.
- Core -> Index for auto-context and semantic search via `crates/core/src/agent/mod.rs` and `crates/core/src/tools/semantic_search.rs`.
- Config -> Runtime wiring via `crates/config/src/lib.rs` merged settings consumed across CLI/TUI/Core/Index.
