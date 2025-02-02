use std::vec;

use serenity::all::{Context, CreateButton, CreateMessage, EditMessage, Message};

use crate::chat;

use super::super::Handler;

impl Handler {
    pub async fn on_message(&self, ctx: Context, msg: Message) {
        let data = self.data.clone();

        if msg.author.bot {
            return;
        } else {
            data.msg_channel.0.send(msg.content.clone()).unwrap();
        }

        self.freewill_dispatch(msg.author.clone(), msg.channel_id, ctx.http.clone())
            .await;

        let typing = ctx.http.start_typing(msg.channel_id);

        let mut user_map = data.user_map.write().await;
        let engine = user_map.entry(msg.author.clone()).or_insert_with({
            data.config.write().await.update();
            let config = data.config.read().await.clone();
            || chat::engine::ChatEngine::new(config, msg.author.id)
        });

        let context = engine.get_context().await;
        let m = match engine.user_prompt(Some(msg.content.clone()), context).await {
            Ok(response) => {
                engine.add_user_message(msg.content);

                let message = CreateMessage::new()
                    .content(response.content.clone())
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

                let msg = msg
                    .channel_id
                    .send_message(ctx.http.clone(), message.clone())
                    .await;

                match msg {
                    Ok(msg) => {
                        engine.add_message(response);
                        Ok(msg)
                    }
                    Err(why) => {
                        println!("Error sending message: {why:?}");
                        Err(why)
                    }
                }
            }
            Err(why) => {
                println!("Error generating response: {why:?}");
                msg.channel_id
                    .say(ctx.http.clone(), "error generating response")
                    .await
            }
        };

        typing.stop();

        tokio::spawn({
            let mut recv = data.msg_channel.0.subscribe();

            async move {
                if let Ok(mut m) = m {
                    // wait for msg to be sent
                    let _ = recv.recv().await;
                    let _ = m
                        .edit(ctx.http.clone(), EditMessage::new().components(vec![]))
                        .await;
                }
            }
        });
    }
}
