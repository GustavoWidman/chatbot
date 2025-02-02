use std::{collections::HashMap, fmt::format};

use crate::{chat::CompletionMessage, config::structure::RetrievalConfig};

use genai::chat::ChatMessage;
use openai_api_rs::v1::{
    api::OpenAIClient,
    chat_completion::{self, ChatCompletionRequest, Tool},
    embedding::EmbeddingRequest,
    types,
};
use serde::{Deserialize, Serialize};
use serenity::all::UserId;

use super::storage::MemoryStorage;

pub struct RetrievalSettings {
    pub model: String,
    pub temperature: f64,
    pub top_p: f64,
    pub max_res_tokens: i64,
    pub vector_size: i32,
    pub user_id: UserId,
}

#[derive(Deserialize, Serialize)]
struct Query {
    query: String,
}

pub struct RetrievalClient {
    pub client: OpenAIClient,
    pub settings: RetrievalSettings,
    pub storage: MemoryStorage,
    pub tool: Tool,
    // pub prompt: String,
    // pub storage: MemoryStorage,
}

impl RetrievalClient {
    pub fn new(config: &RetrievalConfig, user_id: UserId) -> Self {
        let api_key = config.gemini_key.clone();

        let client = OpenAIClient::builder()
            .with_endpoint("https://generativelanguage.googleapis.com/v1beta/openai")
            // .with_endpoint("http://127.0.0.1:8080")
            // .with_proxy("http://127.0.0.1:8080")
            .with_api_key(api_key.clone())
            .build()
            // unwrap is safe because we've set both api key and url,
            // so it's not searching the values in the env
            .unwrap();

        let mut properties = HashMap::new();
        properties.insert(
            "query".to_string(),
            Box::new(types::JSONSchemaDefine {
                schema_type: Some(types::JSONSchemaType::String),
                description: Some("The query to perform on the memory archive".to_string()),
                ..Default::default()
            }),
        );

        let tool = chat_completion::Tool {
            r#type: chat_completion::ToolType::Function,
            function: types::Function {
                name: String::from("memory_recall"),
                description: Some(String::from(
                    "Searches long-term memory on a vector database (qdrant) using cosine similarity. Used for recalling facts, user preferences, or historical context. Always specify both query and threshold. If you believe no memory has to be recalled, do not specify a query. The query should be similar to how you would look up things in a search engine, for example: 'shirt color' or 'favorite movie'. The query should be a phrase, not a sentence, you aren't asking the database, you're querying it.",
                )),
                parameters: types::FunctionParameters {
                    schema_type: types::JSONSchemaType::Object,
                    properties: Some(properties),
                    // required: Some(vec![String::from("query")]),
                    required: None,
                },
            },
        };

        Self {
            client,
            settings: RetrievalSettings {
                model: config.model.clone(),
                temperature: config.temperature.unwrap_or(1.0),
                top_p: config.top_p.unwrap_or(0.95),
                max_res_tokens: config.max_tokens.unwrap_or(1024),
                vector_size: config.vector_size.unwrap_or(256) as i32,
                user_id,
            },
            storage: MemoryStorage::new(config),
            tool,
            // tool,
        }
    }

    async fn embed(&self, context: String) -> anyhow::Result<Vec<f32>> {
        // text-embedding-004

        let mut req = EmbeddingRequest::new("text-embedding-004".to_string(), vec![context]);
        req.dimensions = Some(self.settings.vector_size);

        let result = self.client.embedding(req).await?;

        Ok(result
            .data
            .into_iter()
            .next()
            .ok_or(anyhow::anyhow!("No embedding"))?
            .embedding)
    }

    async fn search(&self, query: String) -> anyhow::Result<Vec<String>> {
        let embedding = self.embed(query).await?;

        self.storage.search(embedding, self.settings.user_id).await
    }

    pub async fn store(&self, context: Vec<CompletionMessage>) -> anyhow::Result<()> {
        let summary = self.summarize(context.clone()).await?;

        println!("summary: {}", summary);

        let embedding = self.embed(summary.clone()).await?;

        self.storage
            .store(summary, embedding, self.settings.user_id)
            .await
    }

    pub async fn recall(&self, context: Vec<CompletionMessage>) -> Option<Vec<String>> {
        let mut ctx = vec![CompletionMessage {
            role: "system".to_string(),
            content: "You are a assistant that can recall memories from long-term memory. You can only recall memories that are relevant to the current conversation. You cannot recall memories that are irrelevant to the current conversation. If you believe no memory has to be recalled, do not specify a query, simply return an empty field. If you believe a memory has to be recalled, specify a query. If you believe a memory has to be recalled, but do not know how to properly query it at the moment, do not specify a query. Attempt to recall memories that are long-termed (like events, people, places, etc.). Do not recall memories that are short-termed (like user input, current state of mind, etc.).".to_string(),
        }];

        ctx.extend(
            context
                .into_iter()
                .filter(|msg| msg.role != "system")
                .collect::<Vec<_>>(),
        );

        let req = ChatCompletionRequest::new(
            self.settings.model.clone(),
            ctx.into_iter().map(|msg| msg.into()).collect(),
        )
        .tools(vec![self.tool.clone()])
        // .max_tokens(1)
        .temperature(0.0)
        .top_p(0.95)
        .tool_choice(chat_completion::ToolChoiceType::Required);

        let result = self
            .client
            .chat_completion(req)
            .await
            .map_err(|e| {
                println!("error: {:?}", e);
                e
            })
            .ok()?;

        let first_choice = result.choices.into_iter().next()?;

        if let Some(tool_calls) = first_choice.message.tool_calls {
            println!("tool calls: {:?}", tool_calls);

            for tool_call in tool_calls {
                if tool_call.function.name == Some("memory_recall".to_owned()) {
                    let arguments = tool_call.function.arguments?;
                    let query: String = serde_json::from_str::<Query>(&arguments).ok()?.query;

                    return self.search(query).await.ok();
                }
            }
        };

        None
    }

    async fn summarize(&self, context: Vec<CompletionMessage>) -> anyhow::Result<String> {
        let mut ctx = vec![CompletionMessage {
            role: "system".to_string(),
            content: "You are a assistant that will take the user's input and summarize it to it's best, putting important discoveries/revelations and new information into bullet points. You will not repeat yourself, and you will not use the same bullet points more than once. The summarized input will be inserted into a long term memory storage, so only note the information you believe to be relevant to be eventually recalled in future conversations (new interests, new information, personality revelations, etc.). Do not state things that are short-termed (the user is going to the bathroom, the user is crying). Only state things that are long-termed (the user likes bananas, the user asked you out on a date).".to_string(),
        }, CompletionMessage {
            role: "user".to_string(),
            content: context
                .into_iter()
                .filter(|msg| msg.role != "system")
                .map(|msg| format!("{}: {}\n---\n", msg.role, msg.content))
                .collect::<Vec<String>>()
                .join("")
                .trim_end_matches("\n---\n")
                .to_owned(),
        }];

        let req = ChatCompletionRequest::new(
            self.settings.model.clone(),
            ctx.into_iter().map(|msg| msg.into()).collect(),
        )
        .tools(vec![self.tool.clone()])
        // .max_tokens(1)
        .temperature(0.0)
        .top_p(0.95);

        let result = self.client.chat_completion(req).await?;

        let first_choice = result
            .choices
            .into_iter()
            .next()
            .ok_or(anyhow::anyhow!("No choices found in response"))?;

        return first_choice
            .message
            .content
            .ok_or(anyhow::anyhow!("No content"));
    }
}
