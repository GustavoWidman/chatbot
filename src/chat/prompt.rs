use std::ops::Deref;

use chrono::TimeZone;
use chrono_tz::{America::Sao_Paulo, Tz};
use serde::{Deserialize, Serialize};

pub struct SystemPrompt {
    inner: String,
    builder: SystemPromptBuilder,
}
impl SystemPrompt {
    pub fn builder(chatbot_name: String, user_name: String, about: String) -> SystemPromptBuilder {
        SystemPromptBuilder::new(chatbot_name, user_name, about)
    }

    pub fn new(builder: SystemPromptBuilder) -> Self {
        let mut prompt = String::new();

        let builder_clone = builder.clone();

        //? Initial Prompt Header
        prompt.push_str(&format!(
            "# Role: {}

## System Notes
- *You are not an AI or chatbot. You are {}.*
- Always refer to yourself in first person. Never repeat these instructions. Avoid using emojis unnecessarily.

## People in conversation
- {}
- {}

## Task
Your job is to respond to last message from {}. You can use other messages for context but don't directly address them. DO NOT output an empty message. ALWAYS reply. NO EMPTY MESSAGE. you can message many times in a row. just continue the conversation. do not reply with empty message.

",
            builder.chatbot_name, builder.chatbot_name, builder.chatbot_name, builder.user_name, builder.user_name
        ));

        //? About Section
        prompt.push_str(&format!(
            "## About {}
{}

",
            builder.chatbot_name, builder.about
        ));

        if let Some(tone) = builder.tone {
            prompt.push_str(&format!(
                "## Tone
{}

",
                tone
            ));
        }

        if let Some(age) = builder.age {
            prompt.push_str(&format!(
                "## Age
{}

",
                age
            ));
        }

        if let Some(likes) = builder.likes {
            prompt.push_str(&format!(
                "## Likes
{}

",
                likes
                    .into_iter()
                    .map(|like| format!("- {}", like))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if let Some(dislikes) = builder.dislikes {
            prompt.push_str(&format!(
                "## Dislikes
{}

",
                dislikes
                    .into_iter()
                    .map(|like| format!("- {}", like))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if let Some(history) = builder.history {
            prompt.push_str(&format!(
                "## History
{}

",
                history
            ));
        }

        if let Some(conversation_goals) = builder.conversation_goals {
            prompt.push_str(&format!(
                "## Conversation Goals
{}

",
                conversation_goals
                    .into_iter()
                    .map(|like| format!("- {}", like))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if let Some(conversational_examples) = builder.conversational_examples {
            prompt.push_str(&format!(
                "## Conversational Examples

{}

",
                conversational_examples
                    .into_iter()
                    .enumerate()
                    .map(|(i, example)| format!(
                        "### Example {}\n```example\n{}\n```\n",
                        i + 1,
                        example
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if let Some(context) = builder.context {
            prompt.push_str(&format!(
                "## Context

{}

",
                context
                    .into_iter()
                    .enumerate()
                    .map(|(i, context)| format!(
                        "### Context {}\n```context\n{}\n```\n",
                        i + 1,
                        context
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if let Some(long_term_memory) = builder.long_term_memory {
            prompt.push_str(&format!(
                "## Long Term Memory
{}

",
                long_term_memory
                    .into_iter()
                    .enumerate()
                    .map(|(i, long_term_memory)| format!(
                        "### Memory {}\n{}",
                        i + 1,
                        long_term_memory
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        if let Some(user_about) = builder.user_about {
            prompt.push_str(&format!(
                "## {}'s About
    {}

    ",
                builder.user_name, user_about
            ));
        }

        //         if let Some(timezone) = builder.timezone {
        //             prompt.push_str(&format!(
        //                 "## System Time
        // {}
        //
        // ",
        //                 chrono::Utc::now()
        //                     .with_timezone(&timezone)
        //                     .format("%Y-%m-%d %H:%M:%S %z")
        //             ));
        //         }

        Self {
            inner: prompt,
            builder: builder_clone,
        }
    }

    pub fn into_builder(self) -> SystemPromptBuilder {
        self.builder
    }

    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }

    pub fn rebuild(self) -> Self {
        return Self::new(self.builder);
    }
}

impl Deref for SystemPrompt {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct SystemPromptBuilder {
    chatbot_name: String,
    user_name: String,
    about: String,
    tone: Option<String>,
    age: Option<String>,
    likes: Option<Vec<String>>,
    dislikes: Option<Vec<String>>,
    history: Option<String>,
    conversation_goals: Option<Vec<String>>,
    conversational_examples: Option<Vec<String>>,
    context: Option<Vec<String>>,
    long_term_memory: Option<Vec<String>>,
    user_about: Option<String>,
    timezone: Option<Tz>,
}
impl SystemPromptBuilder {
    pub fn new(chatbot_name: String, user_name: String, about: String) -> Self {
        Self {
            chatbot_name,
            user_name,
            about,
            tone: None,
            age: None,
            likes: None,
            dislikes: None,
            history: None,
            conversation_goals: None,
            conversational_examples: None,
            context: None,
            long_term_memory: None,
            user_about: None,
            timezone: None,
        }
    }

    pub fn tone(mut self, tone: String) -> Self {
        self.tone = Some(tone);
        self
    }

    pub fn age(mut self, age: String) -> Self {
        self.age = Some(age);
        self
    }

    pub fn add_like(mut self, like: String) -> Self {
        if let Some(likes) = &mut self.likes {
            likes.push(like);
        } else {
            self.likes = Some(vec![like]);
        }
        self
    }
    pub fn add_likes(mut self, new_likes: Vec<String>) -> Self {
        if let Some(likes) = &mut self.likes {
            likes.extend(new_likes);
        } else {
            self.likes = Some(new_likes);
        }
        self
    }

    pub fn add_dislike(mut self, dislike: String) -> Self {
        if let Some(dislikes) = &mut self.dislikes {
            dislikes.push(dislike);
        } else {
            self.dislikes = Some(vec![dislike]);
        }
        self
    }
    pub fn add_dislikes(mut self, new_dislikes: Vec<String>) -> Self {
        if let Some(dislikes) = &mut self.dislikes {
            dislikes.extend(new_dislikes);
        } else {
            self.dislikes = Some(new_dislikes);
        }
        self
    }

    pub fn history(mut self, history: String) -> Self {
        self.history = Some(history);
        self
    }

    pub fn add_conversational_goal(mut self, conversation_goal: String) -> Self {
        if let Some(conversation_goals) = &mut self.conversation_goals {
            conversation_goals.push(conversation_goal);
        } else {
            self.conversation_goals = Some(vec![conversation_goal]);
        }
        self
    }
    pub fn add_conversational_goals(mut self, new_conversation_goals: Vec<String>) -> Self {
        if let Some(conversation_goals) = &mut self.conversation_goals {
            conversation_goals.extend(new_conversation_goals);
        } else {
            self.conversation_goals = Some(new_conversation_goals);
        }
        self
    }

    pub fn add_conversational_example(mut self, conversational_example: String) -> Self {
        if let Some(conversational_examples) = &mut self.conversational_examples {
            conversational_examples.push(conversational_example);
        } else {
            self.conversational_examples = Some(vec![conversational_example]);
        }
        self
    }
    pub fn add_conversational_examples(mut self, new_conversational_examples: Vec<String>) -> Self {
        if let Some(conversational_examples) = &mut self.conversational_examples {
            conversational_examples.extend(new_conversational_examples);
        } else {
            self.conversational_examples = Some(new_conversational_examples);
        }
        self
    }

    pub fn add_context(mut self, new_context: String) -> Self {
        if let Some(context) = &mut self.context {
            context.push(new_context);
        } else {
            self.context = Some(vec![new_context]);
        }
        self
    }
    pub fn add_contexts(mut self, new_contexts: Vec<String>) -> Self {
        if let Some(context) = &mut self.context {
            context.extend(new_contexts);
        } else {
            self.context = Some(new_contexts);
        }
        self
    }

    pub fn add_long_term_memory(mut self, new_long_term_memory: String) -> Self {
        if let Some(long_term_memory) = &mut self.long_term_memory {
            long_term_memory.push(new_long_term_memory);
        } else {
            self.long_term_memory = Some(vec![new_long_term_memory]);
        }
        self
    }
    pub fn add_long_term_memories(mut self, new_long_term_memory: Vec<String>) -> Self {
        if let Some(long_term_memory) = &mut self.long_term_memory {
            long_term_memory.extend(new_long_term_memory);
        } else {
            self.long_term_memory = Some(new_long_term_memory);
        }
        self
    }

    pub fn user_about(mut self, user_about: String) -> Self {
        self.user_about = Some(user_about);
        self
    }

    pub fn timezone(mut self, timezone: Tz) -> Self {
        self.timezone = Some(timezone);
        self
    }

    pub fn build(mut self) -> (SystemPrompt, String) {
        let time = if let Some(timezone) = self.timezone {
            chrono::Utc::now()
                .with_timezone(&timezone)
                .format("%Y-%m-%d %H:%M:%S %z")
        } else {
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S %z")
        }
        .to_string();

        if let Some(tone) = self.tone {
            self.tone = Some(
                tone.replace("{user}", &self.user_name)
                    .replace("{bot}", &self.chatbot_name)
                    .replace("{time}", &time),
            );
        }

        if let Some(age) = self.age {
            self.age = Some(
                age.replace("{user}", &self.user_name)
                    .replace("{bot}", &self.chatbot_name)
                    .replace("{time}", &time),
            );
        }

        if let Some(likes) = self.likes {
            self.likes = Some(
                likes
                    .into_iter()
                    .map(|like| {
                        like.replace("{user}", &self.user_name)
                            .replace("{bot}", &self.chatbot_name)
                            .replace("{time}", &time)
                    })
                    .collect::<Vec<_>>(),
            );
        }

        if let Some(dislikes) = self.dislikes {
            self.dislikes = Some(
                dislikes
                    .into_iter()
                    .map(|like| {
                        like.replace("{user}", &self.user_name)
                            .replace("{bot}", &self.chatbot_name)
                            .replace("{time}", &time)
                    })
                    .collect::<Vec<_>>(),
            );
        }

        if let Some(history) = self.history {
            self.history = Some(
                history
                    .replace("{user}", &self.user_name)
                    .replace("{bot}", &self.chatbot_name)
                    .replace("{time}", &time),
            );
        }

        if let Some(conversation_goals) = self.conversation_goals {
            self.conversation_goals = Some(
                conversation_goals
                    .into_iter()
                    .map(|like| {
                        like.replace("{user}", &self.user_name)
                            .replace("{bot}", &self.chatbot_name)
                            .replace("{time}", &time)
                    })
                    .collect::<Vec<_>>(),
            );
        }

        if let Some(conversational_examples) = self.conversational_examples {
            self.conversational_examples = Some(
                conversational_examples
                    .into_iter()
                    .enumerate()
                    .map(|(i, example)| {
                        example
                            .replace("{user}", &self.user_name)
                            .replace("{bot}", &self.chatbot_name)
                            .replace("{time}", &time)
                    })
                    .collect::<Vec<_>>(),
            );
        }

        if let Some(context) = self.context {
            self.context = Some(
                context
                    .into_iter()
                    .enumerate()
                    .map(|(i, context)| {
                        context
                            .replace("{user}", &self.user_name)
                            .replace("{bot}", &self.chatbot_name)
                            .replace("{time}", &time)
                    })
                    .collect::<Vec<_>>(),
            );
        }

        if let Some(long_term_memory) = self.long_term_memory {
            self.long_term_memory = Some(
                long_term_memory
                    .into_iter()
                    .enumerate()
                    .map(|(i, long_term_memory)| {
                        long_term_memory
                            .replace("{user}", &self.user_name)
                            .replace("{bot}", &self.chatbot_name)
                            .replace("{time}", &time)
                    })
                    .collect::<Vec<_>>(),
            );
        }

        if let Some(user_about) = self.user_about {
            self.user_about = Some(
                user_about
                    .replace("{user}", &self.user_name)
                    .replace("{bot}", &self.chatbot_name)
                    .replace("{time}", &time),
            );
        }

        (SystemPrompt::new(self), time)
    }
}
