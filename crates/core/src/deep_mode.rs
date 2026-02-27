pub const DEEP_MODE_INSTRUCTIONS: &str = r#"## Deep Mode Active

You are operating in DEEP MODE. This means:

1. RESEARCH PHASE: Before making ANY edits, thoroughly explore the codebase.
   - Read all relevant files systematically
   - Use grep, glob, directory_tree, and semantic_search extensively
   - Understand the full context before proposing changes
   - Spend significant time reading; do NOT rush to write

2. PLAN PHASE: After research, compose a detailed plan.
   - Describe what you found during research
   - Outline every change you will make and why
   - Identify potential risks or side effects

3. EXECUTE PHASE: Only after planning, make the changes.
   - Implement changes methodically
   - Verify each change with appropriate tests/checks
   - If something unexpected comes up, return to research

Key behaviors:
- Do NOT ask for confirmation during the research phase
- Do NOT make edits until you have a comprehensive understanding
- Prefer reading 20 files over guessing about 1
- When in doubt, read more code
- Be thorough, not fast
"#;

pub fn deep_mode_system_suffix() -> &'static str {
    DEEP_MODE_INSTRUCTIONS
}

pub fn is_deep_prefix(input: &str) -> bool {
    let lower = input.trim_start().to_lowercase();
    lower.starts_with("deep:") || lower.starts_with("/deep ")
}

pub fn strip_deep_prefix(input: &str) -> &str {
    let trimmed = input.trim_start();
    if let Some(rest) = trimmed.strip_prefix("deep:") {
        rest.trim_start()
    } else if let Some(rest) = trimmed.strip_prefix("/deep ") {
        rest.trim_start()
    } else if let Some(rest) = trimmed.strip_prefix("/deep") {
        rest.trim_start()
    } else {
        trimmed
    }
}
