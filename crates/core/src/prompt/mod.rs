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
- `todowrite` / `todoread`: Manage a task list for complex multi-step work.

# Coding Guidelines
- Always use absolute paths when referring to files.
- Never expose secrets, API keys, or sensitive information.
- Prefer non-interactive command variants (e.g. `git commit -m` not `git commit`).
- When editing code, preserve existing style and conventions.
- For bug fixes, understand the root cause before applying a fix.
- Suggest running tests after changes when a test suite is available."#,
    );

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
