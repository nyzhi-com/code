use nyzhi_provider::{ContentPart, Message, MessageContent};

const CHARS_PER_TOKEN: usize = 4;

pub fn estimate_tokens(text: &str) -> usize {
    text.len() / CHARS_PER_TOKEN + 1
}

pub fn estimate_message_tokens(msg: &Message) -> usize {
    let content_tokens = match &msg.content {
        MessageContent::Text(text) => estimate_tokens(text),
        MessageContent::Parts(parts) => parts
            .iter()
            .map(|p| match p {
                ContentPart::Text { text } => estimate_tokens(text),
                ContentPart::ToolUse { name, input, .. } => {
                    estimate_tokens(name) + estimate_tokens(&input.to_string())
                }
                ContentPart::ToolResult { content, .. } => estimate_tokens(content),
            })
            .sum(),
    };
    // ~4 tokens overhead per message for role/formatting
    content_tokens + 4
}

pub fn estimate_thread_tokens(messages: &[Message], system_prompt: &str) -> usize {
    let system_tokens = estimate_tokens(system_prompt);
    let message_tokens: usize = messages.iter().map(estimate_message_tokens).sum();
    system_tokens + message_tokens
}

pub fn should_compact(estimated_tokens: usize, context_window: u32) -> bool {
    let threshold = (context_window as f64 * 0.8) as usize;
    estimated_tokens > threshold
}

pub fn build_compaction_prompt(messages: &[Message]) -> String {
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
            transcript.push_str(&format!("{role}: {text}\n\n"));
        }
    }

    format!(
        "Summarize this conversation concisely, preserving key decisions, code changes, file paths, \
         and any important context. Keep the summary under 500 words.\n\n---\n\n{transcript}"
    )
}
