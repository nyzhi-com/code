use std::path::Path;

const MAX_BRIEFING_LINES: usize = 60;
const MAX_CHANGE_ENTRIES: usize = 20;
const MAX_MESSAGE_PREVIEW: usize = 5;

/// Shared context state updated after each turn, consumed by subagents at spawn.
#[derive(Debug, Clone, Default)]
pub struct SharedContext {
    pub recent_changes: Vec<String>,
    pub active_todos: Vec<(String, String)>,
    pub conversation_summary: Vec<String>,
    pub project_root: Option<std::path::PathBuf>,
}

impl SharedContext {
    pub fn build_briefing(&self) -> String {
        let mut lines = Vec::new();

        if !self.recent_changes.is_empty() {
            lines.push("## Recent File Changes".to_string());
            for (i, change) in self.recent_changes.iter().take(MAX_CHANGE_ENTRIES).enumerate() {
                lines.push(format!("{}. {change}", i + 1));
            }
            if self.recent_changes.len() > MAX_CHANGE_ENTRIES {
                lines.push(format!(
                    "... and {} more",
                    self.recent_changes.len() - MAX_CHANGE_ENTRIES
                ));
            }
            lines.push(String::new());
        }

        if !self.active_todos.is_empty() {
            lines.push("## Active Todos".to_string());
            for (id, content) in &self.active_todos {
                lines.push(format!("- [{id}] {content}"));
            }
            lines.push(String::new());
        }

        if !self.conversation_summary.is_empty() {
            lines.push("## Recent Conversation".to_string());
            for msg in self.conversation_summary.iter().take(MAX_MESSAGE_PREVIEW) {
                lines.push(format!("- {msg}"));
            }
            lines.push(String::new());
        }

        if let Some(root) = &self.project_root {
            let mem = crate::memory::load_memory_for_prompt(root);
            if !mem.is_empty() {
                lines.push("## Project Memory".to_string());
                let mem_lines: Vec<&str> = mem.lines().take(20).collect();
                lines.push(mem_lines.join("\n"));
                lines.push(String::new());
            }
        }

        let result: Vec<String> = lines.into_iter().take(MAX_BRIEFING_LINES).collect();
        if result.is_empty() {
            return String::new();
        }
        result.join("\n")
    }

    pub fn update_changes(&mut self, changes: Vec<String>) {
        self.recent_changes = changes;
    }

    pub fn update_todos(&mut self, todos: Vec<(String, String)>) {
        self.active_todos = todos;
    }

    pub fn update_conversation(&mut self, messages: Vec<String>) {
        self.conversation_summary = messages;
    }
}

/// Build a briefing string from a ToolContext and thread for ad-hoc injection.
pub fn build_briefing_from_context(
    project_root: &Path,
    change_tracker: &crate::tools::change_tracker::ChangeTracker,
    thread: &crate::conversation::Thread,
) -> String {
    let changes: Vec<String> = change_tracker
        .changed_files()
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    let messages: Vec<String> = thread
        .messages()
        .iter()
        .rev()
        .take(MAX_MESSAGE_PREVIEW)
        .map(|m| {
            let role = match m.role {
                nyzhi_provider::Role::User => "user",
                nyzhi_provider::Role::Assistant => "assistant",
                nyzhi_provider::Role::System => "system",
                nyzhi_provider::Role::Tool => "tool",
            };
            let text = match &m.content {
                nyzhi_provider::MessageContent::Text(t) => t.clone(),
                nyzhi_provider::MessageContent::Parts(parts) => parts
                    .iter()
                    .filter_map(|p| match p {
                        nyzhi_provider::ContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(""),
            };
            let truncated = if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text
            };
            format!("[{role}] {truncated}")
        })
        .collect();

    let ctx = SharedContext {
        recent_changes: changes,
        active_todos: Vec::new(),
        conversation_summary: messages,
        project_root: Some(project_root.to_path_buf()),
    };

    ctx.build_briefing()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context_produces_empty_briefing() {
        let ctx = SharedContext::default();
        assert!(ctx.build_briefing().is_empty());
    }

    #[test]
    fn briefing_includes_changes() {
        let ctx = SharedContext {
            recent_changes: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            ..Default::default()
        };
        let briefing = ctx.build_briefing();
        assert!(briefing.contains("src/main.rs"));
        assert!(briefing.contains("Recent File Changes"));
    }

    #[test]
    fn briefing_includes_todos() {
        let ctx = SharedContext {
            active_todos: vec![("t1".to_string(), "fix the bug".to_string())],
            ..Default::default()
        };
        let briefing = ctx.build_briefing();
        assert!(briefing.contains("fix the bug"));
        assert!(briefing.contains("Active Todos"));
    }

    #[test]
    fn briefing_truncates_long_changes() {
        let changes: Vec<String> = (0..30).map(|i| format!("file_{i}.rs")).collect();
        let ctx = SharedContext {
            recent_changes: changes,
            ..Default::default()
        };
        let briefing = ctx.build_briefing();
        assert!(briefing.contains("and 10 more"));
    }
}
