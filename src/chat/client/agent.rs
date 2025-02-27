use std::{collections::HashMap, sync::Arc};

use anyhow::anyhow;
use rig::{
    OneOrMany,
    completion::{CompletionRequest, ToolDefinition},
    embeddings::Embedding,
    message::{AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent},
    tool::{Tool, ToolDyn},
};
use serde_json::json;
use serenity::all::UserId;

use crate::{
    chat::{ChatMessage, archive::storage::MemoryStorage, context::MessageRole},
    config::structure::LLMConfig,
};

use super::providers::{DynCompletionModel, DynEmbeddingModel};
use super::tools;

pub struct CompletionAgentSettings {
    user_name: String,
    assistant_name: String,
}

pub struct CompletionAgent {
    completion_model: Arc<Box<dyn DynCompletionModel>>,
    embedding_model: Arc<Box<dyn DynEmbeddingModel>>,
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
        let client = config.provider.client(&config.api_key);
        let completion_model = Arc::new(client.completion_model(&config.model).await);

        let embedding_client = config
            .embedding_provider
            .map(|provider| {
                provider.client(&config.embedding_api_key.as_ref().unwrap_or(&config.api_key))
            })
            .unwrap_or(client);

        let embedding_model = match config.vector_size {
            Some(vector_size) => {
                let client = embedding_client
                    .embedding_model_with_ndims(&config.embedding_model, vector_size, None)
                    .await
                    .ok_or(anyhow!("failed to create embedding model"))?;

                Arc::new(client)
            }
            None => {
                let client = embedding_client
                    .embedding_model(&config.embedding_model, None)
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
        prompt: ChatMessage,
        mut system_prompt: String,
        context: Vec<ChatMessage>,
    ) -> anyhow::Result<CompletionResult> {
        //? traditional RAG
        let recalled = self.rag_recall(&prompt).await?;
        // let recalled: Vec<String> = vec![]; // todo testing
        if !recalled.is_empty() {
            log::debug!("RAGged {} memories", recalled.len());
            system_prompt.push_str("
## Spontaneously recalled memories

The following memories were recalled automatically from the long term memory storage based on the user's input:

");
            let formatted = recalled
                .into_iter()
                .enumerate()
                .map(|(i, mem)| {
                    log::trace!("recalled: {mem:?}");
                    format!(
                        "### Spontaneous Memory {}\n```memory\n{}\n```\n",
                        i + 1,
                        mem
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            system_prompt.push_str(&formatted);
        }

        //? rag by tool (incentive)
        let use_tools = self.config.use_tools.unwrap_or(true);
        let tools = if use_tools {
            system_prompt.push_str("
## Tool Usage
- Actively try to utilize the memory_store tool to store important information that you'd like to recall later in the long term memory storage, preferably in bullet points. Do not mention the usage of this tool to the user, just use it when needed.
- Actively try to utilize the memory_recall tool to recall information from previous messages and conversations you are not currently aware of. Do not mention this usage of the tool to the user, just use it when needed. If you believe a memory has already been recalled in the \"Spontaneously recalled memories\" section, choose not to recall it again.

");
            self.tool_definitions().await
        } else {
            vec![]
        };

        let request = CompletionRequest {
            additional_params: Some(json!({
                "top_p": self.config.top_p,
            })),
            chat_history: context.into_iter().map(|x| x.into()).collect(),
            documents: vec![],
            max_tokens: self.config.max_tokens,
            preamble: Some(system_prompt),
            // preamble: None, // todo testing
            temperature: self.config.temperature,
            tools,
            prompt: prompt.into(),
        };

        let response = self.completion_model.completion(request).await?;

        match response.first() {
            rig::message::AssistantContent::Text(mut text) => {
                if self.config.force_lowercase.unwrap_or(false) {
                    text.text = text.text.to_lowercase();
                }

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

                let tool_result: ToolResult = (
                    name.clone(),
                    self.call_tool(&name, arguments.to_string()).await?,
                )
                    .into();

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

    async fn rag_recall(&self, prompt: &ChatMessage) -> anyhow::Result<Vec<String>> {
        let message = prompt
            .content()
            .ok_or(anyhow::anyhow!("message does not have a content"))?;

        let vec = self
            .embedding_model
            .embed_text(&message)
            .await?
            .vec
            .into_iter()
            .map(|x| x as f32)
            .collect::<Vec<f32>>();

        // todo change limit here
        Ok(self
            .memory_storage
            .search(vec, self.user_id, 5, None)
            .await?
            .iter_mut()
            .map(|x| {
                x.replace("<user>", &self.settings.user_name)
                    .replace("<assistant>", &self.settings.assistant_name)
            })
            .collect())
    }

    pub async fn store(
        &self,
        context: Vec<ChatMessage>,
        user_name: String,
        assistant_name: String,
    ) -> anyhow::Result<()> {
        let summary = self
            .summarize(context.clone(), user_name, assistant_name)
            .await?;

        log::info!("summary:\n{}", summary);

        let Embedding { document, vec } = self.embedding_model.embed_text(&summary).await?;
        let vec = vec.into_iter().map(|x| x as f32).collect::<Vec<f32>>();

        self.memory_storage.store(document, vec, self.user_id).await
    }

    async fn summarize(
        &self,
        context: Vec<ChatMessage>,
        user_name: String,
        assistant_name: String,
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

### Example 1
```json
{
    \"good_extraction\": \"<user> lives in Toronto and works as a software engineer\".
    \"poor_extraction\": \"User is currently at home\"
}
```

### Example 2
```json
{
    \"good_extraction\": \"<user> has a 5-year-old daughter named Emma who loves dinosaurs\".
    \"poor_extraction\": \"<user> needs to pick up their child from school today\"
}
```

### Example 3
```json
{
    \"good_extraction\": \"<assistant> mentioned severe peanut allergy multiple times\".
    \"poor_extraction\": \"<assistant> is hungry\"
}
```".to_string();

        let prompt = ChatMessage {
            inner: Message::user(
                context
                    .into_iter()
                    .filter_map(|msg| {
                        let content = msg.content()?;
                        let role = msg.role();
                        Some(format!(
                            "{}: {}\n---\n",
                            match role {
                                MessageRole::User => user_name.clone(),
                                MessageRole::Assistant => assistant_name.clone(),
                            },
                            content
                        ))
                    })
                    .collect::<Vec<String>>()
                    .join("")
                    .trim_end_matches("\n---\n")
                    .replace(&user_name, "<user>")
                    .replace(&assistant_name, "<assistant>")
                    .to_owned(),
            ),
            ..Default::default()
        };

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
            max_tokens: None,
            preamble: Some(preamble),
            temperature: Some(0.2),
            tools: vec![],
            prompt: prompt.into(),
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
