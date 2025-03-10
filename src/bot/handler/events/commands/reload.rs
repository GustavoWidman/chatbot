use poise::CreateReply;

use tokio::sync::RwLock;

use crate::bot::handler::events::HandlerResult;
use crate::bot::handler::framework::Context;
use crate::chat;
use crate::utils::macros::config;

/// Reloads the engine without clearing the context window
pub async fn reload(ctx: Context<'_>) -> HandlerResult<()> {
    let data = ctx.data();

    let config = config!(&data);

    let mut user_map = data.user_map.write().await;

    let result: anyhow::Result<()> = async {
        let author = ctx.author();
        let engine = match user_map.remove(&author.id) {
            Some(engine) => chat::engine::ChatEngine::reload(engine.into_inner(), config).await,
            None => chat::engine::ChatEngine::new(config, ctx.author().id).await,
        }?;
        user_map.insert(author.id, RwLock::new(engine));

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
