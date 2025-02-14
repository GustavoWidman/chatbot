use serenity::all::{Context, Message, MessageUpdateEvent};

use crate::chat::{self, ChatMessage};

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
        let (author_id, author) = if let Some(author) = event.author {
            (author.id, author)
        } else {
            return;
        };

        if author.bot {
            return;
        }

        let data = self.data.clone();

        let new_content = if let Some(new) = event.content {
            new.clone()
        } else {
            return;
        };

        let mut user_map = data.user_map.write().await;
        let engine = user_map.entry(author).or_insert_with({
            data.config.write().await.update();
            let config = data.config.read().await.clone();
            || chat::engine::ChatEngine::new(config, author_id)
        });

        // user message
        let message = if let Some(message) = engine.find_mut(event.id) {
            message
        } else {
            return;
        };

        // push the new message and select it
        message.push(ChatMessage {
            role: "user".to_string(),
            content: new_content,
            ..Default::default()
        });
    }
}
