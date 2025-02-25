use super::{Context, Error};
use crate::bot::handler::{
    Handler,
    events::{HandlerResult, commands},
};

/// Rewrite keys of the LLM config
#[poise::command(slash_command, prefix_command)]
pub(super) async fn config(
    ctx: Context<'_>,
    #[description = "Config property"] key: commands::KeyChoice,

    #[description = "New value (if not provided, will print the current key value)"] value: Option<
        String,
    >,
) -> Result<(), Error> {
    if let HandlerResult::Err(why) = commands::config(ctx, key, value).await {
        Handler::on_error(why).await;
    }

    Ok(())
}
