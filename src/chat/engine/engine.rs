use std::ops::{Deref, DerefMut};

use anyhow::anyhow;
use serenity::all::{Http, UserId};

use crate::{
    chat::{
        client::{CompletionAgent, CompletionResult},
        context::{ContextWindow, MessageIdentifier},
    },
    config::{store::ChatBotConfig, structure::ChatBotConfigInner},
};

use super::super::context::{ChatContext, ChatMessage};

pub struct ChatEngine {
    pub client: CompletionAgent,
    user_id: UserId,
    context: ChatContext,
}

impl ChatEngine {
    pub async fn new(config: ChatBotConfig, user_id: UserId, http: &Http) -> anyhow::Result<Self> {
        let ChatBotConfigInner {
            context: context_config,
            llm: llm_config,
            ..
        } = config.into_inner();

        let context = ChatContext::new(&context_config, user_id, http).await;
        let client = CompletionAgent::new(
            llm_config,
            user_id,
            context_config.system.user_name,
            context_config.system.chatbot_name,
        )
        .await?;

        Ok(Self {
            client,
            context,
            user_id,
        })
    }

    // initializes with
    pub async fn reload(self, config: ChatBotConfig) -> anyhow::Result<Self> {
        // let client = client.unwrap_or(ChatClient::new(&config.llm, user_id))
        let ChatBotConfigInner {
            context: context_config,
            llm: llm_config,
            ..
        } = config.into_inner();

        let client = CompletionAgent::new(
            llm_config,
            self.user_id,
            context_config.system.user_name,
            context_config.system.chatbot_name,
        )
        .await?;

        Ok(Self {
            client,
            context: self.context,
            user_id: self.user_id,
        })
    }

    pub fn into_context(self) -> ChatContext {
        self.context
    }

    pub async fn user_prompt(
        &mut self,
        prompt: Option<(String, MessageIdentifier)>,
        context: Option<ContextType>,
    ) -> anyhow::Result<ChatMessage> {
        let retries = 5;

        let mut i = 0;
        while i < retries {
            let (prompt, message_id) = match prompt.clone() {
                Some((prompt, message_id)) => (Some(prompt), Some(message_id)),
                None => (None, None),
            };

            let context: ContextWindow = match context {
                Some(ContextType::User) => self.context.get_context(prompt).await?,
                Some(ContextType::Freewill) => self.context.freewill_context(prompt).await?,
                Some(ContextType::Regen(ref message_id)) => {
                    self.context.get_regen_context(message_id).await?
                }
                None => self.context.get_context(prompt).await?,
            };

            if let Some(drained) = context.overflow {
                log::info!("draining {drained:?}");
                self.client
                    .store(
                        drained,
                        &self.context.config.system.user_name,
                        &self.context.config.system.chatbot_name,
                    )
                    .await?;
            }

            let mut prompt = if let Some(prompt) = context.user_prompt {
                Some(prompt)
            } else {
                None
            }
            .ok_or(anyhow!("unable to get a user prompt"))?;

            // retry if we get an error as well, but only up to the max retries
            let response = match self
                .client
                .completion(&mut prompt, context.system_prompt, context.history)
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
                        log::trace!("output: {content}");
                        if content.len() > 100000 {
                            i += 1;
                            log::warn!("too big, retry #{i}");
                            continue;
                        } else {
                            self.context.add_user_message(
                                prompt,
                                message_id.unwrap_or(MessageIdentifier::random()),
                            )?;
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
        user_name: &str,
        assistant_name: &str,
    ) -> anyhow::Result<()> {
        self.client.store(context, user_name, assistant_name).await
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
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
