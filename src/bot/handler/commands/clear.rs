use poise::CreateReply;

use super::{Context, Error};
use crate::chat;

/// Clears the current context window and reloads the engine
#[poise::command(slash_command, prefix_command)]
pub(super) async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data().clone();

    let mut user_map = data.user_map.write().await;
    user_map
        .entry(ctx.author().clone())
        .and_modify({
            data.config.write().await.update();
            let config = data.config.read().await.clone();
            |engine| {
                *engine = chat::engine::ChatEngine::new(config, ctx.author().id);
            }
        })
        .or_insert_with({
            data.config.write().await.update();
            let mut config = data.config.read().await.clone();
            config.update();
            || chat::engine::ChatEngine::new(config, ctx.author().id)
        });

    let mut freewill_map = data.freewill_map.write().await;
    if let Some(handle) = freewill_map.remove(ctx.author()) {
        handle.abort();
    }

    ctx.send(
        CreateReply::default()
            .content("cleared context window and reloaded engine.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
