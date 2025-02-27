use anyhow::anyhow;
use rig::message::Message as RigMessage;
use serenity::all::{Context, Message, MessageUpdateEvent};

use crate::chat::{ChatMessage, engine::EngineGuard};

use super::{super::Handler, error::HandlerResult};

impl Handler {
    pub async fn on_edit(
        &self,
        ctx: Context,
        _: Option<Message>,
        _: Option<Message>,
        event: MessageUpdateEvent,
    ) -> HandlerResult<()> {
        let author = if let Some(author) = event.author {
            author
        } else {
            return HandlerResult::ok(());
        };

        if author.bot {
            return HandlerResult::ok(());
        }

        let new_content = if let Some(new) = event.content {
            new
        } else {
            return HandlerResult::ok(());
        };

        let data = self.data.clone();
        let guard = match EngineGuard::lock(&data, author, &ctx.http).await {
            Ok(guard) => guard,
            Err(why) => {
                return HandlerResult::err(
                    why,
                    (
                        ctx.http,
                        event.channel_id,
                        event.message_reference.flatten(),
                    ),
                );
            }
        };

        let mut engine = guard.engine().await.write().await;

        // user message
        let messages = match engine.find_mut(&(event.id, event.channel_id).into()) {
            Some(messages) => messages,
            None => {
                log::warn!(
                    "No conversation thread found for edited message id: {:?}, is this our fault?",
                    event.id
                );
                return HandlerResult::err(
                    anyhow!("message not found in engine"),
                    (
                        ctx.http,
                        event.channel_id,
                        event.message_reference.flatten(),
                    ),
                );
            }
        };

        // push the new message and select it
        messages.push(ChatMessage {
            inner: RigMessage::user(new_content),
            ..Default::default()
        });

        HandlerResult::ok(())
    }
}
