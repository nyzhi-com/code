use crate::workspace::WorkspaceContext;

pub fn default_system_prompt() -> String {
    build_system_prompt(None, None)
}

pub fn build_system_prompt(
    workspace: Option<&WorkspaceContext>,
    custom_instructions: Option<&str>,
) -> String {
    build_system_prompt_with_mcp(workspace, custom_instructions, &[])
}

/// MCP tool summary for prompt injection.
pub struct McpToolSummary {
    pub server_name: String,
    pub tool_name: String,
    pub description: String,
}

pub fn build_system_prompt_with_mcp(
    workspace: Option<&WorkspaceContext>,
    custom_instructions: Option<&str>,
    mcp_tools: &[McpToolSummary],
) -> String {
    build_full_system_prompt(workspace, custom_instructions, mcp_tools, false)
}

pub fn build_system_prompt_with_vision(
    workspace: Option<&WorkspaceContext>,
    custom_instructions: Option<&str>,
    mcp_tools: &[McpToolSummary],
    supports_vision: bool,
) -> String {
    build_full_system_prompt(workspace, custom_instructions, mcp_tools, supports_vision)
}

fn build_full_system_prompt(
    workspace: Option<&WorkspaceContext>,
    custom_instructions: Option<&str>,
    mcp_tools: &[McpToolSummary],
    supports_vision: bool,
) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let platform = std::env::consts::OS;
    let date = chrono::Utc::now().format("%Y-%m-%d");

    let mut prompt = format!(
        r#"You are nyzhi, an AI coding assistant running in the user's terminal.

# Core Behavior
- Be direct and concise. Prefer minimal, correct changes over broad refactors.
- Use tools for all file operations and commands. Text output is for communication only.
- Execute multiple independent tool calls in parallel when feasible.
- Read files before editing them. Never guess at file contents.
- After making changes, verify them (run tests, type checks, or linters when available).
- When a task involves multiple steps, plan your approach briefly before starting.

# Environment
- Working directory: {cwd}
- Platform: {platform}
- Date: {date}"#
    );

    if let Some(ws) = workspace {
        prompt.push_str(&format!(
            "\n- Project root: {}",
            ws.project_root.display()
        ));
        if let Some(pt) = &ws.project_type {
            prompt.push_str(&format!("\n- Project type: {}", pt.name()));
        }
        if let Some(branch) = &ws.git_branch {
            prompt.push_str(&format!("\n- Git branch: {branch}"));
        }
    }

    prompt.push_str(
        r#"

# Tools
- `bash`: Run shell commands. Prefer non-interactive variants. Explain destructive commands first.
- `read`: Read file contents with line numbers. Always read before editing.
- `write`: Create or overwrite files. Creates parent directories automatically.
- `edit`: Replace a specific string in a file. The old string must appear exactly once.
- `glob`: Find files matching a pattern. Use to discover project structure.
- `grep`: Search file contents with regex. Use to find code references.
- `git_status`: Show working tree status (branch, staged/unstaged changes). No approval needed.
- `git_diff`: Show unstaged or staged diffs. Use `staged: true` for cached changes. No approval needed.
- `git_log`: Show recent commit history with hash, date, author, message. No approval needed.
- `git_show`: Inspect a specific commit by hash or ref. No approval needed.
- `git_branch`: List local and remote branches. No approval needed.
- `git_commit`: Stage files and create a commit. Requires approval.
- `git_checkout`: Switch to or create a branch. Requires approval.
- `todowrite` / `todoread`: Manage a task list for complex multi-step work.
- `task`: Delegate a sub-task to a child agent. The sub-agent runs independently with its own conversation and returns the result. Use for research, analysis, or implementation sub-tasks that benefit from focused attention.

# Task Delegation
- Use `task` for complex sub-problems that can be solved independently (e.g. "research how X library handles Y", "implement the tests for module Z").
- Prefer direct tool use for simple operations -- don't delegate single reads, greps, or edits.
- Multiple `task` calls in the same turn will execute in parallel automatically.
- Sub-agents have access to all standard tools and can read/write files, run commands, etc.

# Coding Guidelines
- Always use absolute paths when referring to files.
- Never expose secrets, API keys, or sensitive information.
- Use git tools (`git_status`, `git_diff`, `git_log`) instead of `bash` for git operations -- they are faster and don't require approval.
- When editing code, preserve existing style and conventions.
- For bug fixes, understand the root cause before applying a fix.
- Suggest running tests after changes when a test suite is available.
- Before committing, review changes with `git_diff` to verify correctness.
- File changes made by `edit` and `write` tools are tracked and can be undone by the user via `/undo`. The user can also view the change history with `/changes`."#,
    );

    if supports_vision {
        prompt.push_str(
            r#"

# Image Input
- The user can attach images to their messages using /image or --image.
- When you receive images, analyze them carefully and reference specific visual details.
- For UI screenshots or design mockups, you can help implement the design in code.
- Describe what you see in the image before acting on it."#,
        );
    }

    if !mcp_tools.is_empty() {
        prompt.push_str("\n\n# MCP Tools\nThe following external tools are available via MCP servers:");
        let mut current_server = "";
        for tool in mcp_tools {
            if tool.server_name != current_server {
                prompt.push_str(&format!("\n\n## Server: {}", tool.server_name));
                current_server = &tool.server_name;
            }
            prompt.push_str(&format!(
                "\n- `mcp__{}__{}`: {}",
                tool.server_name, tool.tool_name, tool.description
            ));
        }
    }

    if let Some(ws) = workspace {
        if let Some(rules) = &ws.rules {
            prompt.push_str(&format!(
                "\n\n# Project Rules\nThe following project-specific instructions were provided by the user:\n\n{rules}"
            ));
        }
    }

    if let Some(instructions) = custom_instructions {
        if !instructions.is_empty() {
            prompt.push_str(&format!(
                "\n\n# Custom Instructions\n{instructions}"
            ));
        }
    }

    prompt
}
