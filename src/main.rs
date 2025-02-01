use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chat::engine::ChatEngine;
use config::store::ChatBotConfig;
use serenity::all::{CreateButton, CreateMessage, CurrentUser, Ready, User};
use serenity::model::channel::Message;
use serenity::prelude::*;
use serenity::{Result, async_trait};

mod bot;
mod chat;
mod config;

#[tokio::main]
async fn main() {
    let config = ChatBotConfig::read(PathBuf::from("config.toml")).unwrap();

    let bot = bot::ChatBot::new(config).await.unwrap();
    bot.run().await;
}
