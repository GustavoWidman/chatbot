use serenity::all::{
    ActionRowComponent, ButtonKind, ComponentInteraction, Context, CreateActionRow, CreateButton,
    EditMessage,
};

use super::Handler;

mod next;
mod prev;
mod regen;

impl Handler {
    pub async fn disable_buttons(
        &self,
        component: &mut ComponentInteraction,
        ctx: &Context,
    ) -> anyhow::Result<()> {
        let buttons = CreateActionRow::Buttons(
            component
                .message
                .components
                .clone()
                .into_iter()
                .flat_map(|c| c.components)
                .filter_map(|c| match c {
                    ActionRowComponent::Button(button) => Some(button),
                    _ => None,
                })
                .filter_map(|button| {
                    if let ButtonKind::NonLink { custom_id, style } = button.data {
                        Some(
                            CreateButton::new(custom_id)
                                .disabled(true)
                                .label(button.label.unwrap_or("".to_string()))
                                .emoji(button.emoji.unwrap_or(
                                    serenity::all::ReactionType::Unicode("🔄".to_string()),
                                ))
                                .style(style),
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        );

        component
            .message
            .edit(
                ctx.http.clone(),
                EditMessage::new().components(vec![buttons]),
            )
            .await?;

        Ok(())
    }
}
