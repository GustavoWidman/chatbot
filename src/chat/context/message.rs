use branch_context::{Message, Messages};
use chrono::{DateTime, Utc};
use openai_api_rs::v1::chat_completion::{self, ChatCompletionMessage, MessageRole, ToolCall};

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub sent_at: DateTime<Utc>,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

impl Default for ChatMessage {
    fn default() -> Self {
        Self {
            role: "user".to_string(),
            content: "".to_string(),
            sent_at: Utc::now(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

impl Into<ChatCompletionMessage> for ChatMessage {
    fn into(self) -> ChatCompletionMessage {
        let role = match self.role.as_str() {
            "system" => MessageRole::system,
            "user" => MessageRole::user,
            "assistant" => MessageRole::assistant,
            "tool" => MessageRole::tool,
            "function" => MessageRole::function,
            _ => MessageRole::system,
        };

        chat_completion::ChatCompletionMessage {
            role,
            content: chat_completion::Content::Text(self.content),
            name: self.name,
            tool_calls: self.tool_calls,
            tool_call_id: self.tool_call_id,
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
