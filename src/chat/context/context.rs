use std::{fs::File, path::PathBuf};

use anyhow::{Result, anyhow};
use branch_context::{Message, Messages};
use indexmap::IndexMap;
use rig::message::{Message as RigMessage, UserContent};
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, Http, Message as SerenityMessage, MessageId, UserId};

use crate::{bot::handler::Handler, config::structure::ContextConfig, utils};

use super::{MessageRole, message::ChatMessage};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Copy)]
pub struct MessageIdentifier {
    pub message_id: u64,
    pub channel_id: u64,
    pub random: bool,
}
impl From<Option<(MessageId, ChannelId)>> for MessageIdentifier {
    fn from(value: Option<(MessageId, ChannelId)>) -> Self {
        match value {
            Some((message_id, channel_id)) => Self {
                message_id: message_id.get(),
                channel_id: channel_id.get(),
                random: false,
            },
            None => Self::random(),
        }
    }
}
impl From<(MessageId, ChannelId)> for MessageIdentifier {
    fn from(value: (MessageId, ChannelId)) -> Self {
        Self {
            message_id: value.0.get(),
            channel_id: value.1.get(),
            random: false,
        }
    }
}
impl MessageIdentifier {
    pub fn random() -> Self {
        Self {
            message_id: rand::random(),
            channel_id: rand::random(),
            random: true,
        }
    }

    pub async fn to_message(&self, http: &Http) -> Option<SerenityMessage> {
        if self.random {
            log::warn!("random message {:?} requested", self);
            return None;
        }

        http.get_message(self.channel_id.into(), self.message_id.into())
            .await
            .map_err(|why| {
                log::error!("failed to get message: {why:?}");
            })
            .ok()
    }
}

pub struct ChatContext {
    messages: IndexMap<MessageIdentifier, Messages<ChatMessage>>,
    save_path: Option<PathBuf>,
    pub config: ContextConfig,
}

pub struct ContextWindow {
    pub user_prompt: Option<ChatMessage>,
    pub system_prompt: String,
    pub history: Vec<ChatMessage>,
    pub overflow: Option<Vec<ChatMessage>>,
}

impl ChatContext {
    pub async fn new(config: &ContextConfig, user_id: UserId, http: &Http) -> Self {
        let save_path = &config
            .save_to_disk_folder
            .as_ref()
            .map(|path| {
                if path.is_file() {
                    std::fs::remove_file(&path)
                        .map_err(|e| {
                            log::error!("Failed to remove file: {e}");
                            e
                        })
                        .ok()?;
                }

                std::fs::create_dir_all(&path)
                    .map_err(|e| {
                        log::error!("Failed to create dir: {e}");
                        e
                    })
                    .ok()?;

                Some(path.join(format!("context-{}.bin", user_id)))
            })
            .flatten();

        let result = match save_path {
            Some(path) => {
                File::open(path)
                    .ok()
                    .and_then(|mut file| {
                        serde_cbor::from_reader(&mut file)
                            .map_err(|e| {
                                log::error!("Failed to deserialize context: {e}");
                                e
                            })
                            .ok()
                    })
                    .map(
                        async |messages: IndexMap<MessageIdentifier, Messages<ChatMessage>>| {
                            log::info!(
                                "Recovered context with {} messages for user {}",
                                messages.len(),
                                user_id
                            );

                            if config.disable_buttons.unwrap_or(false) {
                                // reenable buttons
                                // todo group by channel_id and use get_messages instead of
                                // a single get_message call for each one (helps with rate-limiting)
                                let discord_messages = futures::future::join_all(
                                    messages
                                        .iter()
                                        .filter(|(_, message)| {
                                            matches!(
                                                message.selected().role(),
                                                MessageRole::Assistant
                                            )
                                        })
                                        .map(async |(id, message)| {
                                            (id.to_message(&http).await, message)
                                        })
                                        .collect::<Vec<_>>(),
                                )
                                .await
                                .into_iter()
                                .filter_map(|(message, messages)| message.map(|m| (m, messages)))
                                .collect::<Vec<_>>();

                                for (message, messages) in discord_messages {
                                    Handler::enable_buttons(
                                        message,
                                        http,
                                        messages.forward,
                                        messages.backward,
                                    )
                                    .await
                                    .map_err(|why| {
                                        log::error!("failed to enable buttons: {why:?}");
                                        why
                                    })?;
                                }
                            }

                            Ok(Self {
                                messages,
                                save_path: save_path.clone(),
                                config: config.clone(),
                            })
                        },
                    )
            }
            None => None,
        };

        match result {
            Some(future) => future.await.unwrap_or_else(|_: anyhow::Error| Self {
                messages: IndexMap::new(),
                save_path: save_path.clone(),
                config: config.clone(),
            }),
            None => Self {
                messages: IndexMap::new(),
                save_path: save_path.clone(),
                config: config.clone(),
            },
        }
    }

