use anyhow::anyhow;
use serenity::all::{
    ActionRowComponent, Builder, ComponentInteraction, Context, CreateActionRow, CreateInputText,
    CreateInteractionResponse, CreateModal, EditMessage, InputTextStyle, ModalInteraction,
};

use crate::{
    chat::{ChatMessage, context::MessageIdentifier, engine::EngineGuard},
    utils::misc::{self, ButtonStates},
};

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

        let message = if let Some(message) = message {
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

        let (_, identifier, _) = match engine
            .find_full_mut(&(message.id, message.channel_id).into())
        {
            Some(messages) => messages,
            None => {
                log::warn!(
                    "No conversation thread found for edited message id: {:?}, is this our fault?",
                    message.id
                );
                return Err(anyhow!("message not found in engine"));
            }
        };
        let channel = identifier.channel();
        let messages = identifier.messages();

        CreateInteractionResponse::Acknowledge
            .execute(&ctx.http, (id, &token))
            .await?;

        let out: anyhow::Result<(ChatMessage, MessageIdentifier)> = async {
            misc::delete_message_batch(channel, &ctx.http, messages).await?;

            let messages = misc::chunk_message(
                &content,
                ButtonStates {
                    prev_disabled: false,
                    regen_or_next: misc::RegenOrNext::Regen,
                },
            )?;

            let ids = misc::send_message_batch(channel, &ctx.http, messages).await?;
            let last_id = ids.last().ok_or(anyhow::anyhow!("no message ids"))?.clone();

            Ok((
                ChatMessage::assistant(content),
                (last_id, channel, ids).into(),
            ))
        }
        .await;

        match out {
            Ok((response, new_identifier)) => {
                let identifier = (message.id, message.channel_id).into();
                let messages = engine
                    .find_mut(&identifier)
                    .ok_or(anyhow::anyhow!("message not found in engine"))?;

                messages.push(response); // pushes and selects

                let message = ctx
                    .http
                    .get_message(new_identifier.channel(), new_identifier.message())
                    .await;

                engine.swap_identifiers(&identifier, new_identifier)?;

                if let Ok(mut message) = message {
                    tokio::spawn({
                        let mut recv = self.data.msg_channel.0.subscribe();
                        async move {
                            let _ = recv.recv().await;

                            let _ = message
                                .edit(&ctx.http, EditMessage::new().components(vec![]))
                                .await;

                            drop(recv);
                        }
                    });

                    Ok(())
                } else {
                    Err(anyhow::anyhow!("could not fetch discord message"))
                }
            }
            Err(why) => Err(why),
        }
    }
}
