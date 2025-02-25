use std::fmt::Display;

use branch_context::{Message, Messages};
use chrono::{DateTime, Utc};
use rig::message::{AssistantContent, Message as RigMessage, UserContent};

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub inner: RigMessage,
    pub sent_at: DateTime<Utc>,
    pub freewill: bool,
}

pub enum MessageRole {
    User,
    Assistant,
}

impl Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

impl ChatMessage {
    pub fn assistant(content: String) -> Self {
        Self {
            inner: RigMessage::assistant(content),
            sent_at: Utc::now(),
            freewill: false,
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            inner: RigMessage::user(content),
            sent_at: Utc::now(),
            freewill: false,
        }
    }

    pub fn content(&self) -> Option<String> {
        match &self.inner {
            RigMessage::Assistant { content } => {
                let content = content.first();

                if let AssistantContent::Text(text) = content {
                    Some(text.text)
                } else {
                    None
                }
            }
            RigMessage::User { content } => {
                let content = content.first();

                if let UserContent::Text(text) = content {
                    Some(text.text)
                } else {
                    None
                }
            }
        }
    }

    pub fn role(&self) -> MessageRole {
        match &self.inner {
            RigMessage::User { .. } => MessageRole::User,
            RigMessage::Assistant { .. } => MessageRole::Assistant,
        }
    }
}

impl Default for ChatMessage {
    fn default() -> Self {
        Self {
            inner: RigMessage::user(""),
            sent_at: Utc::now(),
            freewill: false,
        }
    }
}

impl Into<ChatMessage> for &Messages<ChatMessage> {
    fn into(self) -> ChatMessage {
        // &self.selected().clone() ends up being a cheaper clone than self.into_selected()
        self.selected().clone()
    }
}

impl Into<Message<ChatMessage>> for ChatMessage {
    fn into(self) -> Message<ChatMessage> {
        Message::new(self)
    }
}

impl Into<RigMessage> for ChatMessage {
    fn into(self) -> RigMessage {
        self.inner
    }
}

impl From<RigMessage> for ChatMessage {
    fn from(message: RigMessage) -> Self {
        Self {
            inner: message,
            sent_at: Utc::now(),
            freewill: false,
        }
    }
}
