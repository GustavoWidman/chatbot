use std::sync::Arc;

use rand::Rng;
use serenity::all::{ChannelId, EditMessage, Http, MessageId, UserId};
use tokio::{task::JoinHandle, time};

use crate::{
    bot::handler::framework::InnerData,
    chat::engine::{ChatEngine, ContextType, EngineGuard},
    utils::{
        macros::config,
        misc::{self, ButtonStates},
    },
};

use super::super::Handler;

impl Handler {
    pub async fn freewill_dispatch(&self, user: UserId, channel: ChannelId, http: Arc<Http>) {
        let mut freewill_map = self.data.freewill_map.write().await;
        freewill_map
            .entry(user)
            .and_modify(|handle| {
                if handle.is_finished() {
                    log::info!("freewill was finished, dispatching again");
                    *handle = Self::freewill_spawn(
                        self.data.clone(),
                        user,
                        channel.clone(),
                        http.clone(),
                    );
                } else {
                    // freewill is already running
                    log::trace!("freewill is already running");
                }
            })
            .or_insert_with(|| {
                log::info!("freewill is not running, dispatching");

                Self::freewill_spawn(self.data.clone(), user, channel, http)
            });
    }

    pub fn freewill_spawn(
        data: Arc<InnerData>,
        user: UserId,
        channel: ChannelId,
        http: Arc<Http>,
    ) -> JoinHandle<()> {
        tokio::spawn({
            async move {
                loop {
                    let min = 60;
                    let max = 120;
                    let interval = time::Duration::from_secs(rand::random_range(min..max));

                    tokio::time::sleep(interval).await;

                    if Self::should_freewill(data.clone(), user, &http).await {
                        let did_freewill =
                            Self::freewill(data.clone(), user, channel.clone(), http.clone()).await;
                        log::info!("freewill done");
                        if did_freewill {
                            return;
                        } else {
                            log::warn!("freewill failed, will retry later once called again");
                        }
                    };
                }
            }
        })
    }

    pub async fn freewill(
        data: Arc<InnerData>,
        user: UserId,
        channel: ChannelId,
        http: Arc<Http>,
    ) -> bool {
        log::debug!("attempting to freewill");
        let guard = if let Ok(engine) = EngineGuard::lock(&data, user, &http).await {
            engine
        } else {
            return false;
        };

        let mut engine = guard.engine().await.write().await;

        let out: anyhow::Result<MessageId> = async {
            Self::freewill_memory_store(&engine).await?;

            let mut response = engine
                .user_prompt(None, Some(ContextType::Freewill))
                .await?;
            response.freewill = true;

            log::info!("freewill response:\n{:?}", response);

            // todo add chunking here

            let messages = misc::chunk_message(
                &response
                    .content()
                    .ok_or(anyhow::anyhow!("message does not have a content"))?,
                ButtonStates {
                    prev_disabled: true,
                    regen_or_next: misc::RegenOrNext::Regen,
                },
            )?;

            let ids = misc::send_message_batch(channel, &http, messages).await?;
            let last_id = ids.last().ok_or(anyhow::anyhow!("no message ids"))?.clone();

            engine.add_message(response, (last_id, channel, ids));

            Ok(last_id)
        }
        .await;

        match out {
            Ok(msg_id) => {
                let message = http.get_message(channel, msg_id).await;

                if let Ok(mut message) = message {
                    let mut recv = data.msg_channel.0.subscribe();
                    tokio::spawn({
                        async move {
                            let _ = recv.recv().await;

                            let _ = message
                                .edit(&http, EditMessage::new().components(vec![]))
                                .await;

                            drop(recv);
                        }
                    });

                    true
                } else {
                    log::error!("could not fetch discord message");
                    false
                }
            }
            Err(why) => {
                log::error!("Error sending message: {why:?}");
                return false;
            }
        }
    }

    pub async fn should_freewill(data: Arc<InnerData>, user: UserId, http: &Http) -> bool {
        let guard = if let Ok(engine) = EngineGuard::lock(&data, user, http).await {
            engine
        } else {
            return false;
        };

        let engine = guard.engine().await.read().await;

        let time_since_last = engine.time_since_last().num_seconds() as f64;

        let config = config!(data);
        let mut rng = rand::rng();
        let threshold = exponential_probability(
            time_since_last,
            0,
            config.freewill.min_time_secs,
            config.freewill.max_time_secs,
            config.freewill.steepness,
        );

        let bool = rng.random_bool(threshold);

        bool
    }

    // todo: post freewill, index context as a memory to simulate human-like behavior
    pub async fn freewill_memory_store(engine: &ChatEngine) -> anyhow::Result<()> {
        log::info!("performing freewill memory store");

        let messages = engine.take_until_freewill().await;

        engine
            .summarize_and_store(
                messages,
                &engine.config.system.user_name,
                &engine.config.system.chatbot_name,
            )
            .await
    }
}

/// Calculate exponential probability between `z` and `y`
/// - `value`: Input value (must be between `x` and `y`)
/// - `x`: Start of the range (probability = 0)
/// - `z`: Start of the exponential curve (probability = 0)
/// - `y`: End of the range (probability = 1)
/// - `steepness`: Controls how quickly the probability increases
pub fn exponential_probability(value: f64, x: u64, z: u64, y: u64, steepness: f64) -> f64 {
    // Clamp value to [x, y]
    let x = x as f64;
    let y = y as f64;
    let z = z as f64;

    let value = value.clamp(x, y);

    // If value is below `z`, probability is 0
    if value <= z {
        return 0.0;
    }

    // Normalize value to [0, 1] range between `z` and `y`
    let normalized = (value - z) / (y - z);

    // Exponential growth formula
    let prob = (steepness * normalized).exp_m1() / (steepness.exp_m1());

    prob.clamp(0.0, 1.0)
}
