use serenity::all::{Context, Interaction};

use super::{super::Handler, error::HandlerResult};

impl Handler {
    pub async fn on_interaction(
        &self,
        ctx: Context,
        interaction: Interaction,
    ) -> HandlerResult<()> {
        match interaction.into_message_component() {
            Some(mut component) => {
                let e = self.disable_buttons(&mut component, &ctx).await;

                if let Err(why) = e {
                    log::error!("error editing message: {why:?}");
                    return HandlerResult::err(why, (ctx.http, *component.message));
                }

                let _ = component.defer(ctx.http.clone()).await;

                let result = match component.data.custom_id.as_str() {
                    "regen" => self.regen(component.clone(), ctx.clone()).await,
                    "prev" => self.prev(component.clone(), ctx.clone()).await,
                    "next" => self.next(component.clone(), ctx.clone()).await,
                    _ => {
                        log::warn!(
                            "unknown custom_id \"{:?}\", ignoring",
                            component.data.custom_id
                        );
                        Ok(())
                    }
                };

                match result {
                    Ok(_) => HandlerResult::ok(()),
                    Err(why) => HandlerResult::err(why, (ctx.http, *component.message)),
                }
            }
            _ => HandlerResult::ok(()),
        }
    }
}
