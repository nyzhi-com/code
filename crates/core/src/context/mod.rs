use std::path::{Path, PathBuf};

use nyzhi_provider::{ContentPart, Message, MessageContent, Role};

const CHARS_PER_TOKEN: usize = 4;
const MICROCOMPACT_THRESHOLD: usize = 4000;
const HOT_TAIL_COUNT: usize = 5;
const OUTPUT_HEADROOM_TOKENS: usize = 16384;

/// Threshold for offloading tool results to files at call time (Cursor pattern).
pub const TOOL_RESULT_FILE_THRESHOLD: usize = 4000;

/// Write a large tool result to a context file and return a reference string.
/// Returns `None` if the result is small enough to keep inline.
pub fn offload_tool_result_to_file(
    tool_name: &str,
    _tool_use_id: &str,
    content: &str,
    context_dir: &Path,
) -> Option<String> {
    if content.len() < TOOL_RESULT_FILE_THRESHOLD {
        return None;
    }
    let dir = context_dir.join("tool-results");
    std::fs::create_dir_all(&dir).ok()?;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let safe_name: String = tool_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    let filename = format!("{ts}-{safe_name}.txt");
    let file_path = dir.join(&filename);
    std::fs::write(&file_path, content).ok()?;
    let line_count = content.lines().count();
    Some(format!(
        "[Output written to {} ({} lines, {} chars). Use read_file or tail_file to inspect.]",
        file_path.display(),
        line_count,
        content.len(),
    ))
}

/// Save the full conversation history to a JSONL file for post-compaction retrieval.
/// Returns the path to the saved history file.
pub fn save_history_file(
    messages: &[Message],
    session_id: &str,
    compact_count: u32,
    context_dir: &Path,
) -> Option<PathBuf> {
    let dir = context_dir.join("history");
    std::fs::create_dir_all(&dir).ok()?;
    let filename = format!("{session_id}-{compact_count}.jsonl");
    let file_path = dir.join(&filename);
    let mut lines = Vec::new();
    for msg in messages {
        if let Ok(json) = serde_json::to_string(msg) {
            lines.push(json);
        }
    }
    std::fs::write(&file_path, lines.join("\n")).ok()?;
    Some(file_path)
}

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
    build_compaction_prompt_full(messages, focus_hint, None, None)
}

