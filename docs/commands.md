# Custom Commands

Custom commands let you define reusable prompt templates as slash commands. They are useful for standardizing common workflows like code review, test generation, or documentation.

---

## Defining Commands

### Method 1: Markdown Files

Create `.md` files in `.nyzhi/commands/` in your project root:

```markdown
<!-- .nyzhi/commands/review.md -->
# Review code for bugs and security issues
Review $ARGUMENTS for bugs, security vulnerabilities, and potential improvements.
Focus on edge cases and error handling.
```

- The first line (`# ...`) becomes the command description.
- The rest is the prompt template.
- `$ARGUMENTS` is replaced with everything after the command name.

Usage:

```
/review src/auth.rs
```

Expands to: *"Review src/auth.rs for bugs, security vulnerabilities, and potential improvements. Focus on edge cases and error handling."*

### Method 2: Config

Define commands inline in `config.toml`:

```toml
[[agent.commands]]
name = "test"
prompt = "Write comprehensive tests for $ARGUMENTS. Cover edge cases, error paths, and happy paths."
description = "Generate tests for a module"

[[agent.commands]]
name = "explain"
prompt = "Explain how $ARGUMENTS works in detail. Include data flow, key types, and error handling."
description = "Explain a code path"

[[agent.commands]]
name = "doc"
prompt = "Write documentation for $ARGUMENTS. Include usage examples and parameter descriptions."
description = "Generate documentation"
```

---

## Precedence

When the same command name exists in multiple sources:

1. **Config commands** override file-based commands with the same name.
2. **`.nyzhi/commands/`** takes precedence over `.claude/commands/` (for compatibility).

---

## `$ARGUMENTS` Expansion

The `$ARGUMENTS` placeholder is replaced with everything the user types after the command name:

| Input | Expansion |
|-------|-----------|
| `/review src/main.rs` | `$ARGUMENTS` → `src/main.rs` |
| `/test the auth module` | `$ARGUMENTS` → `the auth module` |
| `/explain` | `$ARGUMENTS` → `` (empty string) |

---

## Listing Commands

```
/commands
```

Shows all available custom commands with their descriptions.

---

## Compatibility

Nyzhi also scans `.claude/commands/` for command files, providing compatibility with Claude Code projects. If both directories contain a command with the same name, the `.nyzhi/commands/` version wins.

---

## Examples

### Code review

```markdown
<!-- .nyzhi/commands/review.md -->
# Thorough code review
Review $ARGUMENTS with focus on:
1. Correctness and edge cases
2. Security vulnerabilities
3. Performance issues
4. Error handling gaps
5. Test coverage

Present findings ordered by severity.
```

### Refactor

```markdown
<!-- .nyzhi/commands/refactor.md -->
# Safe refactoring
Refactor $ARGUMENTS to improve readability and maintainability.
- Keep changes minimal and reversible
- Maintain all existing behavior
- Update tests to match
- Run verification after changes
```

### Documentation

```markdown
<!-- .nyzhi/commands/doc.md -->
# Generate documentation
Write clear, concise documentation for $ARGUMENTS.
Include:
- Purpose and overview
- Public API with parameter descriptions
- Usage examples
- Error cases
```
