use std::{collections::HashMap, sync::Arc};

use serenity::{
    all::{
        Context, CreateButton, CreateMessage, EditInteractionResponse, EditMessage, EventHandler,
        GetMessages, Interaction, InteractionType, Message, Ready, User,
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

pub struct Handler {
    pub config: ChatBotConfig,
    pub user_map: RwLock<HashMap<User, ChatEngine>>,
    pub msg_channel: (Sender<String>, Receiver<String>),
}
impl Handler {
    pub fn new(config: ChatBotConfig) -> Self {
        Self {
            config,
            user_map: RwLock::new(HashMap::new()),
            msg_channel: tokio::sync::broadcast::channel(100),
        }
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
        if msg.author.bot {
            return;
        } else {
            self.msg_channel.0.send(msg.content.clone()).unwrap();
        }

        let latency = chrono::Utc::now().timestamp_millis() - msg.timestamp.timestamp_millis();

        let typing = ctx.http.start_typing(msg.channel_id);

        let mut user_map = self.user_map.write().await;
        let engine = user_map
            .entry(msg.author.clone())
            .or_insert_with(|| chat::engine::ChatEngine::new(&self.config.prompt));

        let m = match engine
            .user_prompt(msg.content.clone(), engine.get_context())
            .await
        {
            Ok(response) => {
                engine.add_user_message(msg.content, msg.id);

                let m = msg
                    .channel_id
                    .send_message(
                        &ctx,
                        CreateMessage::new()
                            .content(response.content.clone())
                            .button(
                                CreateButton::new("previous")
                                    .label("")
                                    .emoji('⏪')
                                    .style(serenity::all::ButtonStyle::Secondary)
                                    .disabled(true),
                            )
                            .button(
                                CreateButton::new("regen")
                                    .label("")
                                    .emoji('⏩')
                                    .style(serenity::all::ButtonStyle::Secondary),
                            ),
                    )
                    .await;

                match m {
                    Ok(message) => {
                        engine.add_message(response, msg.id);
                        Ok(message)
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
                    .say(&ctx.http, "error generating response")
                    .await
            }
        };

        tokio::spawn({
            let mut recv = self.msg_channel.0.subscribe();

            async move {
                if let Ok(mut m) = m {
                    // wait for msg to be sent
                    let _ = recv.recv().await;
                    println!("new message received");
                    let _ = m
                        .edit(&ctx.http, EditMessage::new().components(vec![]))
                        .await;
                }
            }
        });

        typing.stop();
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction.into_message_component() {
            Some(mut component) => {
                let _ = component.defer(&ctx.http).await;
                let _ = component
                    .message
                    .edit(
                        &ctx.http,
                        EditMessage::new()
                            .button(
                                CreateButton::new("previous")
                                    .label("")
                                    .emoji('⏪')
                                    .style(serenity::all::ButtonStyle::Secondary)
                                    .disabled(true),
                            )
                            .button(
                                CreateButton::new("regen")
                                    .label("")
                                    .emoji('⏩')
                                    .style(serenity::all::ButtonStyle::Secondary)
                                    .disabled(true),
                            ),
                    )
                    .await;

                match component.data.custom_id.as_str() {
                    "regen" => {
                        let mut user_map = self.user_map.write().await;
                        let engine = user_map
                            .entry(component.user.clone())
                            .or_insert_with(|| chat::engine::ChatEngine::new(&self.config.prompt));

                        let (prompt, regen_context) = engine.get_regen_context();

                        let m = match engine.user_prompt(prompt, regen_context).await {
                            Ok(response) => {
                                let m = component
                                    .message
                                    .edit(
                                        &ctx.http,
                                        EditMessage::new()
                                            .content(response.content.clone())
                                            .button(
                                                CreateButton::new("previous")
                                                    .label("")
                                                    .emoji('⏪')
                                                    .style(serenity::all::ButtonStyle::Secondary)
                                                    .disabled(false),
                                            )
                                            .button(
                                                CreateButton::new("regen")
                                                    .label("")
                                                    .emoji('⏩')
                                                    .style(serenity::all::ButtonStyle::Secondary)
                                                    .disabled(false),
                                            ),
                                    )
                                    .await;

                                match m {
                                    Ok(message) => {
                                        engine.regenerate(response);
                                        Ok(message)
                                    }
                                    Err(why) => {
                                        println!("Error sending message: {why:?}");
                                        Err(why)
                                    }
                                }
                            }
                            Err(why) => {
                                println!("Error generating response: {why:?}");
                                component
                                    .message
                                    .edit(
                                        &ctx.http,
                                        EditMessage::new().content("error generating response"),
                                    )
                                    .await
                            }
                        };

                        if let Err(why) = m {
                            println!("Error editing message: {why:?}");
                        }
                    }
                    "previous" => {
                        let mut user_map = self.user_map.write().await;
                        let engine = user_map
                            .entry(component.user.clone())
                            .or_insert_with(|| chat::engine::ChatEngine::new(&self.config.prompt));

                        let (message, can_go_back) = engine.go_back().unwrap();

                        let m = component
                            .message
                            .edit(
                                &ctx.http,
                                EditMessage::new()
                                    .content(message.content.clone())
                                    .button(
                                        CreateButton::new("previous")
                                            .label("")
                                            .emoji('⏪')
                                            .style(serenity::all::ButtonStyle::Secondary)
                                            .disabled(!can_go_back),
                                    )
                                    .button(
                                        CreateButton::new("next")
                                            .label("")
                                            .emoji('⏩')
                                            .style(serenity::all::ButtonStyle::Secondary)
                                            .disabled(false),
                                    ),
                            )
                            .await;

                        if let Err(why) = m {
                            println!("Error editing message: {why:?}");
                        }
                    }
                    "next" => {
                        let mut user_map = self.user_map.write().await;
                        let engine = user_map
                            .entry(component.user.clone())
                            .or_insert_with(|| chat::engine::ChatEngine::new(&self.config.prompt));

                        let (message, can_go_fwd) = engine.go_fwd().unwrap();

                        let can_go_fwd = match can_go_fwd {
                            true => "next",
                            false => "regen",
                        };

                        let m = component
                            .message
                            .edit(
                                &ctx.http,
                                EditMessage::new()
                                    .content(message.content.clone())
                                    .button(
                                        CreateButton::new("previous")
                                            .label("")
                                            .emoji('⏪')
                                            .style(serenity::all::ButtonStyle::Secondary)
                                            .disabled(false),
                                    )
                                    .button(
                                        // regen if cant go fwd, else next
                                        CreateButton::new(can_go_fwd)
                                            .label("")
                                            .emoji('⏩')
                                            .style(serenity::all::ButtonStyle::Secondary)
                                            .disabled(false),
                                    ),
                            )
                            .await;

                        if let Err(why) = m {
                            println!("Error editing message: {why:?}");
                        }
                    }
                    _ => {}
                };
            }
            _ => {}
        }
    }
}
