# Workspace and Rules Internals

Source of truth: `crates/core/src/workspace/mod.rs`.

## Workspace Root Detection

`find_project_root(start)` walks upward and stops at first match:

1. `.nyzhi/` directory
2. `.claude/` directory
3. `.git` marker
4. fallback to start path

## Config Source Classification

`ConfigSource` is assigned in this priority:

1. `Nyzhi` when `.nyzhi/` exists
2. `ClaudeCode` when `.claude/` exists
3. `Cursor` when `.cursorrules` exists
4. `GitOnly` when `.git` exists
5. `None`

## Rule File Priority

`load_rules` selects first non-empty primary file:

1. `AGENTS.md`
2. `.nyzhi/rules.md`
3. `.nyzhi/instructions.md`
4. `CLAUDE.md`
5. `.cursorrules`

Then optional local preferences:

- `NYZHI.local.md`
- `.nyzhi/local.md`

Then modular rules from `.nyzhi/rules/*.md`:

- sorted by filename
- unconditional rule bodies appended
- conditional rules (`paths:` frontmatter) skipped in general load path

## Conditional Rules

`load_conditional_rules(root, file_path)`:

- scans `.nyzhi/rules/*.md`
- extracts `paths:` patterns from frontmatter
- applies lightweight glob matching
- returns bodies whose pattern matches target file path

Supported matching helpers:

- `simple_glob` (`*` wildcard)
- partial `**` handling in `glob_matches`

## Scaffold Behavior (`nyz init`)

`scaffold_nyzhi_dir(root)` creates:

- `.nyzhi/config.toml`
- `.nyzhi/rules.md`
- `.nyzhi/rules/` directory
- `.nyzhi/commands/` directory
- `.nyzhi/commands/review.md`
- `NYZHI.local.md`

It also appends to `.gitignore` when needed:

- `NYZHI.local.md`
- `.nyzhi/local.md`

## Why This Matters

Rule and workspace resolution directly affects:

- system prompt construction
- agent behavior consistency across tools/editors
- local-vs-committed preference separation
