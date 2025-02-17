use std::ops::{Deref, DerefMut};

use openai_api_rs::v1::chat_completion::{ToolCall, ToolCallFunction};
use serde_json::json;
use serenity::all::{MessageId, UserId};

use crate::config::store::ChatBotConfig;

use super::super::{
    client::{ChatClient, PromptResult},
    context::{ChatContext, ChatMessage},
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
    ) -> anyhow::Result<ChatMessage> {
        let retries = 5;

        let mut i = 0;
        let mut has_recalled = false;
        while i < retries {
            let (mut context, drained): (Vec<ChatMessage>, Option<Vec<ChatMessage>>) = match context
            {
                Some(ContextType::User) => self.context.get_context(!has_recalled).await,
                Some(ContextType::Freewill) => self.context.freewill_context(!has_recalled).await?,
                Some(ContextType::Regen(message_id)) => {
                    self.context
                        .get_regen_context(message_id, !has_recalled)
                        .await?
                }
                None => self.context.get_context(!has_recalled).await,
            };

            if let Some(drained) = drained {
                log::info!("draining {drained:?}");
                self.client
                    .store(
                        drained,
                        self.context.system_prompt.user_name.clone(),
                        self.context.system_prompt.chatbot_name.clone(),
                    )
                    .await?;
            }

            if let Some(prompt) = prompt.clone() {
                context.push(ChatMessage {
                    role: "user".to_string(),
                    content: prompt,
                    ..Default::default()
                });
            }

            // retry if we get an error as well, but only up to the max retries
            let response = match self.client.prompt(context.clone(), !has_recalled).await {
                Ok(response) => response,
                Err(why) => {
                    if i + 1 >= retries {
                        return Err(why);
                    } else {
                        log::warn!("error: {why:?}, retrying");
                        i += 1;
                        continue;
                    }
                }
            };

            match response {
                PromptResult::Message(completion_message) => {
                    if completion_message.content.len() > 2000 {
                        i += 1;
                        log::warn!("too big, retry #{i}");
                        continue;
                    } else {
                        return Ok(completion_message);
                    }
                }
                PromptResult::MemoryRecall((query, recalled_memories)) => {
                    has_recalled = true;
                    log::info!("recalled memories: {recalled_memories:?}");
                    self.context
                        .add_long_term_memories(recalled_memories.clone());
                    self.context.add_message(
                        ChatMessage {
                            role: "assistant".to_string(),
                            content: " ".to_string(),
                            tool_calls: Some(vec![ToolCall {
                                id: "".to_string(),
                                r#type: "function".to_string(),
                                function: ToolCallFunction {
                                    name: Some("memory_recall".to_string()),
                                    arguments: Some(
                                        json!({
                                            "query": query,
                                        })
                                        .to_string(),
                                    ),
                                },
                            }]),
                            name: None,
                            ..Default::default()
                        },
                        None::<u64>,
                    );

                    let mut stringified_memories = recalled_memories
                        .join("\n---\n")
                        .trim_end_matches("\n---\n")
                        .to_string();

                    if stringified_memories.is_empty() {
                        stringified_memories = "No memories found.".to_string()
                    }

                    self.context.add_message(
                        ChatMessage {
                            role: "function".to_string(),
                            content: stringified_memories,
                            name: Some("memory_recall".to_string()),
                            tool_calls: None,
                            ..Default::default()
                        },
                        None::<u64>,
                    );
                }
                PromptResult::MemoryStore(memory) => {
                    log::info!("memory stored: {memory}");

                    self.context.add_message(
                        ChatMessage {
                            role: "assistant".to_string(),
                            content: " ".to_string(),
                            tool_calls: Some(vec![ToolCall {
                                id: "".to_string(),
                                r#type: "function".to_string(),
                                function: ToolCallFunction {
                                    name: Some("memory_store".to_string()),
                                    arguments: Some(
                                        json!({
                                            "memory": memory,
                                        })
                                        .to_string(),
                                    ),
                                },
                            }]),
                            name: None,
                            ..Default::default()
                        },
                        None::<u64>,
                    );
                    self.context.add_message(
                        ChatMessage {
                            role: "function".to_string(),
                            content: "Memory stored successfully".to_string(),
                            name: Some("memory_store".to_string()),
                            tool_calls: None,
                            ..Default::default()
                        },
                        None::<u64>,
                    );
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
    Regen(MessageId),
}
