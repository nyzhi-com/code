# Skills

Source of truth:

- `crates/core/src/skills.rs`
- `crates/core/src/tools/load_skill.rs`
- `crates/cli/src/main.rs` (`nyz skills`)

## What Skills Are

Skills are reusable markdown instruction packs stored in project directories and loaded on demand.

They are intended to encode domain-specific guidance and repeatable workflows.

## Skill Directories

Load order:

1. `<project>/.nyzhi/skills/`
2. `<project>/.claude/skills/` (fallback)

Conflict rule:

- `.nyzhi/skills` wins by filename on collisions.

## Skill File Format

- file extension: `.md`
- filename stem becomes skill name
- description extraction:
  1. `description:` frontmatter line if present
  2. first non-empty non-heading line fallback

## Runtime Usage

- skill index is shown in prompt context as metadata list
- full skill content is not injected by default
- use `load_skill` tool to fetch complete content of a selected skill

## CLI and TUI Surfaces

- CLI: `nyz skills` (list learned skills)
- TUI slash command: `/learn`

## Template Helper

`build_skill_template(name, description, patterns)` generates a markdown scaffold for new skill files.

## Recommendations

- keep one skill per file and one responsibility per skill
- include concrete examples
- keep descriptions crisp so skill index remains useful
- prefer project-local skills for team conventions, fallback `.claude/skills` for shared baseline content
