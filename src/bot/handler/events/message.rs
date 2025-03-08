use serenity::all::{ChannelId, Context, EditMessage, Message, MessageId};

use crate::{
    chat::engine::{ContextType, EngineGuard},
    utils::misc::ButtonStates,
};

use super::{super::Handler, error::HandlerResult};
use crate::utils::misc;

impl Handler {
    pub async fn on_message(&self, ctx: Context, msg: Message) -> HandlerResult<()> {
        if msg.author.bot {
            return HandlerResult::ok(());
        } else {
            self.data.msg_channel.0.send(msg.content.clone()).unwrap();
        }

        self.freewill_dispatch(msg.author.id, msg.channel_id, ctx.http.clone())
            .await;

        let typing = ctx.http.start_typing(msg.channel_id);

        let result: anyhow::Result<(MessageId, ChannelId)> = async {
            let guard = EngineGuard::lock(&self.data, msg.author.id, &ctx.http).await?;
            let mut engine = guard.engine().await.write().await;

            let response = engine
                .user_prompt(
                    Some((msg.content.clone(), (msg.id, msg.channel_id).into())),
                    Some(ContextType::User),
                )
                .await?;

            let messages = misc::chunk_message(
                &response
                    .content()
                    .ok_or(anyhow::anyhow!("message does not have a content"))?,
                ButtonStates {
                    prev_disabled: true,
                    regen_or_next: misc::RegenOrNext::Regen,
                },
            )?;

            let ids = misc::send_message_batch(msg.channel_id, &ctx.http, messages).await?;
            let last_id = ids.last().ok_or(anyhow::anyhow!("no message ids"))?.clone();

            engine.add_message(response, (last_id, msg.channel_id, ids));

            Ok((last_id, msg.channel_id))
        }
        .await;

        typing.stop();

        match result {
            Ok((msg_id, chan_id)) => {
                let message = ctx.http.get_message(chan_id, msg_id).await;

                if let Ok(mut message) = message {
                    let mut recv = self.data.msg_channel.0.subscribe();
                    tokio::spawn({
                        async move {
                            let _ = recv.recv().await;

                            let _ = message
                                .edit(&ctx.http, EditMessage::new().components(vec![]))
                                .await;

                            drop(recv);
                        }
                    });

                    HandlerResult::ok(())
                } else {
                    HandlerResult::err(
                        anyhow::anyhow!("could not fetch discord message"),
                        (ctx.http, msg),
                    )
                }
            }
            Err(why) => HandlerResult::err(why, (ctx.http, msg)),
        }
    }
}
