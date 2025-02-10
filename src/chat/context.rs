use std::vec;

use anyhow::{anyhow, Result};
use genai::chat::ChatMessage;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionMessage, MessageRole, ToolCall};
use serenity::all::UserId;

use crate::chat::prompt::SystemPromptBuilder;

#[derive(Debug, Clone)]
pub struct Messages {
    list: Vec<(CompletionMessage, chrono::DateTime<chrono::Utc>)>,
    selected: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionMessage {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl Into<ChatMessage> for CompletionMessage {
    fn into(self) -> ChatMessage {
        match self.role.as_str() {
            "system" => ChatMessage::system(self.content),
            "user" => ChatMessage::user(self.content),
            "assistant" => ChatMessage::assistant(self.content),
            _ => ChatMessage::system(self.content),
        }
    }
}

impl Into<ChatCompletionMessage> for CompletionMessage {
    fn into(self) -> ChatCompletionMessage {
        let role = match self.role.as_str() {
            "system" => MessageRole::system,
            "user" => MessageRole::user,
            "assistant" => MessageRole::assistant,
            "tool" => MessageRole::tool, // todo maybe change back to "tool"
            "function" => MessageRole::function,
            _ => MessageRole::system,
        };

        chat_completion::ChatCompletionMessage {
            role,
            content: chat_completion::Content::Text(self.content),
            name: self.name,
            tool_calls: self.tool_calls,
            tool_call_id: None,
        }
    }
}

impl TryInto<ChatMessage> for Messages {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<ChatMessage> {
        let selected_message = self
            .list
            .into_iter()
            .nth(self.selected)
            .ok_or(anyhow!("Selected message is out of bounds, wtf?"))?;

        Ok(selected_message.0.into())
    }
}

impl TryInto<CompletionMessage> for Messages {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<CompletionMessage> {
        let selected_message = self
            .list
            .into_iter()
            .nth(self.selected)
            .ok_or(anyhow!("Selected message is out of bounds, wtf?"))?;

        Ok(selected_message.0)
    }
}

pub struct ChatContext {
    messages: Vec<Messages>,
    pub system_prompt: SystemPromptBuilder,
}

impl ChatContext {
    pub fn new(system_prompt: &SystemPromptBuilder) -> Self {
        Self {
            messages: vec![],
            system_prompt: system_prompt.clone(),
        }
    }

    pub fn add_message(&mut self, message: CompletionMessage) {
        let messages = Messages {
            list: vec![(message, chrono::Utc::now())],
            selected: 0,
        };

        self.messages.push(messages);
    }

    pub fn add_user_message(&mut self, message: String) {
        self.add_message(CompletionMessage {
            role: "user".to_string(),
            content: message,
            ..Default::default()
        });
    }

    pub fn regenerate(&mut self, message: CompletionMessage) -> anyhow::Result<()> {
        match self.messages.last_mut() {
            // get latest message
            Some(messages) => {
                messages.list.push((message, chrono::Utc::now())); // push new message
                messages.selected = messages.list.len() - 1; // set selected message

                Ok(())
            }
            None => Err(anyhow::anyhow!("Context is empty, nothing to regenerate")),
        }
    }

    pub fn go_back(&mut self) -> anyhow::Result<(CompletionMessage, bool)> {
        match self.messages.last_mut() {
            // get latest message
            Some(messages) => {
                if messages.selected < 1 {
                    unreachable!("if this is happening you are a terrible programmer");
                }

                messages.selected = messages.selected - 1; // set selected message to the previous

                let message = messages.list[messages.selected].clone();

                Ok((message.0, messages.selected != 0))
            }
            None => Err(anyhow::anyhow!("Context is empty, nothing to regenerate")),
        }
    }

    pub fn go_fwd(&mut self) -> anyhow::Result<(CompletionMessage, bool)> {
        match self.messages.last_mut() {
            Some(messages) => {
                if messages.selected + 1 > messages.list.len() - 1 {
                    unreachable!("if this is happening you are a terrible programmer");
                }

                messages.selected = messages.selected + 1; // set selected message to the previous

                let message = messages.list[messages.selected].clone();

                Ok((message.0, messages.selected + 1 <= messages.list.len() - 1))
            }
            None => Err(anyhow::anyhow!("Context is empty, nothing to regenerate")),
        }
    }

    pub async fn get_context(
        &mut self,
        recalling: bool,
    ) -> anyhow::Result<(Vec<CompletionMessage>, Option<Vec<CompletionMessage>>)> {
        let mut ctx = vec![];

        // Add the messages
        self.messages.clone().into_iter().for_each(|messages| {
            match messages.list.into_iter().nth(messages.selected) {
                Some(message) => {
                    ctx.push(message);
                }
                None => {}
            }
        });

        let last_message_time = ctx.last().map(|m| m.1).unwrap_or(chrono::Utc::now());

        // if the STM is full, we will remove the last 20% of the STM from the beginning (oldest)

        // 50 stm, 50 max stm
        let drained = if self.messages.len() >= self.system_prompt.max_stm {
            let to_remove = self.messages.len() - ((self.system_prompt.max_stm * 4) / 5);
            println!("context close to or full, draining {to_remove} messages");
            Some(
                self.messages
                    .drain(0..to_remove)
                    .map(|m| m.try_into())
                    .collect::<anyhow::Result<Vec<CompletionMessage>>>()?,
            )
        } else {
            None
        };

        let system_prompt = self
            .system_prompt
            .clone()
            .build(last_message_time, recalling);

        let mut context = vec![CompletionMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
            ..Default::default()
        }];
        context.extend(ctx.into_iter().map(|m| m.0));

        Ok((context, drained))
    }

    // gets context but excludes the last message and the user prompt is taken as string-only
    pub async fn get_regen_context(
        &mut self,
        recalling: bool,
    ) -> Result<(Vec<CompletionMessage>, Option<Vec<CompletionMessage>>)> {
        let (mut context, drained) = self.get_context(recalling).await?;

        // take off the last two, keep the second to last
        if let Some(pos) = context.iter().rposition(|m| m.role == "assistant") {
            context.remove(pos);
        }

        // context.push(CompletionMessage {
        //     role: "system".to_string(),
        //     content: "Please send a different response than you'd usually do, but keep the same tone and style as you normally would, following all previous instructions".to_string(),
        // });

        Ok((context, drained))
    }

    pub async fn freewill_context(
        &mut self,
        recalling: bool,
    ) -> Result<(Vec<CompletionMessage>, Option<Vec<CompletionMessage>>)> {
        let (mut context, drained) = self.get_context(recalling).await?;
        let last = self
            .messages
            .last()
            .ok_or(anyhow!("Context is empty, nothing to freewill out of"))?;

        let time_since_last = chrono::Utc::now()
            - last
                .list
                .get(last.selected)
                .ok_or(anyhow!("Selected message is out of bounds, wtf?"))?
                .1;

        // testing
        // let time_since_last = time_since_last * 1000;

        let time_since_last_as_str = match time_since_last.num_seconds() {
            0..=59 => {
                let second_suffix = if time_since_last.num_seconds() > 1 {
                    "s"
                } else {
                    ""
                };
                format!("{} second{}", time_since_last.num_seconds(), second_suffix)
            }
            60..=3599 => {
                let minute_suffix = if time_since_last.num_minutes() > 1 {
                    "s"
                } else {
                    ""
                };
                format!("{} minute{}", time_since_last.num_minutes(), minute_suffix)
            }
            3600..=86399 => {
                let hour_suffix = if time_since_last.num_hours() > 1 {
                    "s"
                } else {
                    ""
                };
                format!("{} hour{}", time_since_last.num_hours(), hour_suffix)
            }
            _ => {
                let day_suffix = if time_since_last.num_days() > 1 {
                    "s"
                } else {
                    ""
                };
                format!("{} day{}", time_since_last.num_days(), day_suffix)
            }
        };

        let message = CompletionMessage {
            role: "user".to_string(),
            content: format!(
                "*it's been around {} since you last said something, and the user did not respond. your next response should attempt to pull the user back into the conversation. please respond once again, making sure to keep the same tone and style as you normally would, following all previous instructions, yet keeping the time difference in mind. your response should only contain the actual response, not your thoughts or anything else.*\n\n\"...\"",
                time_since_last_as_str
            ),
            ..Default::default()
        };

        self.add_message(message.clone());

        context.push(message);

        Ok((context, drained))
    }

    pub fn time_since_last(&self) -> anyhow::Result<f64> {
        let last = self
            .messages
            .last()
            .ok_or(anyhow!("Context is empty, nothing to freewill out of"))?;

        Ok((chrono::Utc::now()
            - last
                .list
                .get(last.selected)
                .ok_or(anyhow!("Selected message is out of bounds, wtf?"))?
                .1)
            .num_seconds() as f64)
    }

    pub fn add_long_term_memories(&mut self, memories: Vec<String>) {
        self.system_prompt = self.system_prompt.clone().add_long_term_memories(memories);
    }
}
