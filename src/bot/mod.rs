use anyhow::Result;
use handler::Handler;
use serenity::{Client, all::GatewayIntents};

use crate::config::store::ChatBotConfig;
pub use handler::Data;

mod handler;

pub struct ChatBot {
    client: Client,
}
impl ChatBot {
    pub async fn new(config: ChatBotConfig) -> Result<Self> {
        let builder = serenity::Client::builder(&config.discord.token, GatewayIntents::all());

        let (framework, data) = handler::framework::framework(config).await;

        let client = builder
            .event_handler(Handler::new(data))
            .framework(framework)
            .await?;

        Ok(Self { client })
    }

    pub async fn run(self) {
        let ChatBot { mut client } = self;

        if let Err(why) = client.start().await {
            log::error!("Client error: {why:?}");
        }
    }
}
