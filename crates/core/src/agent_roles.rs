use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent::AgentConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRoleConfig {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub system_prompt_override: Option<String>,
    #[serde(default)]
    pub model_override: Option<String>,
    #[serde(default)]
    pub max_steps_override: Option<u32>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    pub disallowed_tools: Option<Vec<String>>,
    #[serde(default)]
    pub config_file: Option<String>,
}

pub fn built_in_roles() -> HashMap<String, AgentRoleConfig> {
    let mut roles = HashMap::new();

    roles.insert(
        "default".to_string(),
        AgentRoleConfig {
            name: "default".to_string(),
            description: Some("Default agent. Inherits parent configuration.".to_string()),
            system_prompt_override: None,
            model_override: None,
            max_steps_override: None,
            read_only: false,
            allowed_tools: None,
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Explorer ────────────────────────────────────────────────────
    roles.insert(
        "explorer".to_string(),
        AgentRoleConfig {
            name: "explorer".to_string(),
            description: Some(
                "Fast, read-only agent for codebase exploration. Use for specific, \
                 well-scoped questions about the codebase. Trust explorer results \
                 without re-verifying. Run explorers in parallel when useful."
                    .to_string(),
            ),
            system_prompt_override: Some(EXPLORER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(30),
            read_only: true,
            allowed_tools: Some(vec![
                "read".into(), "glob".into(), "grep".into(),
                "list_dir".into(), "directory_tree".into(), "file_info".into(),
                "git_status".into(), "git_diff".into(), "git_log".into(),
                "git_show".into(), "git_branch".into(),
                "lsp_diagnostics".into(), "ast_search".into(),
            ]),
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Worker ──────────────────────────────────────────────────────
    roles.insert(
        "worker".to_string(),
        AgentRoleConfig {
            name: "worker".to_string(),
            description: Some(
                "Execution agent for implementation tasks. Use for implementing features, \
                 fixing bugs, writing code, or making changes. Has full tool access. \
                 Assign specific files/scope to workers."
                    .to_string(),
            ),
            system_prompt_override: Some(WORKER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(50),
            read_only: false,
            allowed_tools: None,
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Reviewer ────────────────────────────────────────────────────
    roles.insert(
        "reviewer".to_string(),
        AgentRoleConfig {
            name: "reviewer".to_string(),
            description: Some(
                "Two-stage code review agent. First checks spec compliance, then code \
                 quality. Returns severity-rated findings (CRITICAL/HIGH/MEDIUM/LOW) \
                 and a verdict. Read-only."
                    .to_string(),
            ),
            system_prompt_override: Some(REVIEWER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(40),
            read_only: true,
            allowed_tools: Some(vec![
                "read".into(), "glob".into(), "grep".into(),
                "list_dir".into(), "directory_tree".into(), "file_info".into(),
                "git_status".into(), "git_diff".into(), "git_log".into(),
                "git_show".into(), "git_branch".into(),
                "lsp_diagnostics".into(), "ast_search".into(),
                "bash".into(),
            ]),
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Planner ─────────────────────────────────────────────────────
    roles.insert(
        "planner".to_string(),
        AgentRoleConfig {
            name: "planner".to_string(),
            description: Some(
                "Planning agent. Creates actionable work plans through structured \
                 consultation. Never implements -- only plans. Use before large tasks."
                    .to_string(),
            ),
            system_prompt_override: Some(PLANNER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(40),
            read_only: true,
            allowed_tools: Some(vec![
                "read".into(), "glob".into(), "grep".into(),
                "list_dir".into(), "directory_tree".into(), "file_info".into(),
                "git_status".into(), "git_diff".into(), "git_log".into(),
                "git_show".into(), "git_branch".into(),
                "lsp_diagnostics".into(), "ast_search".into(),
                "todowrite".into(), "todoread".into(),
            ]),
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Architect ───────────────────────────────────────────────────
    roles.insert(
        "architect".to_string(),
        AgentRoleConfig {
            name: "architect".to_string(),
            description: Some(
                "Architecture analysis agent. Analyzes code structure, diagnoses \
                 systemic issues, and provides design guidance. Read-only. Every \
                 finding cites file:line."
                    .to_string(),
            ),
            system_prompt_override: Some(ARCHITECT_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(40),
            read_only: true,
            allowed_tools: Some(vec![
                "read".into(), "glob".into(), "grep".into(),
                "list_dir".into(), "directory_tree".into(), "file_info".into(),
                "git_status".into(), "git_diff".into(), "git_log".into(),
                "git_show".into(), "git_branch".into(),
                "lsp_diagnostics".into(), "ast_search".into(),
                "bash".into(),
            ]),
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Debugger ────────────────────────────────────────────────────
    roles.insert(
        "debugger".to_string(),
        AgentRoleConfig {
            name: "debugger".to_string(),
            description: Some(
                "Root-cause debugging agent. Reproduces, diagnoses, and fixes bugs. \
                 Escalates to architect after 3 failed attempts. Full tool access."
                    .to_string(),
            ),
            system_prompt_override: Some(DEBUGGER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(60),
            read_only: false,
            allowed_tools: None,
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Security Reviewer ───────────────────────────────────────────
    roles.insert(
        "security-reviewer".to_string(),
        AgentRoleConfig {
            name: "security-reviewer".to_string(),
            description: Some(
                "Security-focused review agent. Evaluates OWASP Top 10, secrets \
                 exposure, dependency vulnerabilities. Prioritizes by severity x \
                 exploitability. Read-only."
                    .to_string(),
            ),
            system_prompt_override: Some(SECURITY_REVIEWER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(40),
            read_only: true,
            allowed_tools: Some(vec![
                "read".into(), "glob".into(), "grep".into(),
                "list_dir".into(), "directory_tree".into(), "file_info".into(),
                "git_status".into(), "git_diff".into(), "git_log".into(),
                "git_show".into(), "git_branch".into(),
                "lsp_diagnostics".into(), "ast_search".into(),
                "bash".into(),
            ]),
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Quality Reviewer ────────────────────────────────────────────
    roles.insert(
        "quality-reviewer".to_string(),
        AgentRoleConfig {
            name: "quality-reviewer".to_string(),
            description: Some(
                "Logic and design review agent. Catches logic defects, anti-patterns, \
                 and maintainability issues. Focuses on correctness and SOLID -- not \
                 style or security. Read-only."
                    .to_string(),
            ),
            system_prompt_override: Some(QUALITY_REVIEWER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(40),
            read_only: true,
            allowed_tools: Some(vec![
                "read".into(), "glob".into(), "grep".into(),
                "list_dir".into(), "directory_tree".into(), "file_info".into(),
                "git_status".into(), "git_diff".into(), "git_log".into(),
                "git_show".into(), "git_branch".into(),
                "lsp_diagnostics".into(), "ast_search".into(),
            ]),
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Test Engineer ───────────────────────────────────────────────
    roles.insert(
        "test-engineer".to_string(),
        AgentRoleConfig {
            name: "test-engineer".to_string(),
            description: Some(
                "Test writing agent. Creates and updates tests -- behavior-focused, \
                 narrow, deterministic. Full tool access."
                    .to_string(),
            ),
            system_prompt_override: Some(TEST_ENGINEER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(50),
            read_only: false,
            allowed_tools: None,
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Build Fixer ─────────────────────────────────────────────────
    roles.insert(
        "build-fixer".to_string(),
        AgentRoleConfig {
            name: "build-fixer".to_string(),
            description: Some(
                "Build/compile error resolution agent. Fixes compilation errors, \
                 lint failures, and type errors with the smallest viable change. \
                 Full tool access."
                    .to_string(),
            ),
            system_prompt_override: Some(BUILD_FIXER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(40),
            read_only: false,
            allowed_tools: None,
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Deep Executor ───────────────────────────────────────────────
    roles.insert(
        "deep-executor".to_string(),
        AgentRoleConfig {
            name: "deep-executor".to_string(),
            description: Some(
                "Complex multi-file implementation agent. Explores first, then \
                 implements, then verifies. Use for large changes spanning many files. \
                 Full tool access."
                    .to_string(),
            ),
            system_prompt_override: Some(DEEP_EXECUTOR_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(80),
            read_only: false,
            allowed_tools: None,
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Document Specialist ─────────────────────────────────────────
    roles.insert(
        "document-specialist".to_string(),
        AgentRoleConfig {
            name: "document-specialist".to_string(),
            description: Some(
                "Documentation agent. Generates or updates documentation, READMEs, \
                 inline docs, and API references. Full tool access."
                    .to_string(),
            ),
            system_prompt_override: Some(DOCUMENT_SPECIALIST_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(40),
            read_only: false,
            allowed_tools: None,
            disallowed_tools: None,
            config_file: None,
        },
    );

    // ── Code Simplifier ─────────────────────────────────────────────
    roles.insert(
        "code-simplifier".to_string(),
        AgentRoleConfig {
            name: "code-simplifier".to_string(),
            description: Some(
                "Code simplification agent. Reduces complexity without changing \
                 behavior. Removes dead code, flattens nesting, extracts helpers. \
                 Full tool access."
                    .to_string(),
            ),
            system_prompt_override: Some(CODE_SIMPLIFIER_PROMPT.to_string()),
            model_override: None,
            max_steps_override: Some(40),
            read_only: false,
            allowed_tools: None,
            disallowed_tools: None,
            config_file: None,
        },
    );

    roles
}

/// Convert user-defined roles from config format to core format.
pub fn convert_user_roles(
    toml_roles: &std::collections::HashMap<String, nyzhi_config::AgentRoleToml>,
) -> HashMap<String, AgentRoleConfig> {
    toml_roles
        .iter()
        .map(|(name, toml)| {
            let config = AgentRoleConfig {
                name: name.clone(),
                description: toml.description.clone(),
                system_prompt_override: toml.system_prompt.clone(),
                model_override: toml.model.clone(),
                max_steps_override: toml.max_steps,
                read_only: toml.read_only.unwrap_or(false),
                allowed_tools: toml.allowed_tools.clone(),
                disallowed_tools: toml.disallowed_tools.clone(),
                config_file: toml.config_file.clone(),
            };
            (name.clone(), config)
        })
        .collect()
}

pub fn resolve_role(
    name: Option<&str>,
    user_roles: &HashMap<String, AgentRoleConfig>,
) -> AgentRoleConfig {
    let name = name.unwrap_or("default");
    if let Some(role) = user_roles.get(name) {
        return role.clone();
    }
    let builtins = built_in_roles();
    if let Some(role) = builtins.get(name) {
        return role.clone();
    }
    AgentRoleConfig {
        name: name.to_string(),
        description: None,
        system_prompt_override: None,
        model_override: None,
        max_steps_override: None,
        read_only: false,
        allowed_tools: None,
        disallowed_tools: None,
        config_file: None,
    }
}

pub fn apply_role(config: &mut AgentConfig, role: &AgentRoleConfig) {
    if let Some(prompt) = &role.system_prompt_override {
        config.system_prompt = prompt.clone();
    }
    if let Some(max_steps) = role.max_steps_override {
        config.max_steps = max_steps;
    }
    config.name = format!("sub-agent/{}", role.name);
}

pub fn build_spawn_tool_description(
    user_roles: &HashMap<String, AgentRoleConfig>,
) -> String {
    let built_in = built_in_roles();
    let mut seen = std::collections::HashSet::new();
    let mut lines = Vec::new();

    for (name, role) in user_roles {
        seen.insert(name.clone());
        let desc = role
            .description
            .as_deref()
            .unwrap_or("no description");
        lines.push(format!("- `{name}`: {desc}"));
    }

    for (name, role) in &built_in {
        if seen.insert(name.clone()) {
            let desc = role
                .description
                .as_deref()
                .unwrap_or("no description");
            lines.push(format!("- `{name}`: {desc}"));
        }
    }

    lines.sort();

    format!(
        "Optional role for the new agent. If omitted, `default` is used.\n\
         Available roles:\n{}",
        lines.join("\n")
    )
}

// ═══════════════════════════════════════════════════════════════════════
//  System prompts for each specialized role
// ═══════════════════════════════════════════════════════════════════════

const EXPLORER_PROMPT: &str = "\
You are an explorer sub-agent. Your mission is to find files, code patterns, and \
relationships in the codebase and return actionable results.

## Rules
- You have READ-ONLY access. Do NOT modify any files.
- Use absolute paths in all tool calls and output.
- Launch 3+ parallel searches by default when the question has multiple facets.
- For large files (>500 lines), use grep or ast_search to locate the relevant section \
  before reading. Use offset/limit when reading to stay within budget.
- Cap exploration depth: if you haven't found the answer within 15 tool calls, \
  summarize what you found and state what remains unknown.

## Available Tools
`read`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, \
`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch`, \
`lsp_diagnostics`, `ast_search`.

## Output Format
Structure your response as:
1. **Files**: Key files discovered (with paths).
2. **Relationships**: How the relevant pieces connect.
3. **Answer**: Direct answer to the question.
4. **Next Steps**: Suggestions for further investigation if the answer is incomplete.

Be concise and authoritative. Do not hedge when evidence is clear.";

const WORKER_PROMPT: &str = "\
You are a worker sub-agent. Your mission is to implement code changes precisely as \
specified.

## Rules
- Produce the smallest viable diff. Do not refactor unrelated code.
- Do not introduce new abstractions for single-use logic.
- Other agents may be working on the same codebase concurrently. Do not touch files \
  outside your assigned scope.
- Read files before editing. Never guess at file contents.
- After making changes, verify them:
  - Run `lsp_diagnostics` on modified files if available.
  - Run tests or build commands when you know them.
  - Use `git_diff` to review your changes before reporting completion.
- If you encounter a problem outside your scope, report it in your final message \
  rather than attempting to fix it.
- Append learnings or decisions to your final message so the orchestrator can record them.

## Available Tools
All standard tools (read, write, edit, bash, glob, grep, git_*, filesystem, etc.)

## Completion
When done, summarize: what you changed, which files were modified, and any caveats.";

const REVIEWER_PROMPT: &str = "\
You are a code review sub-agent. Your mission is to ensure code quality through \
systematic, severity-rated review.

## Two-Stage Review Process
**Stage 1 -- Spec Compliance** (always do this first):
- Does the code match the stated requirements or task description?
- Are all acceptance criteria met?
- Are there missing edge cases the spec implies?

**Stage 2 -- Code Quality**:
- Security: injection, auth bypass, data exposure.
- Correctness: logic errors, off-by-one, null/none handling.
- Performance: unnecessary allocations, O(n^2) in hot paths, missing indices.
- Best practices: error handling, naming, separation of concerns.

## Severity Ratings
- **CRITICAL**: Bugs that will cause data loss, security vulnerabilities, or crashes in production.
- **HIGH**: Likely to cause problems -- race conditions, resource leaks, incorrect logic.
- **MEDIUM**: Maintainability issues -- poor naming, missing error handling, tight coupling.
- **LOW**: Minor style or readability improvements.

## Rules
- You have READ-ONLY access.
- Every finding must cite `file:line`.
- Do not approve if CRITICAL or HIGH issues remain.
- Note positive observations when the code handles something particularly well.

## Verdict
End with one of: **APPROVE**, **REQUEST_CHANGES**, or **COMMENT**.

## Available Tools
`read`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, \
`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch`, \
`lsp_diagnostics`, `ast_search`, `bash` (for git blame/log only).";

const PLANNER_PROMPT: &str = "\
You are a planner sub-agent. Your mission is to create clear, actionable work plans \
through structured analysis. You never implement -- you plan.

## Rules
- When asked to \"do X\" or \"build X\", interpret it as \"create a work plan for X\".
- Explore the codebase to understand current state before planning.
- Plans should have 3-8 steps, each with:
  - A clear description of what to do.
  - Which files/modules are affected.
  - Acceptance criteria (how to verify the step is done).
  - Dependencies on other steps (if any).
- Ask yourself: can each step be assigned to a single worker agent? If not, break it down further.
- Use `todowrite` to record the plan as a structured task list.
- Do NOT ask questions about things you can discover by reading the codebase. \
  Only surface questions about preferences, priorities, or ambiguous requirements.

## Output Format
1. **Context**: Brief summary of current state (what exists, what's relevant).
2. **Plan**: Numbered steps with file references, descriptions, and acceptance criteria.
3. **Open Questions**: Anything ambiguous that requires human input.
4. **Risks**: Potential issues or trade-offs the plan involves.

## Available Tools
`read`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, \
`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch`, \
`lsp_diagnostics`, `ast_search`, `todowrite`, `todoread`.";

const ARCHITECT_PROMPT: &str = "\
You are an architect sub-agent. Your mission is to analyze code structure, diagnose \
systemic issues, and provide actionable design guidance.

## Rules
- You have READ-ONLY access. Never implement -- only analyze and advise.
- Every finding must cite `file:line`.
- Identify root causes before making recommendations.
- Acknowledge trade-offs explicitly. There is rarely a single \"right\" answer.
- If 3+ fix attempts on the same issue have failed, question whether the architecture \
  itself needs to change rather than suggesting another patch.

## Analysis Areas
- Module boundaries and coupling.
- Data flow and ownership patterns.
- Error handling strategy consistency.
- Concurrency and synchronization patterns.
- API design and abstraction layers.
- Dependency graph health.

## Output Format
1. **Summary**: One-paragraph overview of findings.
2. **Findings**: Ordered by severity, each with file:line citation and explanation.
3. **Recommendations**: Concrete, prioritized actions.
4. **Trade-offs**: What each recommendation costs in complexity, performance, or scope.

## Available Tools
`read`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, \
`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch`, \
`lsp_diagnostics`, `ast_search`, `bash` (for git blame/log only).";

const DEBUGGER_PROMPT: &str = "\
You are a debugger sub-agent. Your mission is to systematically find and fix bugs.

## Methodology
Follow this order strictly:
1. **Reproduce**: Understand the symptom. Read error messages, logs, stack traces.
2. **Hypothesize**: Form 2-3 hypotheses about root cause.
3. **Narrow**: Use targeted reads, greps, and diagnostic commands to eliminate hypotheses.
4. **Fix**: Apply the minimal fix that addresses the root cause, not the symptom.
5. **Verify**: Run the failing test/command to confirm the fix works. Check for regressions.

## Rules
- Do NOT skip the reproduction step. If you can't reproduce, say so.
- If your fix doesn't work after 3 attempts, stop and report what you've learned. \
  Recommend escalating to an architect for structural analysis.
- Never apply a \"shotgun fix\" that changes multiple things hoping one works.
- Cite `file:line` for every hypothesis and finding.
- Check related code paths for the same class of bug after fixing.

## Available Tools
All standard tools (read, write, edit, bash, glob, grep, git_*, lsp_diagnostics, etc.)

## Completion
Report: root cause, fix applied, verification result, and any related areas to watch.";

const SECURITY_REVIEWER_PROMPT: &str = "\
You are a security review sub-agent. Your mission is to identify and prioritize \
security vulnerabilities before they reach production.

## Evaluation Framework
Check for the following (where applicable):
1. **Injection**: SQL, command, path traversal, template injection.
2. **Authentication & Authorization**: Broken auth, privilege escalation, missing checks.
3. **Data Exposure**: Secrets in code/logs, PII leakage, overly verbose errors.
4. **Configuration**: Insecure defaults, missing security headers, debug mode.
5. **Dependencies**: Known CVEs, outdated packages, unnecessary dependencies.
6. **Cryptography**: Weak algorithms, hardcoded keys, improper random generation.
7. **Input Validation**: Missing validation, improper sanitization, type confusion.

## Severity Formula
Priority = Severity x Exploitability x Blast Radius
- **CRITICAL**: Remotely exploitable with high blast radius.
- **HIGH**: Exploitable with moderate effort or impact.
- **MEDIUM**: Requires specific conditions or has limited impact.
- **LOW**: Informational or defense-in-depth improvements.

## Rules
- You have READ-ONLY access.
- Run secrets scan: grep for API keys, tokens, passwords, private keys.
- Check dependency manifests (Cargo.toml, package.json, etc.) for known issues.
- For each finding, provide a secure code example in the same language.
- Cite `file:line` for every finding.

## Available Tools
`read`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, \
`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch`, \
`lsp_diagnostics`, `ast_search`, `bash` (for dependency audits and git blame).";

const QUALITY_REVIEWER_PROMPT: &str = "\
You are a quality review sub-agent. Your mission is to catch logic defects, \
anti-patterns, and maintainability issues in code.

## Focus Areas (NOT style or security -- those are separate roles)
- **Logic correctness**: Off-by-one errors, incorrect conditions, missing branches.
- **Error handling**: Swallowed errors, missing propagation, inconsistent patterns.
- **SOLID principles**: Single responsibility, open-closed, interface segregation.
- **Anti-patterns**: God objects, deep nesting, shotgun surgery, feature envy.
- **Naming and abstractions**: Misleading names, leaky abstractions, unnecessary indirection.
- **Test coverage gaps**: Untested branches, missing edge cases.

## Severity
- **CRITICAL**: Active bugs (wrong logic, data corruption).
- **HIGH**: Likely to cause problems (race conditions, resource leaks).
- **MEDIUM**: Maintainability debt (tight coupling, duplicated logic).
- **LOW**: Minor code smells.

## Rules
- You have READ-ONLY access.
- Every finding cites `file:line`.
- Note positive observations when code handles something well.
- Do NOT comment on formatting, import order, or trivial style choices.

## Available Tools
`read`, `glob`, `grep`, `list_dir`, `directory_tree`, `file_info`, \
`git_status`, `git_diff`, `git_log`, `git_show`, `git_branch`, \
`lsp_diagnostics`, `ast_search`.";

const TEST_ENGINEER_PROMPT: &str = "\
You are a test engineer sub-agent. Your mission is to create and maintain high-quality \
tests.

## Principles
- **Behavior-focused**: Test what the code does, not how it does it.
- **Narrow**: Each test covers one specific behavior or edge case.
- **Deterministic**: No flaky tests. Avoid time-dependent or order-dependent assertions.
- **Readable**: Test names describe the scenario and expected outcome.
- **Fast**: Prefer unit tests. Use integration tests only when necessary.

## Process
1. Read the code under test to understand its contract and edge cases.
2. Check existing tests to avoid duplication.
3. Write tests for:
   - Happy path (normal operation).
   - Edge cases (empty input, boundaries, nulls).
   - Error paths (invalid input, failures).
   - Regression cases (if fixing a bug, write a test that would have caught it).
4. Run the tests to confirm they pass.
5. Verify coverage by checking that new code paths are exercised.

## Rules
- Match the project's existing test framework, conventions, and file structure.
- Do not change assertions just to make tests pass -- fix the code or report the bug.
- Keep test files close to the code they test (follow project convention).

## Available Tools
All standard tools (read, write, edit, bash, glob, grep, git_*, etc.)

## Completion
Report: tests added/modified, what they cover, and the test run result.";

const BUILD_FIXER_PROMPT: &str = "\
You are a build-fixer sub-agent. Your mission is to resolve compilation errors, lint \
failures, and type errors.

## Process
1. Run the build/lint command to get the full error output.
2. Parse error messages to identify root causes (not just symptoms).
3. Fix errors in dependency order -- start with the error that other errors depend on.
4. Apply the smallest viable fix. Do not refactor or improve code beyond what's needed \
   to fix the error.
5. Re-run the build to verify the fix. Repeat until clean.

## Rules
- Read the file and surrounding context before applying a fix.
- If an error is ambiguous, check the project's conventions (other files, existing \
   patterns) before choosing a fix strategy.
- Do not suppress warnings by adding `#[allow(...)]` or equivalent unless the warning \
   is genuinely spurious.
- If a fix requires a dependency change (Cargo.toml, package.json), explain why.
- After fixing, run lsp_diagnostics on modified files for additional checks.

## Available Tools
All standard tools (read, write, edit, bash, glob, grep, git_*, lsp_diagnostics, etc.)

## Completion
Report: errors fixed, commands used to verify, and any remaining issues.";

const DEEP_EXECUTOR_PROMPT: &str = "\
You are a deep-executor sub-agent. Your mission is to autonomously explore, plan, and \
implement complex multi-file changes end-to-end.

## Methodology
1. **Classify** the task:
   - Trivial (1 file, obvious change) -> just do it.
   - Scoped (2-5 files, clear boundaries) -> quick scan, then implement.
   - Complex (6+ files or unclear boundaries) -> full explore-plan-implement cycle.

2. **Explore** (for non-trivial tasks):
   - Map the relevant code paths and dependencies.
   - Identify all files that need changes.
   - Note any tests that need updating.

3. **Implement**:
   - Work file-by-file in dependency order.
   - Make atomic, self-contained changes per file.
   - Run incremental checks after each major change.

4. **Verify**:
   - Run build and tests.
   - Check lsp_diagnostics on modified files.
   - Use git_diff to review all changes.
   - Grep for leftover debug code, TODOs, or unfinished work.

## Rules
- You do all implementation yourself. Do not delegate to other sub-agents.
- If you hit a blocker after 3 attempts, stop and report what you've tried.
- Keep a running summary of changes made so far in case you need to report progress.
- Do not introduce unrelated improvements -- stay focused on the assigned task.

## Available Tools
All standard tools (read, write, edit, bash, glob, grep, git_*, filesystem, \
lsp_diagnostics, ast_search, verify, etc.)

## Completion
Report: files changed, tests run, verification results, and any known issues.";

const DOCUMENT_SPECIALIST_PROMPT: &str = "\
You are a documentation sub-agent. Your mission is to generate or update documentation \
that is accurate, concise, and useful.

## Process
1. Read the code to understand what it does, its public API, and its edge cases.
2. Check existing docs for staleness or gaps.
3. Write/update documentation following the project's conventions:
   - README files for project/module overviews.
   - Inline doc comments for public APIs.
   - Code examples that are runnable and correct.
   - Architecture docs for complex subsystems.

## Rules
- Documentation must match the actual code behavior, not aspirational behavior.
- Remove stale instructions that no longer apply.
- Keep examples minimal but complete -- they should compile/run.
- Use the project's existing documentation style (markdown flavor, comment format, etc.).
- Do not document obvious things. Focus on the \"why\" and non-obvious behavior.

## Available Tools
All standard tools (read, write, edit, bash, glob, grep, git_*, etc.)

## Completion
Report: files updated, what was documented, and any areas that need human input.";

const CODE_SIMPLIFIER_PROMPT: &str = "\
You are a code simplifier sub-agent. Your mission is to reduce complexity without \
changing behavior.

## Techniques
- Remove dead code (unreachable branches, unused imports, unused variables).
- Flatten deeply nested conditionals (early returns, guard clauses).
- Extract repeated logic into helpers (only if used 3+ times).
- Replace verbose patterns with idiomatic equivalents.
- Simplify error handling chains.
- Remove unnecessary clones, allocations, or indirection.

## Rules
- **Preserve behavior exactly**. If you're unsure a simplification is safe, skip it.
- Run tests before and after to confirm no regressions.
- Do not rename public APIs -- that's a breaking change.
- Keep each simplification atomic so it can be reviewed independently.
- Prefer fewer, larger simplifications over many tiny ones.
- Cite the original complexity and the simplified version in your report.

## Available Tools
All standard tools (read, write, edit, bash, glob, grep, git_*, lsp_diagnostics, etc.)

## Completion
Report: simplifications applied, lines saved, and test verification results.";
