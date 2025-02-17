use serenity::all::{Context, Message, MessageUpdateEvent};

use crate::chat::{engine::EngineGuard, ChatMessage};

use super::super::Handler;

// todo add proper error handling instead of silently returning
impl Handler {
    pub async fn on_edit(
        &self,
        _: Context,
        _: Option<Message>,
        _: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        // get  author or early return (no err)
        let author = if let Some(author) = event.author {
            author
        } else {
            return;
        };

        if author.bot {
            return;
        }

        let new_content = if let Some(new) = event.content {
            new
        } else {
            return;
        };

        let data = self.data.clone();
        let guard = EngineGuard::lock(&data, author).await;
        let mut engine = guard.engine().await.write().await;

        // user message
        let messages = if let Some(messages) = engine.find_mut(event.id) {
            messages
        } else {
            log::warn!(
                "No conversation thread found for edited message id: {:?}, is this our fault?",
                event.id
            );
            return;
        };

        // push the new message and select it
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: new_content,
            ..Default::default()
        });
    }
}
