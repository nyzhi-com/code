use std::path::{Path, PathBuf};

use nyzhi_provider::{ContentPart, Message, MessageContent};

const CHARS_PER_TOKEN: usize = 4;
const MICROCOMPACT_THRESHOLD: usize = 4000;
const HOT_TAIL_COUNT: usize = 3;

pub fn estimate_tokens(text: &str) -> usize {
    text.len() / CHARS_PER_TOKEN + 1
}

pub const IMAGE_TOKEN_ESTIMATE: usize = 1000;

pub fn estimate_message_tokens(msg: &Message) -> usize {
    let content_tokens = match &msg.content {
        MessageContent::Text(text) => estimate_tokens(text),
        MessageContent::Parts(parts) => parts
            .iter()
            .map(|p| match p {
                ContentPart::Text { text } => estimate_tokens(text),
                ContentPart::Image { .. } => IMAGE_TOKEN_ESTIMATE,
                ContentPart::ToolUse { name, input, .. } => {
                    estimate_tokens(name) + estimate_tokens(&input.to_string())
                }
                ContentPart::ToolResult { content, .. } => estimate_tokens(content),
            })
            .sum(),
    };
    content_tokens + 4
}

pub fn estimate_thread_tokens(messages: &[Message], system_prompt: &str) -> usize {
    let system_tokens = estimate_tokens(system_prompt);
    let message_tokens: usize = messages.iter().map(estimate_message_tokens).sum();
    system_tokens + message_tokens
}

pub fn should_compact(estimated_tokens: usize, context_window: u32) -> bool {
    should_compact_at(estimated_tokens, context_window, 0.8)
}

pub fn should_compact_at(estimated_tokens: usize, context_window: u32, ratio: f64) -> bool {
    let threshold = (context_window as f64 * ratio.clamp(0.1, 0.99)) as usize;
    estimated_tokens > threshold
}

pub fn build_compaction_prompt(messages: &[Message], focus_hint: Option<&str>) -> String {
    let mut transcript = String::new();
    for msg in messages {
        let role = match msg.role {
            nyzhi_provider::Role::User => "User",
            nyzhi_provider::Role::Assistant => "Assistant",
            nyzhi_provider::Role::System => "System",
            nyzhi_provider::Role::Tool => "Tool",
        };
        let text = msg.content.as_text();
        if !text.is_empty() {
            let truncated = if text.len() > 2000 {
                format!("{}...[truncated]", &text[..2000])
            } else {
                text.to_string()
            };
            transcript.push_str(&format!("{role}: {truncated}\n\n"));
        }
    }

    let focus = match focus_hint {
        Some(hint) => format!("\nPay special attention to: {hint}\n"),
        None => String::new(),
    };

    format!(
        "Summarize this conversation into a structured working state that allows \
         continuation without re-asking questions. Include these sections:\n\n\
         ## User Intent\n\
         What the user asked for and any changes to the original request.\n\n\
         ## Key Decisions\n\
         Technical decisions made and why.\n\n\
         ## Files Changed\n\
         Files touched with brief description of changes.\n\n\
         ## Errors & Fixes\n\
         Errors encountered and how they were resolved. Skip if none.\n\n\
         ## Current State\n\
         What has been completed and what remains.\n\n\
         ## Next Step\n\
         The immediate next action to continue work.\n\
         {focus}\n\
         ---\n\n{transcript}"
    )
}

/// Microcompaction: offload large tool results from older messages to disk.
/// Keeps the last `HOT_TAIL_COUNT` tool-result messages fully inline.
/// Returns the number of tool results offloaded.
pub fn microcompact(messages: &mut [Message], storage_dir: &Path) -> usize {
    std::fs::create_dir_all(storage_dir).ok();

    let tool_result_indices: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| {
            matches!(&m.content, MessageContent::Parts(parts) if parts.iter().any(|p| matches!(p, ContentPart::ToolResult { .. })))
        })
        .map(|(i, _)| i)
        .collect();

    let cold_count = tool_result_indices.len().saturating_sub(HOT_TAIL_COUNT);
    let cold_indices: Vec<usize> = tool_result_indices.into_iter().take(cold_count).collect();

    let mut offloaded = 0;
    for idx in cold_indices {
        let msg = &mut messages[idx];
        if let MessageContent::Parts(parts) = &mut msg.content {
            for part in parts.iter_mut() {
                if let ContentPart::ToolResult { tool_use_id, content } = part {
                    if content.len() < MICROCOMPACT_THRESHOLD {
                        continue;
                    }
                    let filename = format!("tool_result_{}.txt", tool_use_id.replace(['/', '\\', ':'], "_"));
                    let file_path = storage_dir.join(&filename);
                    if std::fs::write(&file_path, content.as_bytes()).is_ok() {
                        let chars = content.len();
                        *content = format!(
                            "[Tool output saved to {} ({} chars). Use read_file to retrieve if needed.]",
                            file_path.display(),
                            chars,
                        );
                        offloaded += 1;
                    }
                }
            }
        }
    }
    offloaded
}

