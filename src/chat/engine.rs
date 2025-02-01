use std::ops::{Deref, DerefMut};

use anyhow::Result;
use genai::chat::ChatMessage;

use crate::config::store::ChatBotConfig;

use super::{
    client::ChatClient,
    context::{ChatContext, CompletionMessage},
};

pub struct ChatEngine {
    client: ChatClient,
    context: ChatContext,
}

impl ChatEngine {
    pub fn new(config: ChatBotConfig) -> Self {
        let client = ChatClient::new(&config.llm);
        let context = ChatContext::new(&config.prompt);

        Self { client, context }
    }

    pub async fn user_prompt(
        &mut self,
        prompt: Option<String>,
        context: Vec<CompletionMessage>,
    ) -> anyhow::Result<CompletionMessage> {
        let retries = 5;

        for i in 0..retries {
            let response = self
                .client
                .prompt(
                    prompt.clone(),
                    context.clone().into_iter().map(|m| m.into()).collect(),
                )
                .await?;
            if response.content.len() > 2000 {
                println!("too big, retry #{i}");
                continue;
            } else {
                return Ok(response);
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
