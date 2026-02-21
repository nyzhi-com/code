use chrono::{DateTime, Utc};
use nyzhi_provider::Message;

#[derive(Debug)]
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

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

impl Default for Thread {
    fn default() -> Self {
        Self::new()
    }
}
