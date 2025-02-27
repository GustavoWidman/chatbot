use std::ops::{Deref, DerefMut};

use anyhow::anyhow;
use rig::message::Message as RigMessage;
use serenity::all::{Http, UserId};

use crate::{
    chat::{
        client::{CompletionAgent, CompletionResult},
        context::{ContextWindow, MessageIdentifier},
    },
    config::store::ChatBotConfig,
};

use super::super::context::{ChatContext, ChatMessage};

pub struct ChatEngine {
    client: CompletionAgent,
    context: ChatContext,
}

impl ChatEngine {
    pub async fn new(config: ChatBotConfig, user_id: UserId, http: &Http) -> anyhow::Result<Self> {
        let context = ChatContext::new(&config.context, user_id, http).await;
        let client = CompletionAgent::new(config.llm.clone(), user_id).await?;

        Ok(Self { client, context })
    }

    // initializes with
    pub async fn new_with(
        config: ChatBotConfig,
        user_id: UserId,
        http: &Http,
        client: Option<CompletionAgent>,
        context: Option<ChatContext>,
    ) -> anyhow::Result<Self> {
        // let client = client.unwrap_or(ChatClient::new(&config.llm, user_id))
        let client = client.unwrap_or(CompletionAgent::new(config.llm.clone(), user_id).await?);
        let context = context.unwrap_or(ChatContext::new(&config.context, user_id, http).await);

        Ok(Self { client, context })
    }

    pub fn into_context(self) -> ChatContext {
        self.context
    }

    pub async fn user_prompt(
        &mut self,
        prompt: Option<String>,
        context: Option<ContextType>,
    ) -> anyhow::Result<ChatMessage> {
        let retries = 5;

        let mut i = 0;
        while i < retries {
            let context: ContextWindow = match context {
                Some(ContextType::User) => self.context.get_context().await?,
                Some(ContextType::Freewill) => self.context.freewill_context().await?,
                Some(ContextType::Regen(ref message_id)) => {
                    self.context.get_regen_context(message_id).await?
                }
                None => self.context.get_context().await?,
            };

            if let Some(drained) = context.overflow {
                log::info!("draining {drained:?}");
                self.client
                    .store(
                        drained,
                        self.context.config.system.user_name.clone(),
                        self.context.config.system.chatbot_name.clone(),
                    )
                    .await?;
            }

            let prompt = if let Some(prompt) = context.user_prompt {
                Some(prompt)
            } else if let Some(prompt) = prompt.clone() {
                // we have to clone because of prior or future loops
                Some(ChatMessage {
                    inner: RigMessage::user(prompt),
                    ..Default::default()
                })
            } else {
                None
            }
            .ok_or(anyhow!("unable to get a user prompt"))?;

            // retry if we get an error as well, but only up to the max retries
            let response = match self
                .client
                .completion(prompt, context.system_prompt, context.history)
                .await
            {
                Ok(response) => response,
                Err(why) => {
                    if i + 1 >= retries {
                        return Err(why);
                    } else {
                        log::warn!("error:\n{why:?}\nretrying, attempt {i}");
                        i += 1;
                        continue;
                    }
                }
            };

            match response {
                CompletionResult::Message(completion_message) => {
                    let message = ChatMessage::from(completion_message);

                    let content = message.content();

                    if let Some(content) = content {
                        if content.len() > 2000 {
                            i += 1;
                            log::warn!("too big, retry #{i}");
                            continue;
                        } else {
                            return Ok(message);
                        }
                    } else {
                        log::error!("no content in message");
                        continue;
                    }
                }
                CompletionResult::Tool((call, response)) => {
                    self.context.add_message(ChatMessage::from(call), None);
                    self.context.add_message(ChatMessage::from(response), None);

                    log::info!("called functions, prompting again");

                    continue;
                }
            }
        }

        Err(anyhow::anyhow!("too many retries"))
    }

    pub async fn summarize_and_store(
        &self,
        context: Vec<ChatMessage>,
        user_name: String,
        assistant_name: String,
    ) -> anyhow::Result<()> {
        self.client.store(context, user_name, assistant_name).await
    }

    pub async fn shutdown(&self) -> anyhow::Result<Vec<MessageIdentifier>> {
        self.context.shutdown().await
    }

    pub fn clear_context(&mut self) {
        self.context.clear()
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
    Regen(MessageIdentifier),
}
