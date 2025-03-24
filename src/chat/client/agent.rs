use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use regex::Regex;
use rig::{
    OneOrMany,
    completion::{CompletionRequest, ToolDefinition},
    embeddings::Embedding,
    message::{AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent},
    tool::{Tool, ToolDyn},
};
use rig_dyn::{CompletionModel, EmbeddingModel};
use serde_json::json;
use serenity::all::UserId;

use crate::{
    chat::{
        ChatMessage,
        archive::storage::{Memory, MemoryStorage},
        context::{MessageRole, UserPrompt},
    },
    config::structure::LLMConfig,
};

use super::tools;

pub struct CompletionAgentSettings {
    user_name: String,
    assistant_name: String,
}

pub struct CompletionAgent {
    completion_model: Arc<Box<dyn CompletionModel>>,
    embedding_model: Arc<Box<dyn EmbeddingModel>>,
    memory_storage: Arc<MemoryStorage>,
    tools: HashMap<String, Box<dyn ToolDyn>>,
    user_id: UserId,
    config: LLMConfig,
    settings: CompletionAgentSettings,
}

impl CompletionAgent {
    pub async fn new(
        config: LLMConfig,
        user_id: UserId,
        user_name: String,
        assistant_name: String,
    ) -> anyhow::Result<Self> {
        let client = config.completion.provider.client(
            &config.completion.api_key,
            config.completion.custom_url.as_deref(),
        )?;
        let completion_model = Arc::new(client.completion_model(&config.completion.model).await);

        let embedding_client = config
            .embedding
            .provider
            .map(|provider| {
                provider.client(
                    config
                        .embedding
                        .api_key
                        .as_deref()
                        .unwrap_or(&config.completion.api_key),
                    config.embedding.custom_url.as_deref(),
                )
            })
            .unwrap_or(Ok(client))?;

        let embedding_model = match config.embedding.vector_size {
            Some(vector_size) => {
                let client = embedding_client
                    .embedding_model_with_ndims(&config.embedding.model, vector_size, None)
                    .await
                    .ok_or(anyhow!("failed to create embedding model"))?;

                Arc::new(client)
            }
            None => {
                let client = embedding_client
                    .embedding_model(&config.embedding.model, None)
                    .await
                    .ok_or(anyhow!("failed to create embedding model"))?;

                Arc::new(client)
            }
        };

        // test embedding model and obtain true vector size
        let vector_size = embedding_model.embed_text("a").await?.vec.len() as u64;

        log::info!("vector size: {}", vector_size);

        let memory_storage = Arc::new(MemoryStorage::new(&config, vector_size));
        memory_storage.health_check(user_id).await?;

        let recall = tools::MemoryRecall::new(
            embedding_model.clone(),
            memory_storage.clone(),
            user_id,
            user_name.clone(),
            assistant_name.clone(),
        );
        let store = tools::MemoryStore::new(
            embedding_model.clone(),
            memory_storage.clone(),
            user_id,
            user_name.clone(),
            assistant_name.clone(),
        );

        let mut tools: HashMap<String, Box<dyn ToolDyn>> = HashMap::new();
        tools.insert(tools::MemoryRecall::NAME.to_string(), Box::new(recall));
        tools.insert(tools::MemoryStore::NAME.to_string(), Box::new(store));

        log::info!("engine initialized successfully for {user_id}, health checks passed");

        Ok(Self {
            completion_model,
            embedding_model,
            memory_storage,
            tools,
            user_id,
            config,
            settings: CompletionAgentSettings {
                user_name,
                assistant_name,
            },
        })
    }

