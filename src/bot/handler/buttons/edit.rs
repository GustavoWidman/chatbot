use anyhow::anyhow;
use serenity::all::{
    ActionRowComponent, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateInputText, CreateModal, EditMessage, InputTextStyle, ModalInteraction,
};

use crate::chat::{ChatMessage, engine::EngineGuard};

use super::super::Handler;

impl Handler {
    pub async fn edit_button(
        &self,
        component: ComponentInteraction,
        ctx: Context,
    ) -> anyhow::Result<()> {
        let old_content = component.message.content.clone();

        let modal = CreateModal::new(format!("edit_{}", component.message.id), "Edit Response")
            .components(vec![CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Paragraph, "Response", "response")
                    .placeholder("Edit this response here")
                    .value(old_content)
                    .max_length(2000)
                    .required(true)
                    .min_length(1),
            )]);

        component
            .create_response(
                ctx.http,
                serenity::all::CreateInteractionResponse::Modal(modal),
            )
            .await?;

        Ok(())
    }

    pub async fn edit_modal(
        &self,
        interaction: ModalInteraction,
        ctx: Context,
    ) -> anyhow::Result<()> {
        let mut message = if let Some(message) = interaction.message.clone() {
            message
        } else {
            return Err(anyhow!("could not find message reference to edit"));
        };

        let content = interaction
            .data
            .components
            .first()
            .and_then(|row| row.components.first())
            .and_then(|component| match component {
                ActionRowComponent::InputText(text) => Some(text.value.clone()),
                _ => None,
            })
            .and_then(|value| value)
            .ok_or_else(|| anyhow!("could not find content to edit"))?;

        let data = self.data.clone();

        let guard = EngineGuard::lock(&data, interaction.user.clone(), &ctx.http).await?;
        let mut engine = guard.engine().await.write().await;

        let messages = match engine.find_mut(&(message.id, message.channel_id).into()) {
            Some(messages) => messages,
            None => {
                log::warn!(
                    "No conversation thread found for edited message id: {:?}, is this our fault?",
                    message.id
                );
                return Err(anyhow!("message not found in engine"));
            }
        };

        self.disable_buttons(*message.clone(), &ctx).await?;

        message
            .edit(
                ctx.http.clone(),
                EditMessage::new()
                    .content(content.clone())
                    .button(
                        CreateButton::new("prev")
                            .label("")
                            .emoji('⏪')
                            .style(serenity::all::ButtonStyle::Secondary)
                            .disabled(false),
                    )
                    .button(
                        CreateButton::new("regen")
                            .label("")
                            .emoji('♻')
                            .style(serenity::all::ButtonStyle::Secondary)
                            .disabled(false),
                    )
                    .button(
                        CreateButton::new("edit")
                            .label("")
                            .emoji('✏')
                            .style(serenity::all::ButtonStyle::Secondary)
                            .disabled(false),
                    ),
            )
            .await?;

        messages.push(ChatMessage::assistant(content));

        // fulfill the interaction
        interaction.defer(ctx.http.clone()).await?;

        Ok(())
    }
}
