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
        if self.messages.len() <= keep_recent {
            return;
        }
        let split = self.messages.len() - keep_recent;
        let recent: Vec<Message> = self.messages.drain(split..).collect();
        self.messages.clear();
        self.messages.push(Message {
            role: Role::User,
            content: MessageContent::Text(format!(
                "[Conversation summary]\n{summary}"
            )),
        });
        self.messages.extend(recent);
    }
}

impl Default for Thread {
    fn default() -> Self {
        Self::new()
    }
}