/// Compute detailed context breakdown for `/context` display.
pub struct ContextBreakdown {
    pub system_prompt_tokens: usize,
    pub message_tokens: usize,
    pub message_count: usize,
    pub tool_result_tokens: usize,
    pub total_tokens: usize,
    pub context_window: u32,
    pub auto_compact_threshold: f64,
}

impl ContextBreakdown {
    pub fn compute(messages: &[Message], system_prompt: &str, context_window: u32, threshold: f64) -> Self {
        let system_prompt_tokens = estimate_tokens(system_prompt);
        let mut message_tokens = 0usize;
        let mut tool_result_tokens = 0usize;

        for msg in messages {
            let msg_tokens = estimate_message_tokens(msg);
            message_tokens += msg_tokens;

            if let MessageContent::Parts(parts) = &msg.content {
                for part in parts {
                    if let ContentPart::ToolResult { content, .. } = part {
                        tool_result_tokens += estimate_tokens(content) + 4;
                    }
                }
            }
        }

        let total_tokens = system_prompt_tokens + message_tokens;

        Self {
            system_prompt_tokens,
            message_tokens: message_tokens - tool_result_tokens,
            message_count: messages.len(),
            tool_result_tokens,
            total_tokens,
            context_window,
            auto_compact_threshold: threshold,
        }
    }

    pub fn usage_percent(&self) -> f64 {
        if self.context_window == 0 {
            return 0.0;
        }
        (self.total_tokens as f64 / self.context_window as f64) * 100.0
    }

    pub fn compact_at_tokens(&self) -> usize {
        (self.context_window as f64 * self.auto_compact_threshold) as usize
    }

    pub fn headroom(&self) -> usize {
        let window = self.context_window as usize;
        window.saturating_sub(self.total_tokens)
    }

    pub fn format_display(&self) -> String {
        let pct = self.usage_percent();
        let compact_at = self.compact_at_tokens();
        format!(
            "Context Usage ({} / {} tokens = {:.1}%)\n\
             \x20 System prompt:  {} tokens\n\
             \x20 Messages ({}):  {} tokens\n\
             \x20 Tool results:   {} tokens\n\
             Auto-compact at:    {:.0}% ({} tokens)\n\
             Headroom:           {} tokens remaining",
            format_token_count(self.total_tokens),
            format_token_count(self.context_window as usize),
            pct,
            format_token_count(self.system_prompt_tokens),
            self.message_count,
            format_token_count(self.message_tokens),
            format_token_count(self.tool_result_tokens),
            self.auto_compact_threshold * 100.0,
            format_token_count(compact_at),
            format_token_count(self.headroom()),
        )
    }
}

