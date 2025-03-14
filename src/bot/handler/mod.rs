use std::{
    process::exit,
    sync::{Arc, mpsc},
    time::Duration,
};

use events::HandlerResult;
pub use framework::Data;
use serenity::{
    all::{Context, EventHandler, Interaction, Message, MessageUpdateEvent, Ready, UserId},
    async_trait,
};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::{chat::engine::ChatEngine, utils::macros::config};

mod buttons;
mod events;
pub mod framework;

pub struct Handler {
    pub data: Data,
}
impl Handler {
    pub fn new(data: Data) -> (Arc<Self>, JoinHandle<()>) {
        let handler = Arc::new(Self { data });

        let handle = tokio::spawn({
            let handler = handler.clone();
            let shutdown_rx = setup_ctrlc_handler();
            async move {
                shutdown_rx
                    .recv()
                    .expect("Failed to receive shutdown signal");

                if let Err(err) = handler.shutdown().await {
                    log::error!("Error shutting down: {err}");
                }

                exit(0);
            }
        });

        (handler, handle)
    }
}

fn setup_ctrlc_handler() -> mpsc::Receiver<()> {
    let (sender, receiver) = mpsc::channel();

    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let mut term_signal =
                signal(SignalKind::terminate()).expect("Failed to set SIGTERM handler");
            let mut int_signal =
                signal(SignalKind::interrupt()).expect("Failed to set SIGINT handler");
            let mut hup_signal =
                signal(SignalKind::hangup()).expect("Failed to set SIGHUP handler");

            tokio::select! {
                _ = term_signal.recv() => {
                    log::info!("SIGTERM received, shutting down...");
                    let _ = sender.send(());
                },
                _ = int_signal.recv() => {
                    println!("");
                    log::info!("SIGINT received, shutting down...");
                    let _ = sender.send(());
                },
                _ = hup_signal.recv() => {
                    log::info!("SIGHUP received, shutting down...");
                    let _ = sender.send(());
                },
            };
        }

        #[cfg(windows)]
        {
            use tokio::signal::ctrl_c;

            if let Err(e) = ctrl_c().await {
                log::error!("Failed to set up CTRL+C handler: {e}");
            } else {
                log::info!("CTRL+C received, shutting down...");
                let _ = sender.send(());
            }
        }
    });

    receiver
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        log::info!("{} is connected!", ready.user.name);

        ctx.set_presence(None, serenity::all::OnlineStatus::Online);

        // TODO mobile status
        // ctx.shard
        //     .send_to_shard(serenity::all::ShardRunnerMessage::Message(
        //         tungstenite::Message::Text(
        //             serde_json::to_string(&serde_json::json!({
        //                 "op": 3,  // Status Update opcode
        //                 "d": {
        //                     "since": null,
        //                     "activities": [],
        //                     "status": "online",
        //                     "afk": false,
        //                     "client_info": {
        //                         "$os": "android",  // Try setting mobile OS flag
        //                         "$browser": "Discord Android"
        //                     }
        //                 }
        //             }))
        //             .unwrap(),
        //         ),
        //     ));

        // list contents of directory of saves
        let config = config!(self.data);

        let result: anyhow::Result<()> = async {
            if let Some(path) = &config.context.save_to_disk_folder {
                // walk the directory for files matching the pattern "context-*.bin" and extract the
                // id from the filename
                let files = std::fs::read_dir(path)?;
                let regex = regex::Regex::new(r"^context-(\d+)\.bin$")?;
                for file in files {
                    let file = file?;
                    let filename = file.file_name();
                    if let Some(captures) = regex.captures(&filename.to_string_lossy()) {
                        let id = captures
                            .get(1)
                            .ok_or(anyhow::anyhow!("no id found"))?
                            .as_str()
                            .parse::<u64>()?;
                        log::info!("found saved context with id {id}");

                        let mut user_map = self.data.user_map.write().await;
                        let user = UserId::new(id);
                        let engine = ChatEngine::new(config.clone(), user.clone()).await?;

                        user_map.insert(user, RwLock::new(engine));
                    }
                }
            }

            Ok(())
        }
        .await;

        if let Err(why) = result {
            log::error!("failed to load saved contexts: {why:?}");
        }

        self.data.context.write().await.replace(Arc::new(ctx));
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if let HandlerResult::Err(error) = self.on_message(ctx, msg).await {
            Self::on_error(error).await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let HandlerResult::Err(error) = self.on_interaction(ctx, interaction).await {
            Self::on_error(error).await;
        }
    }

    async fn message_update(
        &self,
        ctx: Context,
        old_if_available: Option<Message>,
        new: Option<Message>,
        event: MessageUpdateEvent,
    ) {
        if let HandlerResult::Err(error) = self.on_edit(ctx, old_if_available, new, event).await {
            Self::on_error(error).await;
        }
    }
}

impl Handler {
    async fn shutdown(&self) -> anyhow::Result<()> {
        log::info!("Shutdown signal received, waiting for locks and shutting down...");
        let user_map = self.data.user_map.write().await;
        let context = self.data.context.write().await;

        self.data.msg_channel.0.send("shutdown".to_string())?;

        // loop for max of 5 seconds until receiver count is 0 (50*100ms = 5000ms)
        for _ in 0..50 {
            let count = self.data.msg_channel.0.receiver_count();

            log::trace!("receiver count: {count}");

            if count == 1 {
                break;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        for (_, engine) in user_map.iter() {
            engine.write().await.shutdown().await?
        }

        if let Some(context) = context.as_ref() {
            context.set_presence(None, serenity::all::OnlineStatus::Offline);

            context.shard.shutdown_clean();
        }

        log::info!("Graceful shutdown complete!");

        Ok(())
    }
}
