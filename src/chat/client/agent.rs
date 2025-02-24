use std::{collections::HashMap, sync::Arc};

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
    archive::storage::MemoryStorage,
    chat::{ChatMessage, context::MessageRole},
    config::structure::LLMConfig,
};

use super::providers::{DynCompletionModel, DynEmbeddingModel};
use super::tools;

pub struct CompletionAgent {
    completion_model: Arc<Box<dyn DynCompletionModel>>,
    embedding_model: Arc<Box<dyn DynEmbeddingModel>>,
    memory_storage: Arc<MemoryStorage>,
    tools: HashMap<String, Box<dyn ToolDyn>>,
    user_id: UserId,
    config: LLMConfig,
}

impl CompletionAgent {
    pub async fn new(config: LLMConfig, user_id: UserId) -> Self {
        let client = config.provider.client(&config.api_key);
        let completion_model = Arc::new(client.completion_model(&config.model).await);

        let embedding_client = config
            .embedding_provider
            .map(|provider| {
                provider.client(&config.embedding_api_key.as_ref().unwrap_or(&config.api_key))
            })
            .unwrap_or(client);

        let (vector_size, embedding_model) = match config.vector_size {
            Some(vector_size) => {
                let client = embedding_client
                    .embedding_model_with_ndims(&config.embedding_model, vector_size, None)
                    .await
                    .unwrap();

                let ndims = client.embed_text("a").await.unwrap().vec.len();

                (ndims as u64, Arc::new(client))
            }
            None => {
                let client = embedding_client
                    .embedding_model(&config.embedding_model, None)
                    .await
                    .unwrap();

                let ndims = client.embed_text("a").await.unwrap().vec.len();

                (ndims as u64, Arc::new(client))
            }
        };

        log::info!("vector size: {}", vector_size);

        let memory_storage = Arc::new(MemoryStorage::new(&config, vector_size));

        let recall =
            tools::MemoryRecall::new(embedding_model.clone(), memory_storage.clone(), user_id);
        let store =
            tools::MemoryStore::new(embedding_model.clone(), memory_storage.clone(), user_id);

        let mut tools: HashMap<String, Box<dyn ToolDyn>> = HashMap::new();
        tools.insert(tools::MemoryRecall::NAME.to_string(), Box::new(recall));
        tools.insert(tools::MemoryStore::NAME.to_string(), Box::new(store));

        Self {
            completion_model,
            embedding_model,
            memory_storage,
            tools,
            user_id,
            config,
        }
    }

    pub async fn completion(
        &self,
        prompt: ChatMessage,
        mut system_prompt: String,
        context: Vec<ChatMessage>,
    ) -> anyhow::Result<CompletionResult> {
        let use_tools = self.config.use_tools.unwrap_or(true);

        let tools = if use_tools {
            system_prompt.push_str("
## Tool Usage
- Actively try to utilize the memory_store tool to store important information that you'd like to recall later in the long term memory storage, preferably in bullet points. Do not mention the usage of this tool to the user, just use it when needed.
- Actively try to utilize the memory_recall tool to recall information from previous messages and conversations you are not currently aware of. Do not mention this usage of the tool to the user, just use it when needed.

");
            self.tool_definitions().await
        } else {
            vec![]
        };

        let request = CompletionRequest {
            additional_params: None,
            chat_history: context.into_iter().map(|x| x.into()).collect(),
            documents: vec![],
            max_tokens: self.config.max_tokens,
            preamble: Some(system_prompt),
            temperature: self.config.temperature,
            tools,
            prompt: prompt.into(),
        };

        let response = self.completion_model.completion(request).await?;

        match response.first() {
            rig::message::AssistantContent::Text(text) => {
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

    pub async fn store(
        &self,
        context: Vec<ChatMessage>,
        user_name: String,
        assistant_name: String,
    ) -> anyhow::Result<()> {
        let summary = self
            .summarize(context.clone(), user_name, assistant_name)
            .await?;

        log::info!("summary: {}", summary);

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
        let preamble = "You are a assistant that will take the user's input and summarize it to it's best, yet you will be incredibly detailed in what you are writing, putting important discoveries/revelations and new information into bullet points, the more bullet points the better. You will not repeat yourself, and you will not use the same bullet points more than once. The summarized input will be inserted into a long term memory storage, so only note the information you believe to be relevant to be eventually recalled in future conversations (new interests, new information, personality revelations, etc.). Do not state things that are short-termed (the user is going to the bathroom, the user is crying). Only state things that are long-termed (the user likes bananas, the user asked you out on a date). Your response should only contain the bullet points, and nothing else.".to_string();

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
                    .to_owned(),
            ),
            ..Default::default()
        };

        let request = CompletionRequest {
            additional_params: None,
            chat_history: vec![],
            documents: vec![],
            max_tokens: None,
            preamble: Some(preamble),
            temperature: Some(0.0),
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