    pub async fn completion(
        &self,
        mut prompt: &mut UserPrompt,
        mut system_prompt: String,
        context: Vec<ChatMessage>,
    ) -> anyhow::Result<CompletionResult> {
        //? traditional RAG
        self.rag_recall(&mut prompt).await?;
        // let recalled: Vec<String> = vec![]; // todo testing

        log::trace!("User prompt: {prompt:?}");
        for (i, message) in context.iter().enumerate() {
            log::trace!("Context {i}: {message:?}");
        }

        // let recent = self
        //     .memory_storage
        //     .find_recent(self.user_id, 5, None)
        //     .await?;

        // log::info!("recent memories: {:?}", recent);

        //? rag by tool (incentive)
        let use_tools = self.config.use_tools.unwrap_or(true);
        let tools = if use_tools {
            system_prompt.push_str("
## Tool Usage
- Actively try to utilize the memory_store tool to store important information that you'd like to recall later in the long term memory storage, preferably in bullet points. Do not mention the usage of this tool to the user, just use it when needed.
- Actively try to utilize the memory_recall tool to recall information from previous messages and conversations you are not currently aware of. Do not mention this usage of the tool to the user, just use it when needed. If you believe a memory has already been recalled by the user (as seen in the \"relevant_memories\" section), choose not to recall it again.

");
            self.tool_definitions().await
        } else {
            vec![]
        };

        if self.config.completion.reason.unwrap_or(false)
            || self.config.completion.fake_reason.unwrap_or(false)
        {
            system_prompt.push_str("
## Reasoning Protocol

When you receive a prompt, always take time to think carefully before responding. Use <think> tags to show your reasoning process. This thinking process should:

1. Analyze the user's request and identify key elements
2. Consider any compliance requirements or restrictions mentioned in the system prompt
3. Review any \"penalty\" adjustments that might apply
4. Process any memory recall instructions
5. Consider the appropriate roleplay response
6. Plan your final response to ensure it meets all requirements

For example:

<think>
- User wants me to [specific request]
- Checking compliance requirements: [note relevant restrictions]
- Considering penalty conditions: [note any potential penalties]
- Reviewing memory instructions: [note any recall requirements]
- Considering roleplay context: [note character perspective]
- Planning response that satisfies all constraints while maintaining character
</think>

After this reasoning step, provide your in-character response. This reasoning process is mandatory for every prompt you receive, ensuring thoughtful, compliant, and in-character interactions.
");
        }

        let mut additional_params: HashMap<String, toml::Value> = self
            .config
            .additional_params
            .clone()
            .unwrap_or(HashMap::new());
        if self.config.completion.reason.unwrap_or(false) {
            additional_params.insert(
                "reasoning".to_string(),
                toml::Value::Table(toml::Table::new()),
            );
        }
        log::trace!("additional_params: {:?}", json!(additional_params));

        let request = CompletionRequest {
            additional_params: Some(json!(additional_params)),
            chat_history: context.into_iter().map(|x| x.into()).collect(),
            documents: vec![],
            max_tokens: self.config.completion.max_tokens,
            preamble: Some(system_prompt),
            // preamble: None, // todo testing
            temperature: self.config.completion.temperature,
            tools,
            prompt: prompt.clone().try_into()?,
        };

        let response = self.completion_model.completion(request).await?;

        match response.first() {
            rig::message::AssistantContent::Text(mut text) => {
                log::trace!("Original response:\n{:?}", text.text);

                if self.config.force_lowercase.unwrap_or(false) {
                    text.text = text.text.to_lowercase();
                }

                // get rid of CoT
                let regex = Regex::new(
                    r"<(?:think|reasoning)>((?:.|\n)*?)<\/(?:think|reasoning)>(?:\n*)?",
                )?;
                let matches: Vec<_> = regex.captures_iter(&text.text).collect();
                for cap in &matches {
                    if let Some(thought) = cap.get(1) {
                        log::trace!("Extracted thought process:\n{}", thought.as_str());
                    }
                }
                text.text = regex.replace_all(&text.text, "").to_string();

                // get rid of weird artifacts
                // 1 or more space before double newline -> double newline
                let regex = Regex::new(r" +\n\n")?;
                text.text = regex.replace_all(&text.text, "\n\n").to_string();
                // 2 or more spaces -> single space
                let regex = Regex::new(r" {2,}")?;
                text.text = regex.replace_all(&text.text, " ").to_string();
                // 3 or more newlines -> 2 newlines
                let regex = Regex::new(r"\n\n\n+")?;
                text.text = regex.replace_all(&text.text, "\n\n").to_string();
                // get rid of "\boxed{TEXT}" if present
                // if text.text.starts_with("\\boxed{") && text.text.ends_with("}") {
                //     text.text = text.text[7..text.text.len() - 1].to_string();
                // }

                Ok(CompletionResult::Message(Message::Assistant {
                    content: OneOrMany::one(AssistantContent::text(&text.text)),
                }))
            }
            rig::message::AssistantContent::ToolCall(tool_call) => {
                let tool_call_msg = AssistantContent::ToolCall(tool_call.clone());

                let ToolCall {
                    id,
                    function: ToolFunction { name, arguments },
                } = tool_call;

                let result = self.call_tool(&name, arguments.to_string()).await?;
                let tool_result: ToolResult = (name, result).into();

                Ok(CompletionResult::Tool((
                    Message::Assistant {
                        content: OneOrMany::one(tool_call_msg),
                    },
                    Message::User {
                        content: OneOrMany::one(UserContent::tool_result(
                            id,
                            OneOrMany::one(tool_result.into()),
                        )),
                    },
                )))
            }
        }
    }

    async fn tool_definitions(&self) -> Vec<ToolDefinition> {
        let mut definitions = Vec::new();
        for tool in self.tools.values() {
            definitions.push(tool.definition(String::new()).await);
        }

        definitions
    }

    async fn call_tool(&self, tool_name: &str, args: String) -> anyhow::Result<String> {
        if let Some(tool) = self.tools.get(tool_name) {
            Ok(tool.call(args).await?)
        } else {
            Err(anyhow::anyhow!("tool not found: {}", tool_name))
        }
    }

    pub async fn rag_recall(&self, prompt: &mut UserPrompt) -> anyhow::Result<()> {
        let message = if let Some(content) = &prompt.content {
            content
        } else {
            return Ok(());
        };

        log::trace!("RAG query message: {message}");

        let vec = self
            .embedding_model
            .embed_text(&message)
            .await?
            .vec
            .into_iter()
            .map(|x| x as f32)
            .collect::<Vec<f32>>();

        // todo change limit here
        let recalled = self
            .memory_storage
            .search(vec, self.user_id, 5, None)
            .await?
            .iter_mut()
            .map(|x| {
                x.content
                    .replace("<user>", &self.settings.user_name)
                    .replace("<assistant>", &self.settings.assistant_name)
            })
            .collect::<Vec<_>>();

        if !recalled.is_empty() {
            log::info!("RAGged {} memories", recalled.len());
            prompt.relevant_memories.extend(recalled);
        }

        Ok(())
    }

    pub async fn store(
        &self,
        context: Vec<ChatMessage>,
        user_name: &str,
        assistant_name: &str,
    ) -> anyhow::Result<()> {
        log::info!("summarizing {} messages", context.len());

        let summary = self.summarize(context, user_name, assistant_name).await?;

        log::trace!("summarized:\n{}", summary);

        let Embedding { document, vec } = self.embedding_model.embed_text(&summary).await?;
        let vec = vec.into_iter().map(|x| x as f32).collect::<Vec<f32>>();

        self.memory_storage
            .store(Memory::new(document), vec, self.user_id)
            .await
    }

    async fn summarize(
        &self,
        context: Vec<ChatMessage>,
        user_name: &str,
        assistant_name: &str,
    ) -> anyhow::Result<String> {
        let preamble = "# Summarization Assistant
You are a specialized summarization assistant that extracts only the most significant, long-term valuable information from conversations. Your purpose is to identify and record information that should be remembered for future interactions.

## Task
Extract only information that meets ALL of these criteria:
- Reveals persistent user preferences, interests, values, or traits
- Has potential relevance beyond the immediate conversation
- Would naturally be remembered by a human conversation partner

## Format
- Provide concise bullet points of key information
- Use consistent, retrievable phrasing
- Prioritize specificity over generality
- Include source context when relevant (e.g., \"When discussing travel, mentioned...\")
- Utilize the <user> and <assistant> tags for user and assistant placeholders

## Avoid
- Temporary states or short-term information (e.g., \"user is going to the store\", \"user is feeling tired today\")
- Obvious or common knowledge
- Conversational mechanics (e.g., \"user asked for help with...\")
- Speculation about the user
- Summarizing the entire conversation
- Creating empty summaries when no meaningful information is present

## Examples

The following are a series of good and poor examples of summaries. You should attempt to apply the same approach to your own summaries, returning only good extractions and ignoring poor extractions.

### Good Example #1

<user> lives in Toronto and works as a software engineer.

### Poor Example #1

User is currently at home

### Good Example #2

<user> has a 5-year-old daughter named Emma who loves dinosaurs.

### Poor Example #2

<user> needs to pick up their child from school today


### Good Example #3

<assistant> mentioned severe peanut allergy multiple times.

### Poor Example #3

<assistant> is hungry".to_string();

        let prompt = Message::user(
            context
                .into_iter()
                .filter_map(|msg| {
                    let role = msg.role();

                    let content = match role {
                        MessageRole::User => {
                            let user_prompt: UserPrompt = TryFrom::try_from(msg)
                                .map_err(|why| {
                                    log::warn!("failed to deserialize user prompt: {why:?}");
                                })
                                .ok()?;

                            user_prompt.content.or(user_prompt.system_note)
                        }
                        MessageRole::Assistant => msg.content(),
                    }?;

                    Some(format!(
                        "{}: {}\n---\n",
                        match role {
                            MessageRole::User => user_name,
                            MessageRole::Assistant => assistant_name,
                        },
                        content
                    ))
                })
                .collect::<Vec<String>>()
                .join("")
                .trim_end_matches("\n---\n")
                .replace(user_name, "<user>")
                .replace(assistant_name, "<assistant>")
                .to_owned(),
        );

        log::trace!("Summarize prompt:\n{:?}", prompt);

        let request = CompletionRequest {
            // todo decide if i want this or not
            // additional_params: Some(json!({
            //     "top_p": 0.2,
            //     "frequency_penalty": 0.2,
            //     "presence_penalty": 0.0,
            // })),
            additional_params: None,
            chat_history: vec![],
            documents: vec![],
            max_tokens: Some(8192),
            preamble: Some(preamble),
            temperature: Some(0.2),
            tools: vec![],
            prompt,
        };

        let response = self.completion_model.completion(request).await?;

        if let AssistantContent::Text(message) = response.first() {
            return Ok(message.text);
        } else {
            return Err(anyhow::anyhow!("Invalid response"));
        }
    }
}
pub struct ToolResult(String, String);
impl From<(String, String)> for ToolResult {
    fn from(value: (String, String)) -> Self {
        Self(value.0, value.1)
    }
}
impl From<ToolResult> for ToolResultContent {
    fn from(val: ToolResult) -> Self {
        ToolResultContent::text(
            json!({
                "name": val.0,
                "result": val.1
            })
            .to_string(),
        )
    }
}

pub enum CompletionResult {
    /// Returns the message (assistant message)
    Message(Message),

    /// Returns the tool call and tool result (assistant and user messages)
    Tool((Message, Message)),
}
