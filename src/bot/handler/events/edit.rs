use anyhow::anyhow;
use serenity::all::{Context, Message, MessageUpdateEvent};

use crate::{
    chat::{ChatMessage, context::UserPrompt, engine::EngineGuard},
    utils,
};

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

        let guard = match EngineGuard::lock(&self.data, author.id, &ctx.http).await {
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

        let user_prompt = match async {
            let mut user_prompt = UserPrompt {
                content: Some(new_content),
                current_time: engine.config.system.get_time(),
                relevant_memories: vec![],
                time_since: utils::time_to_string(engine.time_since_last()),
                system_note: None,
            };
            engine.client.rag_recall(&mut user_prompt).await?;

            Ok::<UserPrompt, anyhow::Error>(user_prompt)
        }
        .await
        {
            Ok(user_prompt) => user_prompt,
            Err(why) => {
                log::error!("failed to rag recall: {why:?}");
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
        // messages.push(ChatMessage::user(new_content));
        if let Err(why) = async {
            messages.push(TryInto::<ChatMessage>::try_into(user_prompt)?);

            Ok::<(), anyhow::Error>(())
        }
        .await
        {
            log::error!("failed to push message: {why:?}");

            return HandlerResult::err(
                why,
                (
                    ctx.http,
                    event.channel_id,
                    event.message_reference.flatten(),
                ),
            );
        }

        HandlerResult::ok(())
    }
}
