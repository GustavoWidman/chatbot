use std::{collections::HashMap, sync::Arc};

use serenity::all::{Framework, User};

use tokio::{
    sync::{
        RwLock,
        broadcast::{Receiver, Sender},
    },
    task::JoinHandle,
};

use crate::{chat::engine::ChatEngine, config::store::ChatBotConfig};

mod clear;

pub struct InnerData {
    pub config: RwLock<ChatBotConfig>,
    pub user_map: RwLock<HashMap<User, ChatEngine>>,
    pub msg_channel: (Sender<String>, Receiver<String>),
    pub freewill_map: RwLock<HashMap<User, JoinHandle<()>>>,
}
pub type Data = Arc<InnerData>;

pub async fn framework(config: ChatBotConfig) -> (impl Framework + 'static, Data) {
    let data = Arc::new(InnerData {
        config: RwLock::new(config),
        user_map: RwLock::new(HashMap::new()),
        msg_channel: tokio::sync::broadcast::channel(100),
        freewill_map: RwLock::new(HashMap::new()),
    });
    let clone = data.clone();

    (
        poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: vec![clear::clear()],
                ..Default::default()
            })
            .setup(move |ctx, _ready, framework| {
                Box::pin({
                    async move {
                        poise::builtins::register_globally(ctx, &framework.options().commands)
                            .await?;
                        Ok(clone)
                    }
                })
            })
            .build(),
        data,
    )
}
