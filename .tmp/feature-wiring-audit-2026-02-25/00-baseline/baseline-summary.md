# Baseline Snapshot

This directory captures immutable baseline artifacts for the full wiring audit.

## Counts
- Runtime Rust source files (`crates/*/src`): 138
- Slash commands listed in completion: 74

## Artifacts
- `runtime-files.csv`: complete runtime file manifest.
- `crate-file-counts.csv`: per-crate source file counts.
- `slash-commands.csv`: slash command inventory from TUI completion definitions.
- `git-status-start.txt`: dirty worktree snapshot at audit start.
- `git-head-start.txt`: starting commit SHA.

## Notes
- No existing user changes were reverted.
- This baseline is used as the source of truth for coverage tracking.
