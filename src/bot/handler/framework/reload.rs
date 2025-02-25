use super::{Context, Error};
use crate::bot::handler::{
    Handler,
    events::{HandlerResult, commands},
};

/// Reloads the engine without clearing the context window
#[poise::command(slash_command, prefix_command)]
pub(super) async fn reload(ctx: Context<'_>) -> Result<(), Error> {
    if let HandlerResult::Err(why) = commands::reload(ctx).await {
        Handler::on_error(why).await;
    }

    Ok(())
}
