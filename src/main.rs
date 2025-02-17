use std::path::PathBuf;

use chat::ChatMessage;
use config::store::ChatBotConfig;
use log::info;

extern crate proc_macro;

mod archive;
mod bot;
mod chat;
mod config;
mod utils;

#[tokio::main]
async fn main() {
    utils::log::Logger::init(None);
    info!("starting chatbot");

    let config = ChatBotConfig::read(PathBuf::from("config.toml")).unwrap();

    let bot = bot::ChatBot::new(config).await.unwrap();
    bot.run().await;
}
