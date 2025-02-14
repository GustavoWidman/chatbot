use std::sync::Arc;

use rand::Rng;
use serenity::all::{ChannelId, CreateButton, CreateMessage, EditMessage, Http, Message, User};
use tokio::{task::JoinHandle, time};

use crate::{
    bot::handler::commands::InnerData,
    chat::{self, engine::ContextType},
};

use super::super::Handler;

impl Handler {
    pub async fn freewill_dispatch(&self, user: User, channel: ChannelId, http: Arc<Http>) {
        let data = self.data.clone();
        let mut freewill_map = data.freewill_map.write().await;
        freewill_map
            .entry(user.clone())
            .and_modify(|handle| {
                if handle.is_finished() {
                    println!("freewill was finished, dispatching again");
                    *handle = Self::freewill_spawn(
                        data.clone(),
                        user.clone(),
                        channel.clone(),
                        http.clone(),
                    );
                } else {
                    // freewill is already running
                    println!("freewill is already running");
                }
            })
            .or_insert_with(|| {
                println!("freewill is not running, dispatching");

                let data = self.data.clone();

                Self::freewill_spawn(data, user.clone(), channel.clone(), http.clone())
            });
    }

    pub fn freewill_spawn(
        data: Arc<InnerData>,
        user: User,
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

                    if Self::should_freewill(data.clone(), user.clone()).await {
                        let did_freewill = Self::freewill(
                            data.clone(),
                            user.clone(),
                            channel.clone(),
                            http.clone(),
                        )
                        .await;
                        println!("freewill done");
                        if did_freewill {
                            return;
                        } else {
                            println!("freewill failed");
                        }
                    };
                }
            }
        })
    }

    pub async fn freewill(
        data: Arc<InnerData>,
        user: User,
        channel: ChannelId,
        http: Arc<Http>,
    ) -> bool {
        println!("freewilling");
        let user_id = user.id;

        let mut user_map = data.user_map.write().await;
        let engine = user_map.entry(user).or_insert_with({
            data.config.write().await.update();
            let config = data.config.read().await.clone();
            || chat::engine::ChatEngine::new(config, user_id)
        });

        let out: anyhow::Result<Message> = async {
            let response = engine
                .user_prompt(None, Some(ContextType::Freewill))
                .await?;

            println!("{:?}", response);

            let message = CreateMessage::new()
                .content(response.content.clone())
                .button(
                    CreateButton::new("prev")
                        .label("")
                        .emoji('⏪')
                        .style(serenity::all::ButtonStyle::Secondary)
                        .disabled(true),
                )
                .button(
                    CreateButton::new("regen")
                        .label("")
                        .emoji('♻')
                        .style(serenity::all::ButtonStyle::Secondary)
                        .disabled(true), // todo fix freewill regen context (keep the old sys prompt maybe)
                );

            let msg = channel.send_message(http.clone(), message.clone()).await?;

            // only change context after we're sure everything is okay
            engine.add_message(response, Some(msg.id));

            Ok(msg)
        }
        .await;

        match out {
            Ok(_) => true,
            Err(why) => {
                println!("Error sending message: {why:?}");
                return false;
            }
        }
    }

    pub async fn should_freewill(data: Arc<InnerData>, user: User) -> bool {
        let user_id = user.id;
        let mut user_map = data.user_map.write().await;
        let engine = user_map.entry(user).or_insert_with({
            data.config.write().await.update();
            let config = data.config.read().await.clone();
            || chat::engine::ChatEngine::new(config, user_id)
        });

        let time_since_last = engine
            .time_since_last()
            .map(|t| t.num_seconds() as f64)
            .unwrap_or(0.0);

        data.config.write().await.update();
        let config = data.config.read().await;
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
