use anyhow::anyhow;
use serenity::all::{
    ActionRowComponent, Builder, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateInputText, CreateInteractionResponse, CreateModal, EditMessage, InputTextStyle,
    ModalInteraction,
};

use crate::chat::{ChatMessage, engine::EngineGuard};

use super::super::Handler;

impl Handler {
    pub async fn edit_button(
        &self,
        component: ComponentInteraction,
        ctx: Context,
    ) -> anyhow::Result<()> {
        let modal = CreateModal::new(format!("edit_{}", component.message.id), "Edit Response")
            .components(vec![CreateActionRow::InputText(
                CreateInputText::new(InputTextStyle::Paragraph, "Response", "response")
                    .placeholder("Edit this response here")
                    .value(&component.message.content)
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
        let ModalInteraction {
            id,
            token,
            user,
            message,
            data,
            ..
        } = interaction;

        let mut message = if let Some(message) = message {
            message
        } else {
            return Err(anyhow!("could not find message reference to edit"));
        };

        let content = data
            .components
            .into_iter()
            .next()
            .and_then(|row| row.components.into_iter().next())
            .and_then(|component| match component {
                ActionRowComponent::InputText(text) => Some(text.value),
                _ => None,
            })
            .flatten()
            .ok_or_else(|| anyhow!("could not find content to edit"))?;

        let data = &self.data;

        let guard = EngineGuard::lock(&data, user.id, &ctx.http).await?;
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

        self.disable_buttons(&mut *message, &ctx).await?;

        message
            .edit(
                &ctx.http,
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

        CreateInteractionResponse::Acknowledge
            .execute(&ctx.http, (id, &token))
            .await?;

        Ok(())
    }
}
