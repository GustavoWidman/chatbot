use anyhow::{Result, anyhow};
use branch_context::{Message, Messages};
use indexmap::IndexMap;
use rig::message::{Message as RigMessage, UserContent};
use serenity::all::MessageId;

use crate::{chat::prompt::SystemPromptBuilder, utils};

use super::message::ChatMessage;

pub struct ChatContext {
    messages: IndexMap<u64, Messages<ChatMessage>>,
    pub system_prompt: SystemPromptBuilder,
}

pub struct ContextWindow {
    pub user_prompt: Option<ChatMessage>,
    pub system_prompt: String,
    pub history: Vec<ChatMessage>,
    pub overflow: Option<Vec<ChatMessage>>,
}

impl ChatContext {
    pub fn new(system_prompt: &SystemPromptBuilder) -> Self {
        Self {
            messages: IndexMap::new(),
            system_prompt: system_prompt.clone(),
        }
    }

    pub fn add_message(
        &mut self,
        message: impl Into<Message<ChatMessage>>,
        id: Option<impl Into<u64>>,
    ) {
        let message = Messages::new(message.into(), id.map(|id| id.into()));
        self.messages.insert(message.id, message);
    }

    pub fn add_user_message(&mut self, message: String, id: MessageId) {
        self.add_message(
            ChatMessage {
                inner: RigMessage::user(message),
                ..Default::default()
            },
            Some(id),
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
    pub fn find(&self, id: impl Into<u64>) -> Option<&Messages<ChatMessage>> {
        self.messages.get(&id.into())
    }
    #[allow(unused)]
    /// Returns the message at the given index (not id, if you want the id use [ChatContext::find])
    pub fn get(&self, index: usize) -> Option<&Messages<ChatMessage>> {
        self.messages.get_index(index).map(|(_, m)| m)
    }
    /// Same as [ChatContext::find] but for mutable references
    pub fn find_mut(&mut self, id: impl Into<u64>) -> Option<&mut Messages<ChatMessage>> {
        self.messages.get_mut(&id.into())
    }
    #[allow(unused)]
    /// Same as [ChatContext::get] but for mutable references
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Messages<ChatMessage>> {
        self.messages.get_index_mut(index).map(|(_, m)| m)
    }
    /// Finds message with the given id, returning the index, the id, and the message itself.
    pub fn find_full(&self, id: impl Into<u64>) -> Option<(usize, &u64, &Messages<ChatMessage>)> {
        self.messages.get_full(&id.into())
    }
    #[allow(unused)]
    /// Same as [ChatContext::find_full] but for mutable references to the message.
    pub fn find_full_mut(
        &mut self,
        id: impl Into<u64>,
    ) -> Option<(usize, &u64, &mut Messages<ChatMessage>)> {
        self.messages.get_full_mut(&id.into())
    }

    /// If STM is full, drain until STM is 80% of max_stm
    async fn drain_overflow(&mut self) -> Option<Vec<ChatMessage>> {
        if self.messages.len() >= self.system_prompt.max_stm {
            let to_remove = self.messages.len() - ((self.system_prompt.max_stm * 4) / 5);
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
                .system_prompt
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

        let system_prompt = self.system_prompt.clone().build(
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
    pub async fn get_regen_context(&mut self, message_id: MessageId) -> Result<ContextWindow> {
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

        let system_prompt = self.system_prompt.clone().build(
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
        self.add_message(message.clone(), None::<u64>);

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
        self.system_prompt.add_long_term_memories(memories);
    }
}
