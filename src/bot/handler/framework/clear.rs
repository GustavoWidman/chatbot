use super::{Context, Error};
use crate::bot::handler::{
    Handler,
    events::{HandlerResult, commands},
};

/// Clears the current context window and reloads the engine
#[poise::command(slash_command, prefix_command)]
pub(super) async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    if let HandlerResult::Err(why) = commands::clear(ctx).await {
        Handler::on_error(why).await;
    }

    Ok(())
}
