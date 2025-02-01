use std::collections::HashMap;

use commands::Data;
use serenity::{
    all::{
        Context, CreateButton, CreateMessage, EditMessage, EventHandler, Interaction, Message,
        Ready, User,
    },
    async_trait,
};
use tokio::sync::{
    RwLock,
    broadcast::{Receiver, Sender},
};

use crate::{
    chat::{self, engine::ChatEngine},
    config::store::ChatBotConfig,
};

mod buttons;
pub mod commands;
mod events;

pub struct Handler {
    pub data: Data,
}
impl Handler {
    pub fn new(data: Data) -> Self {
        Self { data }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        ctx.set_presence(None, serenity::all::OnlineStatus::Online);
    }

    // async fn typing_start(&self, ctx: Context, event: TypingStartEvent) {
    //     let is_dm = event.guild_id.is_none();
    //
    //     if is_dm {
    //         // someone is typing in MY dms
    //         println!("User {} is typing in DMs", event.user_id);
    //         println!("Channel ID: {:?}", event.channel_id);
    //
    //         let typing = ctx.http.start_typing(event.channel_id);
    //
    //         tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    //
    //         if let Err(why) = event.channel_id.say(&ctx.http, "hey... you up too?").await {
    //             println!("Error sending message: {why:?}");
    //         }
    //
    //         typing.stop();
    //     }
    // }

    async fn message(&self, ctx: Context, msg: Message) {
        self.on_message(ctx, msg).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction.into_message_component() {
            Some(mut component) => {
                let e = self.disable_buttons(&mut component, &ctx).await;

                if let Err(why) = e {
                    println!("error editing message: {why:?}");
                    return;
                }

                let _ = component.defer(ctx.http.clone()).await;

                let result = match component.data.custom_id.as_str() {
                    "regen" => self.regen(component, ctx).await,
                    "prev" => self.prev(component, ctx).await,
                    "next" => self.next(component, ctx).await,
                    _ => {
                        println!("unknown custom_id: {:?}", component.data.custom_id);
                        Ok(())
                    }
                };

                if let Err(why) = result {
                    println!("error handling interaction: {why:?}");
                    return;
                }
            }
            _ => {}
        }
    }
}
