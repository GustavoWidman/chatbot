use serenity::all::{Context, CreateButton, CreateMessage, Message};

use crate::chat::engine::{ContextType, EngineGuard};

use super::super::Handler;

impl Handler {
    pub async fn on_message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let data = self.data.clone();

        self.freewill_dispatch(msg.author.clone(), msg.channel_id, ctx.http.clone())
            .await;

        let typing = ctx.http.start_typing(msg.channel_id);

        let guard = EngineGuard::lock(&data, msg.author).await;
        let mut engine = guard.engine().await.write().await;

        let _ = match engine
            .user_prompt(Some(msg.content.clone()), Some(ContextType::User))
            .await
        {
            Ok(response) => {
                engine.add_user_message(msg.content, msg.id);

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
                    );

                match msg
                    .channel_id
                    .send_message(ctx.http.clone(), message.clone())
                    .await
                {
                    Ok(msg) => {
                        engine.add_message(response, Some(msg.id));
                        Ok(msg)
                    }
                    Err(why) => {
                        log::error!("Error sending message: {why:?}");
                        Err(why)
                    }
                }
            }
            Err(why) => {
                log::error!("Error generating response: {why:?}");
                msg.channel_id
                    .say(ctx.http.clone(), "error generating response")
                    .await
            }
        };

        typing.stop();
    }
}
