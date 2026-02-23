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
    pub fn compact_with_restore(
        &mut self,
        summary: &str,
        keep_recent: usize,
        restore_files: &[PathBuf],
    ) {
        self.compact_with_rehydration(summary, keep_recent, restore_files, None, None, None);
    }

    /// Full rehydration compact: restores files, todos, plan state, and notepad content.
    pub fn compact_with_rehydration(
        &mut self,
        summary: &str,
        keep_recent: usize,
        restore_files: &[PathBuf],
        todo_summary: Option<&str>,
        plan_content: Option<&str>,
        notepad_content: Option<&str>,
    ) {
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

        let mut state_block = String::new();

        if let Some(todos) = todo_summary {
            if !todos.is_empty() {
                state_block.push_str(&format!("\n## Active Todos\n{todos}\n"));
            }
        }

        if let Some(plan) = plan_content {
            if !plan.is_empty() {
                let truncated = if plan.len() > 4000 {
                    format!("{}...[plan truncated]", &plan[..4000])
                } else {
                    plan.to_string()
                };
                state_block.push_str(&format!("\n## Active Plan\n{truncated}\n"));
            }
        }

        if let Some(notepad) = notepad_content {
            if !notepad.is_empty() {
                let truncated = if notepad.len() > 2000 {
                    format!("{}...[truncated]", &notepad[..2000])
                } else {
                    notepad.to_string()
                };
                state_block.push_str(&format!("\n## Accumulated Wisdom\n{truncated}\n"));
            }
        }

        if !state_block.is_empty() {
            self.messages.push(Message {
                role: Role::User,
                content: MessageContent::Text(format!(
                    "[Working state restored after compaction]{state_block}"
                )),
            });
        }

        let mut restoration = String::new();
        for path in restore_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                let truncated = if content.len() > 12000 {
                    format!("{}...[truncated to 12000 chars]", &content[..12000])
                } else {
                    content
                };
                restoration.push_str(&format!("\n--- {} ---\n{}\n", path.display(), truncated));
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
