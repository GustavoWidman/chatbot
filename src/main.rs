use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use archive::retrieval;
use chat::engine::ChatEngine;
use config::store::ChatBotConfig;
use genai::chat::ChatMessage;
use serenity::all::{CreateButton, CreateMessage, CurrentUser, Ready, User};
use serenity::model::channel::Message;
use serenity::prelude::*;
use serenity::{Result, async_trait};

mod archive;
mod bot;
mod chat;
mod config;

// #[tokio::main]
// async fn main() {
//     env_logger::init();

//     let config = ChatBotConfig::read(PathBuf::from("config.toml")).unwrap();

//     let bot = bot::ChatBot::new(config).await.unwrap();
//     bot.run().await;
// }

// #[tokio::main]
// async fn main() {
//     // memory archival test
//     let mut storage = archive::MemoryStorage::new();
//     storage.add_memory("hello world").unwrap();
//     storage.add_memory("hello again").unwrap();
//     storage.add_memory("hello again").unwrap();

//     println!(
//         "search results: {:?}",
//         storage.search("again", 0.0).unwrap()
//     );
// }

#[tokio::main]
async fn main() {
    let config = ChatBotConfig::read(PathBuf::from("config.toml")).unwrap();

    let bot = retrieval::RetrievalClient::new(&config.retrieval)
        .await
        .unwrap();

    // bot.recall(ChatMessage::user(
    //     "hey... do you remember what i told you about my shirt?",
    // ))
    // .await;
}
