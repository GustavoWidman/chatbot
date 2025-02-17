use poise::CreateReply;

use tokio::sync::RwLock;

use super::{Context, Error};
use crate::chat;

/// Reloads the engine without clearing the context window
#[poise::command(slash_command, prefix_command)]
pub(super) async fn reload(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data().clone();

    data.config.write().await.update();
    let config = data.config.read().await.clone();

    let mut user_map = data.user_map.write().await;
    let engine = match user_map.remove(ctx.author()) {
        Some(engine) => chat::engine::ChatEngine::new_with(
            config,
            ctx.author().id,
            None,
            Some(engine.into_inner().into_context()),
        ),
        None => chat::engine::ChatEngine::new(config, ctx.author().id),
    };
    user_map.insert(ctx.author().clone(), RwLock::new(engine));

    ctx.send(
        CreateReply::default()
            .content("reloaded engine, context intact.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
