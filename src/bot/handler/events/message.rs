use serenity::all::{Context, CreateButton, CreateMessage, Message};

use crate::chat::engine::{ContextType, EngineGuard};

use super::{super::Handler, error::HandlerResult};

impl Handler {
    pub async fn on_message(&self, ctx: Context, msg: Message) -> HandlerResult<()> {
        if msg.author.bot {
            return HandlerResult::ok(());
        }

        let data = self.data.clone();

        self.freewill_dispatch(msg.author.clone(), msg.channel_id, ctx.http.clone())
            .await;

        let typing = ctx.http.start_typing(msg.channel_id);

        let result: anyhow::Result<Message> = async {
            let guard = EngineGuard::lock(&data, msg.author.clone(), &ctx.http).await?;
            let mut engine = guard.engine().await.write().await;

            let response = engine
                .user_prompt(
                    Some((msg.content.clone(), (msg.id, msg.channel_id).into())),
                    Some(ContextType::User),
                )
                .await?;

            let message = CreateMessage::new()
                // unwrap is safe because user_prompt guarantees a content
                .content(response.content().unwrap())
                .button(
                    CreateButton::new("prev")
                        .label("")
                        .emoji('⏪')
                        .style(serenity::all::ButtonStyle::Secondary)
                        .disabled(true),
                )
                .button(
                    CreateButton::new("regen")
                        .label("")
                        .emoji('♻')
                        .style(serenity::all::ButtonStyle::Secondary),
                )
                .button(
                    CreateButton::new("edit")
                        .label("")
                        .emoji('✏')
                        .style(serenity::all::ButtonStyle::Secondary)
                        .disabled(false),
                );

            let msg = msg
                .channel_id
                .send_message(ctx.http.clone(), message.clone())
                .await?;

            engine.add_message(response, (msg.id, msg.channel_id));

            Ok(msg)
        }
        .await;

        typing.stop();

        match result {
            Ok(_) => HandlerResult::ok(()),
            Err(why) => HandlerResult::err(why, (ctx.http, msg)),
        }
    }
}
