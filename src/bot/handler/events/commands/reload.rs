use poise::CreateReply;

use tokio::sync::RwLock;

use crate::bot::handler::events::HandlerResult;
use crate::bot::handler::framework::Context;
use crate::chat;

/// Reloads the engine without clearing the context window
pub async fn reload(ctx: Context<'_>) -> HandlerResult<()> {
    let data = ctx.data().clone();

    data.config.write().await.update();
    let config = data.config.read().await.clone();

    let mut user_map = data.user_map.write().await;

    let result: anyhow::Result<()> = async {
        let engine = match user_map.remove(ctx.author()) {
            Some(engine) => chat::engine::ChatEngine::reload(engine.into_inner()).await,
            None => chat::engine::ChatEngine::new(config, ctx.author().id, ctx.http()).await,
        }?;
        user_map.insert(ctx.author().clone(), RwLock::new(engine));

        ctx.send(
            CreateReply::default()
                .content("reloaded engine, context intact.")
                .ephemeral(true),
        )
        .await?;

        Ok(())
    }
    .await;

    match result {
        Ok(_) => HandlerResult::ok(()),
        Err(why) => HandlerResult::err(why, ctx),
    }
}
