use std::ops::{Deref, DerefMut};

use anyhow::Result;
use genai::chat::ChatMessage;
use serde_json::json;
use serenity::all::UserId;

use crate::config::store::ChatBotConfig;

use super::{
    client::ChatClient,
    context::{self, ChatContext, CompletionMessage, Messages},
};

pub struct ChatEngine {
    client: ChatClient,
    context: ChatContext,
}

impl ChatEngine {
    pub fn new(config: ChatBotConfig, user_id: UserId) -> Self {
        let client = ChatClient::new(&config.llm, user_id);
        let context = ChatContext::new(&config.prompt);

        Self { client, context }
    }

    // initializes with
    pub fn new_with(
        config: ChatBotConfig,
        user_id: UserId,
        client: Option<ChatClient>,
        context: Option<ChatContext>,
    ) -> Self {
        let client = client.unwrap_or(ChatClient::new(&config.llm, user_id));
        let context = context.unwrap_or(ChatContext::new(&config.prompt));

        Self { client, context }
    }

    pub fn into_context(self) -> ChatContext {
        self.context
    }

    pub fn into_client(self) -> ChatClient {
        self.client
    }

    pub async fn user_prompt(
        &mut self,
        prompt: Option<String>,
        context: Option<ContextType>,
    ) -> anyhow::Result<CompletionMessage> {
        let retries = 5;

        let mut i = 0;
        let mut has_recalled = false;
        while i < retries {
            let (mut context, drained): (Vec<CompletionMessage>, Option<Vec<CompletionMessage>>) =
                match context {
                    Some(ContextType::User) => self.context.get_context(!has_recalled).await,
                    Some(ContextType::Freewill) => {
                        self.context.freewill_context(!has_recalled).await
                    }
                    Some(ContextType::Regen) => self.context.get_regen_context(!has_recalled).await,
                    None => self.context.get_context(!has_recalled).await,
                }?;

            if let Some(drained) = drained {
                println!("draining {drained:?}");
                self.client
                    .store(
                        drained,
                        self.context.system_prompt.user_name.clone(),
                        self.context.system_prompt.chatbot_name.clone(),
                    )
                    .await?;
            }

            if let Some(prompt) = prompt.clone() {
                context.push(CompletionMessage {
                    role: "user".to_string(),
                    content: prompt,
                });
            }

            let response = self.client.prompt(context.clone(), !has_recalled).await?;

            match response {
                super::client::PromptResult::Message(completion_message) => {
                    if completion_message.content.len() > 2000 {
                        i += 1;
                        println!("too big, retry #{i}");
                        continue;
                    } else {
                        return Ok(completion_message);
                    }
                }
                super::client::PromptResult::MemoryRecall(recalled_memories) => {
                    has_recalled = true;
                    println!("recalled memories: {recalled_memories:?}");
                    self.context
                        .add_long_term_memories(recalled_memories.clone());
                    self.context.add_message(CompletionMessage {
                        role: "tool".to_string(),
                        // content: recalled_memories
                        //     .join("\n---\n")
                        //     .trim_end_matches("\n---\n")
                        //     .to_string(),
                        content: json!({
                            "name": "memory_recall",
                            "asdasd": recalled_memories
                            .join("\n---\n")
                            .trim_end_matches("\n---\n")
                            .to_string(),
                        })
                        .to_string(),
                    });
                }
            }
        }

        Err(anyhow::anyhow!("too many retries"))
    }
}

impl Deref for ChatEngine {
    type Target = ChatContext;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl DerefMut for ChatEngine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

pub enum ContextType {
    User,
    Freewill,
    Regen,
}
