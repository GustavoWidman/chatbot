use chrono::Duration;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::utils;

use super::{prompt::SystemPrompt, template::TemplateVariables};

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct SystemPromptBuilder {
    pub chatbot_name: String,
    pub user_name: String,
    pub about: String,
    pub max_ltm: usize,
    pub tone: Option<String>,
    pub age: Option<String>,
    pub likes: Option<Vec<String>>,
    pub dislikes: Option<Vec<String>>,
    pub history: Option<String>,
    pub conversation_goals: Option<Vec<String>>,
    pub conversational_examples: Option<Vec<String>>,
    pub context: Option<Vec<String>>,

    #[serde(skip)]
    pub long_term_memory: Option<Vec<String>>,

    pub user_about: Option<String>,
    pub timezone: Option<Tz>,
    pub language: Option<String>,
}
impl SystemPromptBuilder {
    #[allow(unused)]
    pub fn add_long_term_memory(mut self, new_memory: String) -> Self {
        // Maintain a rolling window for long term memory.
        if let Some(ref mut memories) = self.long_term_memory {
            if memories.len() + 1 > self.max_ltm {
                memories.remove(0);
            }
            memories.push(new_memory);
        } else {
            self.long_term_memory = Some(vec![new_memory]);
        }
        self
    }

    pub fn add_long_term_memories(&mut self, new_memories: Vec<String>) {
        if let Some(memories) = &mut self.long_term_memory {
            // If adding all new memories would exceed the limit, drain the oldest.
            let new_total = memories.len() + new_memories.len();
            if new_total > self.max_ltm {
                memories.drain(0..(new_total - self.max_ltm));
            }
            memories.extend(new_memories);
        } else {
            self.long_term_memory = Some(new_memories);
        }
    }

    pub fn build(mut self, time_since_last: Duration) -> SystemPrompt {
        let time = if let Some(timezone) = self.timezone {
            chrono::Utc::now()
                .with_timezone(&timezone)
                .format("%Y-%m-%d %H:%M:%S %z")
                .to_string()
        } else {
            chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S %z")
                .to_string()
        };

        let time_since = utils::time_to_string(time_since_last);

        let variables = TemplateVariables::new(
            self.user_name.clone(),
            self.chatbot_name.clone(),
            time,
            time_since,
        );

        self.tone = variables.substitute_optional_template(self.tone.as_deref());
        self.age = variables.substitute_optional_template(self.age.as_deref());
        self.likes = variables.substitute_optional_templates(self.likes.as_deref());
        self.dislikes = variables.substitute_optional_templates(self.dislikes.as_deref());
        self.history = variables.substitute_optional_template(self.history.as_deref());
        self.conversation_goals =
            variables.substitute_optional_templates(self.conversation_goals.as_deref());
        self.conversational_examples =
            variables.substitute_optional_templates(self.conversational_examples.as_deref());
        self.context = variables.substitute_optional_templates(self.context.as_deref());
        self.long_term_memory =
            variables.substitute_optional_templates(self.long_term_memory.as_deref());
        self.user_about = variables.substitute_optional_template(self.user_about.as_deref());
        self.language = variables.substitute_optional_template(self.language.as_deref());

        SystemPrompt::new(self)
    }
}
