use anyhow::Result;
use handler::Handler;
use serenity::{Client, all::GatewayIntents};
use tokio::task::JoinHandle;

use crate::config::store::ChatBotConfig;
pub use handler::Data;

mod handler;

pub struct ChatBot {
    client: Client,
    handle: JoinHandle<()>,
}

impl ChatBot {
    pub async fn new(config: ChatBotConfig) -> Result<Self> {
        let builder = serenity::Client::builder(&config.discord.token, GatewayIntents::all());

        let (framework, data) = handler::framework::framework(config).await;
        let (handler, handle) = Handler::new(data);

        let client = builder
            .event_handler_arc(handler)
            .framework(framework)
            .await?;

        Ok(Self { client, handle })
    }

    pub async fn run(self) {
        let ChatBot { mut client, handle } = self;

        client.shard_manager.shutdown_all().await;

        if let Err(why) = client.start().await {
            log::error!("Client error: {why:?}");
        }

        handle.await.unwrap();
    }
}
