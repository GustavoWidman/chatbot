use std::path::PathBuf;

use config::store::ChatBotConfig;
use log::info;

extern crate proc_macro;

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

// #[tokio::main]
// async fn main() {
//     utils::log::Logger::init(None);
//     info!("starting chatbot");
//
//     let config = ChatBotConfig::read(PathBuf::from("config.toml")).unwrap();
//
//     let engine = chat::engine::ChatEngine::new(
//         config.clone(),
//         1120638385124556870.into(),
//         &serenity::all::Http::new(&config.discord.token),
//     )
//     .await
//     .unwrap();
//
//     let user_name = engine.config.system.user_name.clone();
//     let assistant_name = engine.config.system.chatbot_name.clone();
//
//     let messages = engine.take_until_freewill().await;
//
//     engine
//         .summarize_and_store(messages, user_name, assistant_name)
//         .await
//         .unwrap()
// }
