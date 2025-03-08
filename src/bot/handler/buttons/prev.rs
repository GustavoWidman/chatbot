use anyhow::bail;
use serenity::all::{ComponentInteraction, Context, EditMessage};

use crate::{
    chat::engine::EngineGuard,
    utils::misc::{self, ButtonStates},
};

use super::super::Handler;

impl Handler {
    pub async fn prev(&self, component: ComponentInteraction, ctx: Context) -> anyhow::Result<()> {
        let guard = EngineGuard::lock(&self.data, component.user.id, &ctx.http).await?;
        let mut engine = guard.engine().await.write().await;

        let (_, identifier, message) = engine
            .find_full_mut(&(component.message.id, component.message.channel_id).into())
            .ok_or(anyhow::anyhow!("message not found in engine"))?;

        if !message.backward {
            bail!("message is already at the end of the context");
        }

        let backward = message.backward();

        let channel = identifier.channel();
        let messages = identifier.messages();
        let content = backward.content();
        let button_states = ButtonStates {
            prev_disabled: !message.backward,
            regen_or_next: misc::RegenOrNext::Next,
        };

        let typing = ctx.http.start_typing(channel);

        let result: anyhow::Result<()> = async {
            let content = content.ok_or(anyhow::anyhow!("Message does not have a content"))?;

            misc::delete_message_batch(channel, &ctx.http, messages).await?;

            let messages = misc::chunk_message(&content, button_states)?;

            let ids = misc::send_message_batch(channel, &ctx.http, messages).await?;
            let last_id = ids.last().ok_or(anyhow::anyhow!("no message ids"))?.clone();

            let mut message = ctx.http.get_message(channel, last_id).await?;

            engine.swap_identifiers(
                &(component.message.id, component.message.channel_id).into(),
                (last_id, channel, ids),
            )?;

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
        }
        .await;

        typing.stop();

        result
    }
}
