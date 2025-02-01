use std::collections::hash_map::Entry;

use poise::serenity_prelude as serenity;

use crate::{chat, config};

use super::Data;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Clears the current context window
#[poise::command(slash_command, prefix_command)]
pub(super) async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data().clone();

    let mut user_map = data.user_map.write().await;
    user_map
        .entry(ctx.author().clone())
        .and_modify({
            let mut config = data.config.read().await.clone();
            config.update();
            |engine| {
                *engine = chat::engine::ChatEngine::new(config);
            }
        })
        .or_insert_with({
            let mut config = data.config.read().await.clone();
            config.update();
            || chat::engine::ChatEngine::new(config)
        });

    ctx.say("wacked!").await?;

    Ok(())
}