fn format_token_count(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Extract file paths mentioned in recent tool results for post-compaction restoration.
pub fn extract_recent_file_paths(messages: &[Message], max_files: usize) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for msg in messages.iter().rev() {
        if let MessageContent::Parts(parts) = &msg.content {
            for part in parts {
                match part {
                    ContentPart::ToolUse { name, input, .. } if name == "read_file" || name == "write_file" || name == "edit_file" => {
                        if let Some(path_str) = input.get("path").and_then(|v| v.as_str()) {
                            let p = PathBuf::from(path_str);
                            if !paths.contains(&p) {
                                paths.push(p);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if paths.len() >= max_files {
            break;
        }
    }

    paths.truncate(max_files);
    paths
}

pub const CONTINUATION_MESSAGE: &str = "\
This session is being continued from a compacted conversation. \
The summary above covers the earlier portion. Continue the current \
task without re-asking the user any questions.";

#[cfg(test)]
mod tests {
    use super::*;
    use nyzhi_provider::Role;

    fn text_msg(role: Role, text: &str) -> Message {
        Message {
            role,
            content: MessageContent::Text(text.to_string()),
        }
    }

    fn tool_result_msg(tool_use_id: &str, content: &str) -> Message {
        Message {
            role: Role::User,
            content: MessageContent::Parts(vec![ContentPart::ToolResult {
                tool_use_id: tool_use_id.to_string(),
                content: content.to_string(),
            }]),
        }
    }

    fn tool_use_msg(name: &str, path: &str) -> Message {
        Message {
            role: Role::Assistant,
            content: MessageContent::Parts(vec![ContentPart::ToolUse {
                id: "tu_1".to_string(),
                name: name.to_string(),
                input: serde_json::json!({ "path": path }),
            }]),
        }
    }

    #[test]
    fn microcompact_offloads_large_results() {
        let dir = tempfile::tempdir().unwrap();
        let large_content = "x".repeat(5000);
        let mut messages = vec![
            tool_result_msg("old_1", &large_content),
            tool_result_msg("old_2", "small output"),
            tool_result_msg("recent_1", &large_content),
            tool_result_msg("recent_2", &large_content),
            tool_result_msg("recent_3", &large_content),
        ];

        let offloaded = microcompact(&mut messages, dir.path());
        assert_eq!(offloaded, 1);

        if let MessageContent::Parts(parts) = &messages[0].content {
            if let ContentPart::ToolResult { content, .. } = &parts[0] {
                assert!(content.contains("Tool output saved to"));
            } else {
                panic!("Expected ToolResult");
            }
        }
        // Small result should be unchanged
        if let MessageContent::Parts(parts) = &messages[1].content {
            if let ContentPart::ToolResult { content, .. } = &parts[0] {
                assert_eq!(content, "small output");
            }
        }
    }

    #[test]
    fn microcompact_hot_tail_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let large = "y".repeat(5000);
        let mut messages = vec![
            tool_result_msg("r1", &large),
            tool_result_msg("r2", &large),
            tool_result_msg("r3", &large),
        ];

        let offloaded = microcompact(&mut messages, dir.path());
        assert_eq!(offloaded, 0);
    }

    #[test]
    fn structured_prompt_contains_sections() {
        let messages = vec![
            text_msg(Role::User, "Build a REST API"),
            text_msg(Role::Assistant, "I'll create the API using axum."),
        ];
        let prompt = build_compaction_prompt(&messages, None);
        assert!(prompt.contains("## User Intent"));
        assert!(prompt.contains("## Key Decisions"));
        assert!(prompt.contains("## Files Changed"));
        assert!(prompt.contains("## Current State"));
        assert!(prompt.contains("## Next Step"));
    }

    #[test]
    fn structured_prompt_includes_focus() {
        let messages = vec![text_msg(Role::User, "Test")];
        let prompt = build_compaction_prompt(&messages, Some("API changes"));
        assert!(prompt.contains("Pay special attention to: API changes"));
    }

    #[test]
    fn should_compact_at_85_percent() {
        assert!(!should_compact_at(169_000, 200_000, 0.85));
        assert!(should_compact_at(171_000, 200_000, 0.85));
    }

    #[test]
    fn context_breakdown_compute() {
        let messages = vec![
            text_msg(Role::User, "Hello"),
            text_msg(Role::Assistant, "Hi there"),
        ];
        let bd = ContextBreakdown::compute(&messages, "system", 200_000, 0.85);
        assert!(bd.total_tokens > 0);
        assert!(bd.usage_percent() < 1.0);
        assert_eq!(bd.compact_at_tokens(), 170_000);
        assert!(bd.headroom() > 199_000);
    }

    #[test]
    fn extract_file_paths_from_tool_use() {
        let messages = vec![
            tool_use_msg("read_file", "/src/main.rs"),
            tool_result_msg("tu_1", "file contents"),
            tool_use_msg("write_file", "/src/lib.rs"),
            tool_result_msg("tu_2", "done"),
        ];
        let paths = extract_recent_file_paths(&messages, 5);
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&PathBuf::from("/src/lib.rs")));
        assert!(paths.contains(&PathBuf::from("/src/main.rs")));
    }

    #[test]
    fn format_token_count_units() {
        assert_eq!(format_token_count(500), "500");
        assert_eq!(format_token_count(1500), "1.5k");
        assert_eq!(format_token_count(1_500_000), "1.5M");
    }
}
