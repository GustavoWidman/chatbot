use poise::CreateReply;

use tokio::sync::RwLock;

use crate::bot::handler::events::HandlerResult;
use crate::bot::handler::framework::Context;
use crate::chat;

/// Clears the current context window and reloads the engine
pub async fn clear(ctx: Context<'_>) -> HandlerResult<()> {
    let data = ctx.data().clone();

    let mut user_map = data.user_map.write().await;

    let result: anyhow::Result<()> = async {
        user_map
            .entry(ctx.author().clone())
            .and_modify({
                data.config.write().await.update();
                let config = data.config.read().await.clone();
                let new_engine = chat::engine::ChatEngine::new(config, ctx.author().id).await?;
                |engine| {
                    *engine = RwLock::new(new_engine);
                }
            })
            .or_insert_with({
                data.config.write().await.update();
                let config = data.config.read().await.clone();
                let engine = chat::engine::ChatEngine::new(config, ctx.author().id).await?;
                || RwLock::new(engine)
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
    .await;

    match result {
        Ok(_) => HandlerResult::ok(()),
        Err(why) => HandlerResult::err(why, ctx),
    }
}
