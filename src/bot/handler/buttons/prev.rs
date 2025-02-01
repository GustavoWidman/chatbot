use serenity::all::{ComponentInteraction, Context, CreateButton, EditMessage};

use crate::chat;

use super::super::Handler;

impl Handler {
    pub async fn prev(
        &self,
        mut component: ComponentInteraction,
        ctx: Context,
    ) -> anyhow::Result<()> {
        let data = self.data.clone();

        let mut user_map = data.user_map.write().await;
        let engine = user_map.entry(component.user.clone()).or_insert_with({
            data.config.write().await.update();
            let config = data.config.read().await.clone();
            || chat::engine::ChatEngine::new(config)
        });

        let (message, can_go_back) = engine.go_back().unwrap();

        component
            .message
            .edit(
                ctx.http.clone(),
                EditMessage::new()
                    .content(message.content.clone())
                    .button(
                        CreateButton::new("prev")
                            .label("")
                            .emoji('⏪')
                            .style(serenity::all::ButtonStyle::Secondary)
                            .disabled(!can_go_back),
                    )
                    .button(
                        CreateButton::new("next")
                            .label("")
                            .emoji('⏩')
                            .style(serenity::all::ButtonStyle::Secondary)
                            .disabled(false),
                    ),
            )
            .await?;

        Ok(())
    }
}