    pub async fn shutdown(&self) -> anyhow::Result<Vec<MessageIdentifier>> {
        if let Some(path) = &self.save_path {
            log::info!(
                "Saving context with {} messages to {}",
                self.messages.len(),
                path.display()
            );

            let file = File::options().write(true).create(true).open(path)?;
            file.set_len(0)?;
            serde_cbor::to_writer(file, &self.messages)?;
        }

        // return the message ids in the index map
        if self.config.disable_buttons.unwrap_or(false) {
            Ok(self
                .messages
                .iter()
                .filter(|(_, message)| matches!(message.selected().role(), MessageRole::Assistant))
                .map(|(id, _)| id)
                .cloned()
                .collect())
        } else {
            // todo: this is a cheap hack to make sure no buttons are disabled,
            // there might be some consequences to this laziness
            Ok(vec![])
        }
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn add_message(
        &mut self,
        message: impl Into<Message<ChatMessage>>,
        id: impl Into<MessageIdentifier>,
    ) {
        let message = Messages::new(message.into());
        self.messages.insert(id.into(), message);
    }

    pub fn add_user_message(&mut self, message: String, id: impl Into<MessageIdentifier>) {
        self.add_message(
            ChatMessage {
                inner: RigMessage::user(message),
                ..Default::default()
            },
            id,
        );
    }

    pub fn latest(&self) -> Option<&Messages<ChatMessage>> {
        self.messages.last().map(|(_, m)| m)
    }

    // #[allow(unused)]
    /// Returns the latest message with the given role.
    // pub fn latest_with_role(&self, user: String) -> Option<&Messages<ChatMessage>> {
    //     self.messages
    //         .iter()
    //         .rev()
    //         .find(|(_, m)| m.selected().role == user)
    //         .map(|(_, m)| m)
    // }

    #[allow(unused)]
    /// Returns the message with the given id (not index, if you want the index use [ChatContext::get])
    pub fn find(&self, id: impl Into<MessageIdentifier>) -> Option<&Messages<ChatMessage>> {
        self.messages.get(&id.into())
    }
    #[allow(unused)]
    /// Returns the message at the given index (not id, if you want the id use [ChatContext::find])
    pub fn get(&self, index: usize) -> Option<&Messages<ChatMessage>> {
        self.messages.get_index(index).map(|(_, m)| m)
    }
    /// Same as [ChatContext::find] but for mutable references
    pub fn find_mut(&mut self, id: &MessageIdentifier) -> Option<&mut Messages<ChatMessage>> {
        self.messages.get_mut(id)
    }
    #[allow(unused)]
    /// Same as [ChatContext::get] but for mutable references
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Messages<ChatMessage>> {
        self.messages.get_index_mut(index).map(|(_, m)| m)
    }
    /// Finds message with the given id, returning the index, the id, and the message itself.
    pub fn find_full(
        &self,
        id: &MessageIdentifier,
    ) -> Option<(usize, &MessageIdentifier, &Messages<ChatMessage>)> {
        self.messages.get_full(id)
    }
    #[allow(unused)]
    /// Same as [ChatContext::find_full] but for mutable references to the message.
    pub fn find_full_mut(
        &mut self,
        id: &MessageIdentifier,
    ) -> Option<(usize, &MessageIdentifier, &mut Messages<ChatMessage>)> {
        self.messages.get_full_mut(id)
    }

    /// If STM is full, drain until STM is 80% of max_stm
    async fn drain_overflow(&mut self) -> Option<Vec<ChatMessage>> {
        if self.messages.len() >= self.config.max_stm {
            let to_remove = self.messages.len() - ((self.config.max_stm * 4) / 5);
            log::info!("context close to or full, draining {to_remove} messages");
            Some(
                self.messages
                    .drain(0..to_remove)
                    .map(|(_, m)| m.into_selected())
                    .collect::<Vec<ChatMessage>>(),
            )
        } else {
            None
        }
    }

    async fn get_messages(&self) -> Vec<ChatMessage> {
        self.messages
            .iter()
            .map(|(_, messages)| messages.selected().clone())
            .collect::<Vec<_>>()
    }

    pub async fn take_until_freewill(&self) -> Vec<ChatMessage> {
        self.messages
            .iter()
            .take_while(|(_, messages)| !messages.selected().freewill)
            .map(|(_, messages)| messages.selected().clone())
            .collect::<Vec<_>>()
    }

    pub async fn get_context(&mut self) -> Result<ContextWindow> {
        if self.messages.is_empty() {
            let system_prompt = self
                .config
                .system
                .clone()
                .build(chrono::Duration::seconds(0));

            return Ok(ContextWindow {
                history: vec![],
                overflow: None,
                system_prompt: system_prompt.to_string(),
                user_prompt: None,
            });
        }

        // Add the messages
        let ctx = self.get_messages().await;

        let drained = self.drain_overflow().await;

        let system_prompt = self.config.system.clone().build(
            // unwrapping is safe because we know the context is not empty
            self.time_since_last().unwrap(),
        );

        Ok(ContextWindow {
            user_prompt: None,
            system_prompt: system_prompt.to_string(),
            history: ctx,
            overflow: drained,
        })
    }

    // gets context but excludes the last message and the user prompt is taken as string-only
    // regenerating never drains, because it does not increase the STM size, so .1 is always None
    pub async fn get_regen_context(
        &mut self,
        message_id: &MessageIdentifier,
    ) -> Result<ContextWindow> {
        let (index, _, _) = self
            .find_full(message_id)
            .ok_or(anyhow!("message not found"))?;

        // get from 0..index
        let mut ctx = self
            .messages
            .get_range(0..index)
            .ok_or(anyhow!("context not found"))?
            .iter()
            .map(|(_, messages)| messages.selected().clone())
            .collect::<Vec<_>>();

        // Extract the last text message from the user as the prompt
        let last_message = ctx
            .iter()
            .rposition(|msg| {
                matches!(
                    msg.inner,
                    RigMessage::User {
                        content: ref c
                    } if matches!(c.first(), UserContent::Text(_))
                )
            })
            .map(|idx| ctx.remove(idx))
            .ok_or_else(|| anyhow::anyhow!("No user text messages found for prompting"))?;

        let system_prompt = self.config.system.clone().build(
            // unwrapping is safe because we know the context is not empty
            self.time_since_last().unwrap(),
        );

        // if let Some(pos) = context.iter().rposition(|m| m.role == "assistant") {
        //     context.remove(pos);
        // }

        Ok(ContextWindow {
            user_prompt: Some(last_message),
            history: ctx,
            system_prompt: system_prompt.to_string(),
            overflow: None, // there is no overflow when regenerating
        })
    }

    pub async fn freewill_context(&mut self) -> Result<ContextWindow> {
        let ContextWindow {
            history,
            overflow,
            system_prompt,
            ..
        } = self.get_context().await?;

        let message = ChatMessage {
            inner: RigMessage::user(format!(
                "*it's been around {} since you last said something, and the user did not respond. your next response should attempt to pull the user back into the conversation. please respond once again, making sure to keep the same tone and style as you normally would, following all previous instructions, yet keeping the time difference in mind. your response should only contain the actual response, not your thoughts or anything else.*\n\n\"...\"",
                utils::time_to_string(self.time_since_last()?)
            )),
            ..Default::default()
        };

        // id-less message
        self.add_message(message.clone(), None);

        Ok(ContextWindow {
            user_prompt: Some(message),
            history,
            system_prompt,
            overflow,
        })
    }

    pub fn time_since_last(&self) -> anyhow::Result<chrono::Duration> {
        let last = self
            .latest()
            .ok_or(anyhow!("Context is empty, there's no last message"))?;

        Ok(chrono::Utc::now() - last.selected().sent_at)
    }

    #[allow(unused)]
    pub fn add_long_term_memories(&mut self, memories: Vec<String>) {
        self.config.system.add_long_term_memories(memories);
    }
}