pub fn build_compaction_prompt_full(
    messages: &[Message],
    focus_hint: Option<&str>,
    previous_summary: Option<&str>,
    compact_instructions: Option<&str>,
) -> String {
    let mut transcript = String::new();
    let mut tool_call_count = 0;
    let mut file_set: std::collections::HashSet<String> = std::collections::HashSet::new();

    for msg in messages {
        let role = match msg.role {
            Role::User => "User",
            Role::Assistant => "Assistant",
            Role::System => "System",
            Role::Tool => "Tool",
        };

        match &msg.content {
            MessageContent::Text(text) if !text.is_empty() => {
                let truncated = if text.len() > 3000 {
                    format!("{}...[truncated]", &text[..3000])
                } else {
                    text.clone()
                };
                transcript.push_str(&format!("{role}: {truncated}\n\n"));
            }
            MessageContent::Parts(parts) => {
                let mut part_text = String::new();
                for part in parts {
                    match part {
                        ContentPart::Text { text } => {
                            let t = if text.len() > 2000 {
                                format!("{}...", &text[..2000])
                            } else {
                                text.clone()
                            };
                            part_text.push_str(&t);
                        }
                        ContentPart::ToolUse { name, input, .. } => {
                            tool_call_count += 1;
                            if let Some(path) = input.get("path").and_then(|v| v.as_str()) {
                                file_set.insert(path.to_string());
                            }
                            part_text.push_str(&format!("[tool:{name}] "));
                        }
                        ContentPart::ToolResult { content, .. } => {
                            let preview = if content.len() > 500 {
                                format!("{}...", &content[..500])
                            } else {
                                content.clone()
                            };
                            part_text.push_str(&format!("→ {preview}\n"));
                        }
                        ContentPart::Image { .. } => {
                            part_text.push_str("[image] ");
                        }
                    }
                }
                if !part_text.is_empty() {
                    transcript.push_str(&format!("{role}: {part_text}\n\n"));
                }
            }
            _ => {}
        }
    }

    let focus = match focus_hint {
        Some(hint) => format!("\n**Focus**: {hint}\n"),
        None => String::new(),
    };

    let delta_context = match previous_summary {
        Some(prev) => format!(
            "\n## Previous Summary (for delta update)\nThe conversation was previously compacted. \
             Here is the last summary — update and extend it rather than starting from scratch:\n\n{prev}\n\n---\n"
        ),
        None => String::new(),
    };

    let custom = match compact_instructions {
        Some(inst) => format!("\n## Project-Specific Instructions\n{inst}\n"),
        None => String::new(),
    };

    let stats = format!(
        "\nConversation stats: {} messages, {} tool calls, {} unique files touched.\n",
        messages.len(), tool_call_count, file_set.len()
    );

    format!(
        "You are a compaction engine. Produce a structured working state summary that enables \
         seamless continuation of this coding session. Your summary will REPLACE the conversation \
         history, so completeness is critical — anything you omit is lost forever.\n\n\
         {delta_context}\
         ## Required Sections\n\n\
         ### 1. Primary Request and Intent\n\
         The user's original request and how it evolved. Include exact requirements.\n\n\
         ### 2. Key Technical Decisions\n\
         Architecture choices, library selections, approach decisions, and WHY each was made.\n\n\
         ### 3. Files and Code Sections\n\
         Every file touched, with:\n\
         - File path\n\
         - Summary of changes made\n\
         - Important code snippets if they contain non-obvious logic\n\n\
         ### 4. Errors and Fixes\n\
         Every error encountered, its root cause, and the fix applied. These are critical for \
         avoiding regressions. Skip this section entirely if no errors occurred.\n\n\
         ### 5. Problem Solving\n\
         Key debugging steps, investigations, and solutions that led to breakthroughs.\n\n\
         ### 6. Current State\n\
         What is complete, what is partially done, and what remains untouched.\n\n\
         ### 7. Pending Tasks\n\
         Numbered list of remaining work items, in priority order.\n\n\
         ### 8. Next Step\n\
         The single most important next action to take.\n\
         {focus}\
         {custom}\
         {stats}\
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

/// Deduplicate tool results: if the same file was read/written multiple times,
/// keep only the most recent result. Returns number of entries deduplicated.
pub fn dedup_tool_results(messages: &mut Vec<Message>) -> usize {
    let mut seen_files: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut to_collapse: Vec<usize> = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        if let MessageContent::Parts(parts) = &msg.content {
            for part in parts {
                if let ContentPart::ToolUse { name, input, .. } = part {
                    if matches!(name.as_str(), "read" | "read_file" | "write" | "write_file" | "edit" | "edit_file") {
                        if let Some(path) = input.get("path").and_then(|v| v.as_str()) {
                            if let Some(prev) = seen_files.insert(path.to_string(), i) {
                                to_collapse.push(prev);
                                if prev + 1 < messages.len() {
                                    to_collapse.push(prev + 1);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    to_collapse.sort_unstable();
    to_collapse.dedup();

    let count = to_collapse.len();
    for &idx in to_collapse.iter().rev() {
        if idx < messages.len() {
            let msg = &mut messages[idx];
            msg.content = MessageContent::Text("[earlier duplicate — superseded by later access]".to_string());
        }
    }
    count
}

/// Prune errored tool inputs after they're N messages old.
/// Keeps the error message but collapses the original tool call input.
pub fn prune_old_errors(messages: &mut [Message], age_threshold: usize) -> usize {
    let len = messages.len();
    if len < age_threshold {
        return 0;
    }
    let cutoff = len - age_threshold;
    let mut pruned = 0;

    for i in 0..cutoff {
        let msg = &messages[i];
        let has_error = if let MessageContent::Parts(parts) = &msg.content {
            parts.iter().any(|p| {
                if let ContentPart::ToolResult { content, .. } = p {
                    content.contains("Error:") || content.contains("error:") || content.starts_with("ERROR")
                } else {
                    false
                }
            })
        } else {
            false
        };

        if has_error && i > 0 {
            let prev = &mut messages[i - 1];
            if let MessageContent::Parts(parts) = &prev.content {
                let is_tool_call = parts.iter().any(|p| matches!(p, ContentPart::ToolUse { .. }));
                if is_tool_call {
                    prev.content = MessageContent::Text("[tool call that produced error — input collapsed]".to_string());
                    pruned += 1;
                }
            }
        }
    }
    pruned
}

/// Collapse write tool results when the file was subsequently read.
/// The read result supersedes the write content.
pub fn supersede_writes(messages: &mut Vec<Message>) -> usize {
    let mut write_indices: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    let mut read_files: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (i, msg) in messages.iter().enumerate().rev() {
        if let MessageContent::Parts(parts) = &msg.content {
            for part in parts {
                if let ContentPart::ToolUse { name, input, .. } = part {
                    let path = input.get("path").and_then(|v| v.as_str());
                    if let Some(path) = path {
                        if matches!(name.as_str(), "read" | "read_file") {
                            read_files.insert(path.to_string());
                        } else if matches!(name.as_str(), "write" | "write_file") {
                            write_indices.entry(path.to_string()).or_default().push(i);
                        }
                    }
                }
            }
        }
    }

    let mut collapsed = 0;
    for (path, indices) in &write_indices {
        if read_files.contains(path) {
            for &idx in indices {
                if idx + 1 < messages.len() {
                    let result_msg = &mut messages[idx + 1];
                    let token_cost = estimate_message_tokens(result_msg);
                    if token_cost > 30 {
                        if let MessageContent::Parts(parts) = &mut result_msg.content {
                            for part in parts.iter_mut() {
                                if let ContentPart::ToolResult { content, .. } = part {
                                    if content.len() > 200 {
                                        *content = format!("[write result for {} — file was later read, content superseded]", path);
                                        collapsed += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    collapsed
}

/// Run the full progressive compaction pipeline.
/// Returns (tokens_saved, description) for transparency.
pub fn progressive_compact(
    messages: &mut Vec<Message>,
    storage_dir: &Path,
    estimated_tokens: usize,
    context_window: u32,
    threshold: f64,
) -> Vec<(String, usize)> {
    let mut savings: Vec<(String, usize)> = Vec::new();
    let target = (context_window as f64 * threshold) as usize;

    if estimated_tokens <= target {
        return savings;
    }

    let before = messages.iter().map(estimate_message_tokens).sum::<usize>();

    let deduped = dedup_tool_results(messages);
    if deduped > 0 {
        let after = messages.iter().map(estimate_message_tokens).sum::<usize>();
        let saved = before.saturating_sub(after);
        savings.push((format!("dedup: {deduped} entries collapsed"), saved));
        if after + estimate_tokens("") <= target {
            return savings;
        }
    }

    let before2 = messages.iter().map(estimate_message_tokens).sum::<usize>();
    let superseded = supersede_writes(messages);
    if superseded > 0 {
        let after = messages.iter().map(estimate_message_tokens).sum::<usize>();
        let saved = before2.saturating_sub(after);
        savings.push((format!("supersede: {superseded} write results collapsed"), saved));
    }

    let before3 = messages.iter().map(estimate_message_tokens).sum::<usize>();
    let pruned = prune_old_errors(messages, 8);
    if pruned > 0 {
        let after = messages.iter().map(estimate_message_tokens).sum::<usize>();
        let saved = before3.saturating_sub(after);
        savings.push((format!("error prune: {pruned} failed tool inputs collapsed"), saved));
    }

    let before4 = messages.iter().map(estimate_message_tokens).sum::<usize>();
    let offloaded = microcompact(messages, storage_dir);
    if offloaded > 0 {
        let after = messages.iter().map(estimate_message_tokens).sum::<usize>();
        let saved = before4.saturating_sub(after);
        savings.push((format!("microcompact: {offloaded} tool outputs offloaded"), saved));
    }

    savings
}

/// Check if full compaction is still needed after progressive passes.
pub fn needs_full_compact(
    estimated_tokens: usize,
    context_window: u32,
    threshold: f64,
    message_count: usize,
) -> bool {
    let target = (context_window as f64 * threshold) as usize;
    let headroom = (context_window as usize).saturating_sub(estimated_tokens);
    estimated_tokens > target && message_count > 10 && headroom < OUTPUT_HEADROOM_TOKENS * 2
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
            tool_result_msg("recent_4", &large_content),
            tool_result_msg("recent_5", &large_content),
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
            tool_result_msg("r4", &large),
            tool_result_msg("r5", &large),
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
        assert!(prompt.contains("Primary Request and Intent"));
        assert!(prompt.contains("Key Technical Decisions"));
        assert!(prompt.contains("Files and Code Sections"));
        assert!(prompt.contains("Current State"));
        assert!(prompt.contains("Next Step"));
    }

    #[test]
    fn structured_prompt_includes_focus() {
        let messages = vec![text_msg(Role::User, "Test")];
        let prompt = build_compaction_prompt(&messages, Some("API changes"));
        assert!(prompt.contains("API changes"));
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

    #[test]
    fn dedup_collapses_repeated_reads() {
        let mut messages = vec![
            tool_use_msg("read_file", "/src/main.rs"),
            tool_result_msg("tu_1", &"a".repeat(500)),
            tool_use_msg("read_file", "/src/main.rs"),
            tool_result_msg("tu_2", &"b".repeat(500)),
        ];
        let count = dedup_tool_results(&mut messages);
        assert!(count > 0);
        let first_text = messages[0].content.as_text();
        assert!(first_text.contains("superseded"));
    }

    #[test]
    fn supersede_writes_collapses_when_read_follows() {
        let mut messages = vec![
            tool_use_msg("write_file", "/src/main.rs"),
            tool_result_msg("tu_1", &"x".repeat(1000)),
            tool_use_msg("read_file", "/src/main.rs"),
            tool_result_msg("tu_2", &"y".repeat(500)),
        ];
        let count = supersede_writes(&mut messages);
        assert!(count > 0);
    }

    #[test]
    fn prune_old_errors_collapses_tool_input() {
        let error_msg = "Error: file not found";
        let mut messages = vec![
            tool_use_msg("read_file", "/nonexistent.rs"),
            tool_result_msg("tu_1", error_msg),
        ];
        for _ in 0..10 {
            messages.push(text_msg(Role::User, "continue"));
            messages.push(text_msg(Role::Assistant, "ok"));
        }
        let count = prune_old_errors(&mut messages, 8);
        assert!(count > 0);
    }

    #[test]
    fn needs_full_compact_checks_headroom() {
        assert!(needs_full_compact(180_000, 200_000, 0.85, 15));
        assert!(!needs_full_compact(100_000, 200_000, 0.85, 15));
        assert!(!needs_full_compact(180_000, 200_000, 0.85, 5));
    }

    #[test]
    fn build_compaction_prompt_full_includes_delta() {
        let messages = vec![text_msg(Role::User, "Build API")];
        let prompt = build_compaction_prompt_full(
            &messages,
            Some("API layer"),
            Some("Previous summary here"),
            Some("Focus on TypeScript"),
        );
        assert!(prompt.contains("Previous Summary"));
        assert!(prompt.contains("Previous summary here"));
        assert!(prompt.contains("Focus on TypeScript"));
        assert!(prompt.contains("API layer"));
    }
}
