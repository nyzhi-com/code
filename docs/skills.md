# Skills

Skills are reusable patterns that Nyzhi learns from your sessions. Once learned, they are available across all future sessions in the same project, giving the agent institutional knowledge about your codebase and workflows.

---

## Learning a Skill

Use `/learn` in the TUI to extract a pattern from the current session:

```
/learn api-error-handling
```

This creates a skill template at `.nyzhi/skills/api-error-handling.md`. The agent analyzes the session to extract patterns, conventions, and reusable approaches.

### Skill Template Structure

```markdown
---
description: How we handle API errors in this project
---

# API Error Handling

## Pattern

All API handlers use the `AppError` type for error responses...

## Examples

```rust
async fn get_user(id: Path<Uuid>) -> Result<Json<User>, AppError> {
    // ...
}
```

## Rules

- Always return structured error responses
- Log errors at the handler boundary
- Use specific error variants, not generic strings
```

---

## Storage

Skills are stored as markdown files in `.nyzhi/skills/` in the project root:

```
.nyzhi/
  skills/
    api-error-handling.md
    database-migrations.md
    test-patterns.md
```

Nyzhi also reads `.claude/skills/` for compatibility. If both directories contain a skill with the same name, the `.nyzhi/skills/` version wins.

---

## Listing Skills

```bash
# CLI
nyz skills

# TUI
/learn
```

Shows all learned skills with their descriptions.

---

## How Skills Are Used

Skills are loaded lazily to keep the system prompt small:

1. At session start, a compact skill index is included in the system prompt (name + description only).
2. When the agent determines a skill is relevant, it calls the `load_skill` tool to fetch the full content.
3. The loaded skill content is injected into the conversation context.

This means skills don't consume context budget until they're actually needed.

---

## Creating Skills Manually

You can create skill files directly without using `/learn`:

```markdown
<!-- .nyzhi/skills/testing.md -->
---
description: Testing conventions for this project
---

# Testing Conventions

## Structure
- Unit tests go in `src/tests/`
- Integration tests go in `tests/`
- Use `#[cfg(test)]` modules for inline unit tests

## Naming
- Test functions: `test_<what>_<scenario>`
- Test modules: `tests` (inline) or `test_<module>` (file)

## Patterns
- Use `tempfile` for filesystem tests
- Use `mockall` for mocking traits
- Assert specific error variants, not just `is_err()`
```

---

## Description Extraction

The skill description (shown in the index) is extracted from:

1. A `description:` field in YAML frontmatter, if present.
2. Otherwise, the first non-heading, non-empty line of the file.

---

## Skill Templates

When you use `/learn <name>`, the agent generates a skill template with:

- A description summarizing the pattern
- The patterns and conventions observed
- Concrete code examples from the session
- Rules and guidelines

You can edit the generated file to refine it.
