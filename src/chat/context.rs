use std::vec;

use anyhow::{Result, anyhow};
use chrono::Duration;
use genai::chat::ChatMessage;
use serenity::all::MessageId;

use crate::chat::prompt::SystemPromptBuilder;

#[derive(Debug, Clone)]
struct Messages {
    id: MessageId,
    list: Vec<(CompletionMessage, chrono::DateTime<chrono::Utc>)>,
    selected: usize,
}

#[derive(Debug, Clone)]
pub struct CompletionMessage {
    pub role: String,
    pub content: String,
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

#[derive(Debug)]
pub struct ChatContext {
    messages: Vec<Messages>,
    system_prompt: SystemPromptBuilder,
}

impl ChatContext {
    pub fn new(system_prompt: &SystemPromptBuilder) -> Self {
        Self {
            messages: vec![],
            system_prompt: system_prompt.clone(),
        }
    }

    pub fn add_message(&mut self, message: CompletionMessage, id: MessageId) {
        let messages = Messages {
            id,
            list: vec![(message, chrono::Utc::now())],
            selected: 0,
        };

        self.messages.push(messages);
    }

    pub fn add_user_message(&mut self, message: String, discord_message_id: MessageId) {
        self.add_message(
            CompletionMessage {
                role: "user".to_string(),
                content: message,
            },
            discord_message_id,
        );
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

    pub fn get_context(&self) -> Vec<CompletionMessage> {
        let mut context = vec![];
        let (system_prompt, time) = self.system_prompt.clone().build();

        // context.push(CompletionMessage {
        //     role: "system".to_string(),
        //     content: system_prompt.to_string(),
        // });

        // Add the messages
        self.messages.clone().into_iter().for_each(|messages| {
            match messages.list.into_iter().nth(messages.selected) {
                Some(message) => {
                    context.push(message.0);
                }
                None => {}
            }
        });

        // Add the time
        // context.push(CompletionMessage {
        //     role: "system".to_string(),
        //     content: format!(
        //         "Updated date and time, use the following timestamp for this reply: {}",
        //         time
        //     ),
        // });

        context
    }

    // gets context but excludes the last message and the user prompt is taken as string-only
    pub fn get_regen_context(&self) -> Vec<CompletionMessage> {
        let mut context = self.get_context();

        // take off the last two, keep the second to last
        if let Some(pos) = context.iter().rposition(|m| m.role == "assistant") {
            context.remove(pos);
        }

        context.push(CompletionMessage {
            role: "system".to_string(),
            content: "Please send a different response than you'd usually do, but keep the same tone and style as you normally would, following all previous instructions".to_string(),
        });

        context
    }

    pub fn freewill_context(&self) -> Result<Vec<CompletionMessage>> {
        let mut context = self.get_context();
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

        context.push(CompletionMessage {
            role: "system".to_string(),
            content: format!("It's been around {} since you last said something, and the user did not respond. Your next response should attempt to pull the user back into the conversation. Please respond once again, making sure to keep the same tone and style as you normally would, following all previous instructions, yet keeping the time difference in mind. Your response should only contain the actual response, not your thoughts or anything else.", time_since_last_as_str),
        });
        context.push(CompletionMessage {
            role: "user".to_string(),
            content: "...".to_string(),
        });

        Ok(context)
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
}
