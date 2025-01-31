use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use handler::Handler;
use serenity::{
    Client,
    all::{EventHandler, GatewayIntents, User},
    prelude::TypeMapKey,
};
use tokio::sync::RwLock;

use crate::{chat::engine::ChatEngine, config::store::ChatBotConfig};

mod handler;

pub struct ChatBot {
    client: Client,
}
impl ChatBot {
    pub async fn new(config: ChatBotConfig) -> Result<Self> {
        let intents = GatewayIntents::all();

        let client = Client::builder(&config.discord.token, intents)
            .event_handler(Handler::new(config))
            .await?;

        Ok(Self { client })
    }

    pub async fn run(self) {
        let ChatBot { mut client } = self;

        if let Err(why) = client.start().await {
            println!("Client error: {why:?}");
        }
    }
}
