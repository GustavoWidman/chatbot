use std::{collections::HashMap, sync::Arc};

use serenity::all::{Framework, UserId};

use tokio::{sync::RwLock, task::JoinHandle};

use crate::{chat::engine::ChatEngine, config::store::ChatBotConfig};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

mod clear;
mod config;
mod reload;

pub struct InnerData {
    pub config: RwLock<ChatBotConfig>,
    pub user_map: RwLock<HashMap<UserId, RwLock<ChatEngine>>>,
    pub freewill_map: RwLock<HashMap<UserId, JoinHandle<()>>>,
    pub context: RwLock<Option<Arc<serenity::client::Context>>>,
}
pub type Data = Arc<InnerData>;

pub async fn framework(config: ChatBotConfig) -> (impl Framework + 'static, Data) {
    let data = Arc::new(InnerData {
        config: RwLock::new(config),
        user_map: RwLock::new(HashMap::new()),
        freewill_map: RwLock::new(HashMap::new()),
        context: RwLock::new(None),
    });

    (
        poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: vec![clear::clear(), reload::reload(), config::config()],
                ..Default::default()
            })
            .setup({
                let data = data.clone();
                move |ctx, _ready, framework| {
                    Box::pin({
                        async move {
                            poise::builtins::register_globally(ctx, &framework.options().commands)
                                .await?;
                            Ok(data)
                        }
                    })
                }
            })
            .build(),
        data,
    )
}
