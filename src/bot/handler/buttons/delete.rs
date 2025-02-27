use serenity::all::{ComponentInteraction, Context};

use super::super::Handler;

impl Handler {
    pub async fn delete_error(
        &self,
        component: ComponentInteraction,
        ctx: Context,
    ) -> anyhow::Result<()> {
        component.message.delete(ctx.http).await?;

        Ok(())
    }
}
