use anyhow::bail;
use serenity::all::{ComponentInteraction, Context, CreateButton, EditMessage};

use crate::chat::engine::EngineGuard;

use super::super::Handler;

impl Handler {
    pub async fn prev(
        &self,
        mut component: ComponentInteraction,
        ctx: Context,
    ) -> anyhow::Result<()> {
        let data = self.data.clone();

        let guard = EngineGuard::lock(&data, component.user).await?;
        let mut engine = guard.engine().await.write().await;

        let message = engine
            .find_mut(component.message.id)
            .ok_or(anyhow::anyhow!("message not found in engine"))?;

        if !message.backward {
            bail!("message is already at the end of the context");
        }

        let content = message
            .backward()
            .content()
            .ok_or(anyhow::anyhow!("message does not have a content"))?;

        component
            .message
            .edit(
                ctx.http.clone(),
                EditMessage::new()
                    .content(content)
                    .button(
                        CreateButton::new("prev")
                            .label("")
                            .emoji('⏪')
                            .style(serenity::all::ButtonStyle::Secondary)
                            .disabled(!message.backward),
                    )
                    .button(
                        CreateButton::new("next")
                            .label("")
                            .emoji('⏩')
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

        Ok(())
    }
}
