use serenity::all::{ComponentInteraction, Context, EditMessage};

use crate::{
    chat::{
        ChatMessage,
        context::MessageIdentifier,
        engine::{ContextType, EngineGuard},
    },
    utils::misc::{self, ButtonStates},
};

use super::super::Handler;

impl Handler {
    pub async fn regen(&self, component: ComponentInteraction, ctx: Context) -> anyhow::Result<()> {
        let guard = EngineGuard::lock(&self.data, component.user.id, &ctx.http).await?;
        let mut engine = guard.engine().await.write().await;

        // uses this to find the error before other things
        let (_, identifier, _) = engine
            .find_full_mut(&(component.message.id, component.message.channel_id).into())
            .ok_or(anyhow::anyhow!("Message not found in engine"))?;
        let channel = identifier.channel();
        let messages = identifier.messages();

        let typing = ctx.http.start_typing(channel);

        let out: anyhow::Result<(ChatMessage, MessageIdentifier)> = async {
            let response = engine
                .user_prompt(
                    None,
                    Some(ContextType::Regen(
                        (component.message.id, component.message.channel_id).into(),
                    )),
                )
                .await?;

            let content = response
                .content()
                .ok_or(anyhow::anyhow!("Message does not have a content"))?;

            misc::delete_message_batch(channel, &ctx.http, messages).await?;

            let messages = misc::chunk_message(
                &content,
                ButtonStates {
                    prev_disabled: false,
                    regen_or_next: misc::RegenOrNext::Regen,
                },
            )?;

            let ids = misc::send_message_batch(channel, &ctx.http, messages).await?;
            let last_id = ids.last().ok_or(anyhow::anyhow!("no message ids"))?.clone();

            Ok((response, (last_id, channel, ids).into()))
        }
        .await;

        typing.stop();

        match out {
            Ok((message, new_identifier)) => {
                let identifier = (component.message.id, component.message.channel_id).into();
                let messages = engine
                    .find_mut(&identifier)
                    .ok_or(anyhow::anyhow!("message not found in engine"))?;

                messages.push(message); // pushes and selects

                let message = ctx
                    .http
                    .get_message(new_identifier.channel(), new_identifier.message())
                    .await;

                engine.swap_identifiers(&identifier, new_identifier)?;

                if let Ok(mut message) = message {
                    tokio::spawn({
                        let mut recv = self.data.msg_channel.0.subscribe();
                        async move {
                            let _ = recv.recv().await;

                            let _ = message
                                .edit(&ctx.http, EditMessage::new().components(vec![]))
                                .await;

                            drop(recv);
                        }
                    });

                    Ok(())
                } else {
                    Err(anyhow::anyhow!("could not fetch discord message"))
                }
            }
            Err(why) => Err(why),
        }
    }
}
