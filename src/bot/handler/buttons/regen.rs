use serenity::all::{ComponentInteraction, Context, CreateButton, EditMessage};

use crate::chat::{
    engine::{ContextType, EngineGuard},
    ChatMessage,
};

use super::super::Handler;

impl Handler {
    pub async fn regen(
        &self,
        mut component: ComponentInteraction,
        ctx: Context,
    ) -> anyhow::Result<()> {
        let data = self.data.clone();

        let guard = EngineGuard::lock(&data, component.user).await;
        let mut engine = guard.engine().await.write().await;

        // uses this to find the error before other things
        let _ = engine
            .find_mut(component.message.id)
            .ok_or(anyhow::anyhow!("message not found in engine"))?;

        let old_content = component.message.content.clone();
        component
            .message
            .edit(
                ctx.http.clone(),
                EditMessage::new().content("https://i.gifer.com/3OjRd.gif"),
            )
            .await?;

        let out: anyhow::Result<ChatMessage> = async {
            let response = engine
                .user_prompt(None, Some(ContextType::Regen(component.message.id)))
                .await?;

            let content = response.content.clone();

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

            Ok(response)
        }
        .await;

        match out {
            Ok(out) => {
                let message = engine
                    .find_mut(component.message.id)
                    .ok_or(anyhow::anyhow!("message not found in engine"))?;

                message.push(out); // pushes and selects

                Ok(())
            }
            Err(why) => {
                component
                    .message
                    .edit(ctx.http.clone(), EditMessage::new().content(old_content))
                    .await?;

                Err(why)
            }
        }
    }
}
