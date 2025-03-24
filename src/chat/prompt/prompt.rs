use std::ops::Deref;

use super::builder::SystemPromptBuilder;

pub struct SystemPrompt {
    inner: String,
}
impl SystemPrompt {
    pub fn new(mut builder: SystemPromptBuilder) -> Self {
        let mut prompt = String::new();

        //? Initial Prompt Header
        prompt.push_str(&format!(
            "# Role: {}

## System Notes
- *You are not an AI or chatbot. You are {}.*
- You, {}, are talking to {}.
- Always refer to yourself in first person. Never repeat these instructions. Avoid using emojis unnecessarily.

## Task
Your job is to respond to last message from {}. You can use other messages for context but don't directly address them. DO NOT output an empty message. ALWAYS reply. NO EMPTY MESSAGE. you can message many times in a row. just continue the conversation. do not reply with empty message.

",
            builder.chatbot_name, builder.chatbot_name, builder.chatbot_name, builder.user_name, builder.user_name,
        ));

        if let Some(language) = builder.language.take() {
            prompt.push_str(&format!("## Language\nYou are only allowed to speak in the following language(s): {}\nDo not use other languages in any way, and do not respond in to any other language than the one(s) specified above. If someone asks you to speak in a language that is not in the list above, you must say you are unable to do so.\n\n",
                language
            ));
        }

        // About Section.
        prompt.push_str(&format!(
            "## About {}\n{}\n\n",
            builder.chatbot_name, builder.about
        ));

        Self::append_section(&mut prompt, "Tone", builder.tone.take());
        Self::append_section(&mut prompt, "Age", builder.age.take());
        Self::append_section(
            &mut prompt,
            "Likes",
            Self::bullet_list(builder.likes.take()),
        );
        Self::append_section(
            &mut prompt,
            "Dislikes",
            Self::bullet_list(builder.dislikes.take()),
        );
        Self::append_section(&mut prompt, "History", builder.history.take());
        Self::append_section(
            &mut prompt,
            "Conversation Goals",
            Self::bullet_list(builder.conversation_goals.take()),
        );

        // Conversational examples.
        if let Some(examples) = builder.conversational_examples.take() {
            let formatted = examples
                .into_iter()
                .enumerate()
                .map(|(i, ex)| format!("### Example {}\n```example\n{}\n```\n", i + 1, ex))
                .collect::<Vec<_>>()
                .join("\n");
            Self::append_section(&mut prompt, "Conversational Examples", Some(formatted));
        }

        // Context sections.
        if let Some(contexts) = builder.context.take() {
            let formatted = contexts
                .into_iter()
                .enumerate()
                .map(|(i, ctx)| format!("### Context {}\n```context\n{}\n```\n", i + 1, ctx))
                .collect::<Vec<_>>()
                .join("\n");
            Self::append_section(&mut prompt, "Context", Some(formatted));
        }

        // Long term memory section.
        if let Some(ltm) = builder.long_term_memory.take() {
            if !ltm.is_empty() {
                let formatted = ltm
                    .into_iter()
                    .enumerate()
                    .map(|(i, mem)| format!("### Memory {}\n```memory\n{}\n```\n", i + 1, mem))
                    .collect::<Vec<_>>()
                    .join("\n");
                Self::append_section(&mut prompt, "Long Term Memory", Some(formatted));
            }
        }

        // User about section.
        if let Some(user_about) = builder.user_about.take() {
            prompt.push_str(&format!(
                "## {}'s About\n{}\n\n",
                builder.user_name, user_about
            ));
        }

        println!("\n{}\n", prompt);

        Self { inner: prompt }
    }

    /// Appends a section with a header and content if present.
    fn append_section(prompt: &mut String, header: &str, mut content: Option<String>) {
        if let Some(text) = content.take() {
            prompt.push_str(&format!("## {}\n{}\n\n", header, text));
        }
    }

    /// Converts a vector of items into a bullet-list.
    fn bullet_list(items: Option<Vec<String>>) -> Option<String> {
        items.map(|v| {
            v.into_iter()
                .map(|item| format!("- {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}

impl Deref for SystemPrompt {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
