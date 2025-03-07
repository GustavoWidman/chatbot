use serenity::all::{Context, Interaction};

use super::{
    super::Handler,
    error::{ErrorLocation, HandlerResult},
};

impl Handler {
    pub async fn on_interaction(
        &self,
        ctx: Context,
        interaction: Interaction,
    ) -> HandlerResult<()> {
        match interaction.kind() {
            serenity::all::InteractionType::Component => {
                return self.on_component(ctx, interaction).await;
            }
            serenity::all::InteractionType::Modal => {
                return self.on_modal_submit(ctx, interaction).await;
            }
            _ => {}
        }

        HandlerResult::ok(())
    }

    async fn on_component(&self, ctx: Context, interaction: Interaction) -> HandlerResult<()> {
        if let Some(mut component) = interaction.into_message_component() {
            let result = match component.data.custom_id.as_str() {
                id @ ("regen" | "prev" | "next") => {
                    if let Err(why) = self.disable_buttons(&mut *component.message, &ctx).await {
                        log::error!("error editing message: {why:?}");
                        return HandlerResult::err(why, (ctx.http, *component.message));
                    };

                    if let Err(err) = component.defer(&ctx.http).await {
                        log::error!("error deferring component: {err:?}");
                        return HandlerResult::err(err, (ctx.http, *component.message));
                    }

                    match id {
                        "regen" => self.regen(component.clone(), ctx.clone()).await,
                        "prev" => self.prev(component.clone(), ctx.clone()).await,
                        "next" => self.next(component.clone(), ctx.clone()).await,
                        _ => unreachable!(),
                    }
                }
                "delete_error" => self.delete_error(component.clone(), ctx.clone()).await,
                "edit" => self.edit_button(component.clone(), ctx.clone()).await,
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
        } else {
            log::warn!("unknown interaction type");
            HandlerResult::ok(())
        }
    }

    async fn on_modal_submit(&self, ctx: Context, interaction: Interaction) -> HandlerResult<()> {
        if let Some(modal) = interaction.into_modal_submit() {
            let result = match modal.data.custom_id.as_str() {
                custom_id if custom_id.starts_with("edit_") => {
                    self.edit_modal(modal.clone(), ctx.clone()).await
                }
                _ => {
                    log::warn!("unknown custom_id \"{:?}\", ignoring", modal.data.custom_id);
                    Ok(())
                }
            };

            match result {
                Ok(_) => HandlerResult::ok(()),
                Err(why) => {
                    let location: ErrorLocation<'_> = match modal.message {
                        Some(message) => (ctx.http, *message).into(),
                        None => (ctx.http, modal.channel_id).into(),
                    };

                    HandlerResult::err(why, location)
                }
            }
        } else {
            log::warn!("unknown interaction type");
            HandlerResult::ok(())
        }
    }
}
