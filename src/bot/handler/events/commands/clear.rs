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
        let new_engine = {
            data.config.write().await.update();
            let config = data.config.read().await.clone();
            let mut new_engine =
                chat::engine::ChatEngine::new(config, ctx.author().id, ctx.http()).await?;
            new_engine.clear_context();
            RwLock::new(new_engine)
        };

        user_map.remove(ctx.author());
        user_map.insert(ctx.author().clone(), new_engine);

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
