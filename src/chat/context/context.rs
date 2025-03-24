use std::{fs::File, hash::Hash, path::PathBuf};

use anyhow::{Result, anyhow};
use branch_context::{Message, Messages};
use indexmap::IndexMap;
use rig::message::{Message as RigMessage, UserContent};
use serde::{Deserialize, Serialize};
use serenity::all::{ChannelId, Http, Message as SerenityMessage, MessageId, UserId};

use crate::{config::structure::ContextConfig, utils};

use super::{MessageRole, message::ChatMessage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageIdentifier {
    pub message_id: u64,
    pub channel_id: u64,
    pub random: bool,
    pub message_ids: Vec<u64>,
}
impl PartialEq for MessageIdentifier {
    fn eq(&self, other: &Self) -> bool {
        self.message_id == other.message_id
            && self.channel_id == other.channel_id
            && self.random == other.random
    }
}
impl Eq for MessageIdentifier {}
impl Hash for MessageIdentifier {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.message_id.hash(state);
        self.channel_id.hash(state);
        self.random.hash(state);
    }
}

impl From<Option<(MessageId, ChannelId)>> for MessageIdentifier {
    fn from(value: Option<(MessageId, ChannelId)>) -> Self {
        match value {
            Some((message_id, channel_id)) => Self {
                message_id: message_id.get(),
                channel_id: channel_id.get(),
                random: false,
                message_ids: vec![message_id.get()],
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
            message_ids: vec![value.0.get()],
        }
    }
}
impl From<(MessageId, ChannelId, Vec<MessageId>)> for MessageIdentifier {
    fn from(value: (MessageId, ChannelId, Vec<MessageId>)) -> Self {
        Self {
            message_id: value.0.get(),
            channel_id: value.1.get(),
            random: false,
            message_ids: value.2.into_iter().map(|id| id.get()).collect(),
        }
    }
}
impl MessageIdentifier {
    pub fn random() -> Self {
        let message_id = rand::random();
        Self {
            message_id,
            channel_id: rand::random(),
            random: true,
            message_ids: vec![message_id],
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

    pub fn channel(&self) -> ChannelId {
        ChannelId::new(self.channel_id)
    }

    pub fn message(&self) -> MessageId {
        MessageId::new(self.message_id)
    }

    pub fn messages(&self) -> Vec<MessageId> {
        self.message_ids
            .iter()
            .map(|id| MessageId::new(*id))
            .collect::<Vec<_>>()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserPrompt {
    pub content: Option<String>,
    pub current_time: String,
    #[serde(rename = "time_since_last_message")]
    pub time_since: String,
    pub relevant_memories: Vec<String>,
    pub system_note: Option<String>,
    #[serde(skip)]
    pub freewill: bool,
}

pub struct ChatContext {
    messages: IndexMap<MessageIdentifier, Messages<ChatMessage>>,
    save_path: Option<PathBuf>,
    pub config: ContextConfig,
}
impl TryInto<ChatMessage> for UserPrompt {
    type Error = anyhow::Error;

    fn try_into(mut self) -> Result<ChatMessage, Self::Error> {
        let mut message = String::new();

        if self.system_note.is_some() {
            message.push_str(&format!(
                "System Note:\n{}\n\n",
                self.system_note.as_ref().unwrap()
            ));
        }

        message.push_str(&format!(
            "The current time is {}, {} since the last message before this one.\n\n",
            self.current_time, self.time_since
        ));

        if self.relevant_memories.len() > 0 {
            message.push_str(&format!(
                "You have recalled the following memories:\n{}\n\n",
                self.relevant_memories
                    .iter_mut()
                    .map(|m| format!("```memory\n{}\n```", m))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if let Some(content) = self.content {
            message.push_str(&format!(
                "Respond to the following message:\n{}\n\n",
                content
            ));
        }

        // remove the last double newline
        message.pop();
        message.pop();

        log::trace!("message:\n{}\n\n", message);

        // let mut message = ChatMessage::user(serde_json::to_string(&self)?);
        let mut message = ChatMessage::user(message);

        message.freewill = self.freewill;
        Ok(message)
    }
}
impl TryInto<RigMessage> for UserPrompt {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<RigMessage, Self::Error> {
        Ok(RigMessage::user(serde_json::to_string(&self)?))
    }
}
impl TryFrom<ChatMessage> for UserPrompt {
    type Error = anyhow::Error;

    fn try_from(value: ChatMessage) -> Result<Self, Self::Error> {
        serde_json::from_str::<Self>(
            &value
                .content()
                .ok_or(anyhow::anyhow!("message does not have a content"))?,
        )
        .map_err(|why| anyhow::anyhow!("failed to deserialize user prompt: {why:?}"))
    }
}

pub struct ContextWindow {
    pub user_prompt: Option<UserPrompt>,
    pub system_prompt: String,
    pub history: Vec<ChatMessage>,
    pub overflow: Option<Vec<ChatMessage>>,
}

impl ChatContext {
    pub async fn new(config: &ContextConfig, user_id: UserId) -> Self {
        log::info!("creating new context");

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
                        ciborium::from_reader(&mut file)
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

                            // get latest message and reenable buttons
                            let context = Self {
                                messages,
                                save_path: save_path.clone(),
                                config: config.clone(),
                            };

                            Ok(context)
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

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        if let Some(path) = &self.save_path {
            log::info!(
                "Saving context with {} messages to {}",
                self.messages.len(),
                path.display()
            );

            let file = File::options().write(true).create(true).open(path)?;
            file.set_len(0)?;
            ciborium::into_writer(&self.messages, file)?;
        }

        Ok(())
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        if let Some(path) = &self.save_path {
            std::fs::remove_file(path).ok();
        }
    }

    pub fn add_message(
        &mut self,
        message: impl Into<Message<ChatMessage>>,
        id: impl Into<MessageIdentifier>,
    ) {
        let message = Messages::new(message.into());
        self.messages.insert(id.into(), message);
    }

    pub fn add_user_message(
        &mut self,
        message: UserPrompt,
        id: impl Into<MessageIdentifier>,
    ) -> anyhow::Result<()> {
        self.add_message(TryInto::<ChatMessage>::try_into(message)?, id);

        Ok(())
    }

    pub fn latest(&self) -> Option<&Messages<ChatMessage>> {
        self.messages.last().map(|(_, m)| m)
    }
    pub fn latest_mut(&mut self) -> Option<&mut Messages<ChatMessage>> {
        self.messages.last_mut().map(|(_, m)| m)
    }

    /// Returns the latest message with the given role.
    pub fn latest_with_role(&self, role: MessageRole) -> Option<&Messages<ChatMessage>> {
        self.latest_with_role_full(role).map(|(_, m)| m)
    }
    pub fn latest_with_role_full(
        &self,
        role: MessageRole,
    ) -> Option<(&MessageIdentifier, &Messages<ChatMessage>)> {
        self.messages
            .iter()
            .rev()
            .find(|(_, m)| m.selected().role() == role)
    }

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

    pub fn swap_identifiers(
        &mut self,
        old_id: &MessageIdentifier,
        new_id: impl Into<MessageIdentifier>,
    ) -> anyhow::Result<()> {
        let old_messages = self
            .messages
            .get(old_id)
            .ok_or(anyhow::anyhow!("message not found in engine"))?
            .clone();

        // insert the new identifier
        // clone is necessary, unfortunately
        if self.messages.insert(new_id.into(), old_messages).is_some() {
            anyhow::bail!("identifier already exists");
        }

        // remove the old identifier
        if self.messages.swap_remove(old_id).is_none() {
            anyhow::bail!("identifier not found");
        };

        Ok(())
    }

    /// If STM is full, drain until STM is x% of max_stm
    async fn drain_overflow(&mut self) -> Option<Vec<ChatMessage>> {
        if self.messages.len() >= self.config.max_stm {
            let to_remove = self.messages.len()
                - (self
                    .config
                    .stm_drain_percentage
                    .map(|p| 1.0 - p)
                    .unwrap_or(0.8) // default to 80% drain
                    * self.config.max_stm as f64)
                    .round() as usize;
            log::info!("context close to or full, draining {to_remove} messages");

            // set the latest message to be a "freewill" message
            // (even though it's not, just mark it as the delimiter for any next drains)
            if let Some(latest) = self.latest_mut() {
                latest.mut_selected().freewill = true;
            }

            Some(
                self.messages
                    .drain(0..to_remove)
                    .rev()
                    // only return all the way until a freewill message
                    .map_while(|(_, messages)| {
                        let message = messages.into_selected();
                        (!message.freewill).then_some(message)
                    })
                    .collect::<Vec<ChatMessage>>(),
            )
        } else {
            None
        }
    }

    async fn get_messages(&self) -> Vec<ChatMessage> {
        self.messages
            .iter()
            .map(|(_, messages)| messages.selected())
            .cloned()
            .collect::<Vec<_>>()
    }

    pub async fn take_until_freewill(&self) -> Vec<ChatMessage> {
        self.messages
            .iter()
            .rev()
            .map_while(|(_, messages)| {
                let selected = messages.selected();
                (!selected.freewill).then_some(selected)
            })
            .cloned()
            .collect::<Vec<_>>()
    }

    pub async fn get_context(&mut self, user_prompt: Option<String>) -> Result<ContextWindow> {
        let user_prompt: Option<UserPrompt> = match user_prompt {
            Some(prompt) => Some(UserPrompt {
                content: Some(prompt),
                current_time: self.config.system.get_time(),
                relevant_memories: vec![],
                time_since: utils::time_to_string(self.time_since_last()),
                system_note: None,
                freewill: false,
            }),
            None => None,
        };

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
                user_prompt,
            });
        }

        // Add the messages
        let ctx = self.get_messages().await;

        let drained = self.drain_overflow().await;

        let system_prompt = self.config.system.clone().build(self.time_since_last());

        Ok(ContextWindow {
            user_prompt,
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
            .map(|(_, messages)| messages.selected())
            .cloned()
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

        let system_prompt = self.config.system.clone().build(self.time_since_last());

        // if let Some(pos) = context.iter().rposition(|m| m.role == "assistant") {
        //     context.remove(pos);
        // }

        Ok(ContextWindow {
            user_prompt: Some(last_message.try_into()?),
            history: ctx,
            system_prompt: system_prompt.to_string(),
            overflow: None, // there is no overflow when regenerating
        })
    }

    pub async fn freewill_context(&mut self, user_prompt: Option<String>) -> Result<ContextWindow> {
        let ContextWindow {
            history,
            overflow,
            system_prompt,
            ..
        } = self.get_context(user_prompt).await?;

        // let message = ChatMessage::user(format!(
        //     "*it's been around {} since you last said something, and the user did not respond. your next response should attempt to pull the user back into the conversation. please respond once again, making sure to keep the same tone and style as you normally would, following all previous instructions, yet keeping the time difference in mind. your response should only contain the actual response, not your thoughts or anything else.*\n\n\"...\"",
        //     utils::time_to_string(self.time_since_last()?)
        // ));
        let message = UserPrompt {
            content: None,
            current_time: self.config.system.get_time(),
            relevant_memories: vec![],
            time_since: utils::time_to_string(self.time_since_last()),
            system_note: Some(
                "Please attempt to pull the user back into the conversation, making sure to keep the same tone and style as you normally would, following all previous instructions, yet keeping the time difference in mind. Your response should only contain the actual response, not your thoughts or anything else.".to_string(),
            ),
            freewill: true,
        };

        // id-less message
        self.add_message(TryInto::<ChatMessage>::try_into(message.clone())?, None);

        Ok(ContextWindow {
            user_prompt: Some(message),
            history,
            system_prompt,
            overflow,
        })
    }

    pub fn time_since_last(&self) -> chrono::Duration {
        let last = match self.latest() {
            Some(last) => last,
            None => return chrono::Duration::seconds(0),
        };

        chrono::Utc::now() - last.selected().sent_at
    }

    #[allow(unused)]
    pub fn add_long_term_memories(&mut self, memories: Vec<String>) {
        self.config.system.add_long_term_memories(memories);
    }
}
