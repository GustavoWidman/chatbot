use serenity::all::{ComponentInteraction, Context, CreateButton, EditMessage};

use crate::chat;

use super::super::Handler;

impl Handler {
    pub async fn regen(
        &self,
        mut component: ComponentInteraction,
        ctx: Context,
    ) -> anyhow::Result<()> {
        let data = self.data.clone();

        let mut user_map = data.user_map.write().await;
        let engine = user_map.entry(component.user.clone()).or_insert_with({
            let config = data.config.read().await.clone();
            || chat::engine::ChatEngine::new(config)
        });

        let (prompt, regen_context) = engine.get_regen_context();

        component
            .message
            .edit(
                &ctx.http,
                EditMessage::new().content("https://i.gifer.com/3OjRd.gif"),
            )
            .await?;

        let response = engine.user_prompt(prompt, regen_context).await?;

        let content = response.content.clone();

        engine.regenerate(response)?;

        component
            .message
            .edit(
                &ctx.http,
                EditMessage::new()
                    .content(content)
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
                    ),
            )
            .await?;

        Ok(())
    }
}
