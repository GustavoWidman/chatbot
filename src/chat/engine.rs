use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use anyhow::{Result, anyhow};
use rig::completion::{CompletionModel, Message};
use serenity::prelude::TypeMapKey;
use tokio::sync::RwLock;

use crate::config::store::ChatBotConfig;

use super::{
    client::ChatClient,
    context::ChatContext,
    prompt::{SystemPrompt, SystemPromptBuilder},
};

pub struct ChatEngine {
    client: ChatClient,
    context: ChatContext,
}

impl TypeMapKey for ChatEngine {
    type Value = ChatEngine;
}

impl ChatEngine {
    pub fn new(config: ChatBotConfig) -> Self {
        let client = ChatClient::new(&config.llm);
        let context = ChatContext::new(&config.prompt);

        Self { client, context }
    }

    pub async fn user_prompt(
        &mut self,
        prompt: String,
        context: Vec<Message>,
    ) -> anyhow::Result<Message> {
        Ok(self.client.prompt(&prompt, context).await?)
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
