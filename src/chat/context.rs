use std::vec;

use rig::completion::Message as CompletionMessage;
use serenity::all::MessageId;

use crate::chat::prompt::SystemPromptBuilder;

#[derive(Debug, Clone)]
struct Messages {
    id: MessageId,
    list: Vec<CompletionMessage>,
    selected: usize,
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
            list: vec![message],
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
                messages.list.push(message); // push new message
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

                Ok((message, messages.selected != 0))
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

                Ok((message, messages.selected + 1 <= messages.list.len() - 1))
            }
            None => Err(anyhow::anyhow!("Context is empty, nothing to regenerate")),
        }
    }

    pub fn get_context(&self) -> Vec<CompletionMessage> {
        let mut context = vec![];
        let (system_prompt, time) = self.system_prompt.clone().build();

        context.push(CompletionMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        });

        // Add the messages
        self.messages.clone().into_iter().for_each(|messages| {
            match messages.list.into_iter().nth(messages.selected) {
                Some(message) => {
                    context.push(message);
                }
                None => {}
            }
        });

        // Add the time
        context.push(CompletionMessage {
            role: "system".to_string(),
            content: format!(
                "Updated date and time, use the following timestamp for this reply: {}",
                time
            ),
        });

        context
    }

    // gets context but excludes the last message and the user prompt is taken as string-only
    pub fn get_regen_context(&self) -> (String, Vec<CompletionMessage>) {
        let context = self.get_context();
        let len = context.len();

        // take off the last two, keep the second to last
        let mut context = context.into_iter().take(len - 1).collect::<Vec<_>>();

        let prompt = context.iter().nth(len - 2).unwrap().content.clone();

        context.push(CompletionMessage {
            role: "system".to_string(),
            content: "Please send a different response than you'd usually do, but keep the same tone and style as you normally would, following all previous instructions".to_string(),
        });

        (prompt, context)
    }

    pub fn clear_context(&mut self) {
        self.messages.clear();
    }
}
