use serenity::all::{
    ActionRowComponent, ButtonKind, Context, CreateActionRow, CreateButton, EditMessage, Message,
};

use super::Handler;

mod edit;
mod next;
mod prev;
mod regen;

impl Handler {
    pub async fn disable_buttons(&self, mut message: Message, ctx: &Context) -> anyhow::Result<()> {
        let buttons = CreateActionRow::Buttons(
            message
                .components
                .iter()
                .flat_map(|c| &c.components)
                .filter_map(|c| match c {
                    ActionRowComponent::Button(button) => Some(button),
                    _ => None,
                })
                .filter_map(|button| {
                    if let ButtonKind::NonLink { custom_id, style } = &button.data {
                        Some(
                            CreateButton::new(custom_id)
                                .disabled(true)
                                .label(
                                    button
                                        .label
                                        .as_ref()
                                        .map(|l| l.clone())
                                        .unwrap_or("".to_string()),
                                )
                                .emoji(button.emoji.as_ref().map(|e| e.clone()).unwrap_or(
                                    serenity::all::ReactionType::Unicode("🔄".to_string()),
                                ))
                                .style(*style),
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        );

        message
            .edit(
                ctx.http.clone(),
                EditMessage::new().components(vec![buttons]),
            )
            .await?;

        Ok(())
    }
}
