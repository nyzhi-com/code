use std::path::PathBuf;

use chrono::{DateTime, Utc};
use nyzhi_provider::{Message, MessageContent, Role};
use serde::{Deserialize, Serialize};

use crate::context;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub created_at: DateTime<Utc>,
    messages: Vec<Message>,
}

impl Thread {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            messages: Vec::new(),
        }
    }

    pub fn push_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn estimated_tokens(&self, system_prompt: &str) -> usize {
        context::estimate_thread_tokens(&self.messages, system_prompt)
    }

    /// Replace older messages with a summary, keeping the most recent `keep_recent` messages.
    pub fn compact(&mut self, summary: &str, keep_recent: usize) {
        self.compact_with_restore(summary, keep_recent, &[]);
    }

    /// Compact with post-compaction file restoration.
    /// After summarizing, re-reads the given files and injects their contents
    /// plus a continuation instruction so the agent picks up seamlessly.
    pub fn compact_with_restore(&mut self, summary: &str, keep_recent: usize, restore_files: &[PathBuf]) {
        if self.messages.len() <= keep_recent {
            return;
        }
        let split = self.messages.len() - keep_recent;
        let recent: Vec<Message> = self.messages.drain(split..).collect();
        self.messages.clear();

        self.messages.push(Message {
            role: Role::User,
            content: MessageContent::Text(format!("[Conversation summary]\n{summary}")),
        });

        let mut restoration = String::new();
        for path in restore_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                let truncated = if content.len() > 8000 {
                    format!("{}...[truncated to 8000 chars]", &content[..8000])
                } else {
                    content
                };
                restoration.push_str(&format!(
                    "\n--- {} ---\n{}\n",
                    path.display(),
                    truncated
                ));
            }
        }

        if !restoration.is_empty() {
            self.messages.push(Message {
                role: Role::User,
                content: MessageContent::Text(format!(
                    "[Recently accessed files restored after compaction]{restoration}"
                )),
            });
        }

        self.messages.push(Message {
            role: Role::User,
            content: MessageContent::Text(context::CONTINUATION_MESSAGE.to_string()),
        });

        self.messages.extend(recent);
    }
}

impl Default for Thread {
    fn default() -> Self {
        Self::new()
    }
}
