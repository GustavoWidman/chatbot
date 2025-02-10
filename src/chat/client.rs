use std::collections::HashMap;

use anyhow::Result;

use openai_api_rs::v1::{
    api::OpenAIClient,
    assistant,
    chat_completion::{self, ChatCompletionRequest, Tool},
    embedding::EmbeddingRequest,
    types,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serenity::all::UserId;

use crate::{archive::storage::MemoryStorage, config::structure::LLMConfig};

use super::context::CompletionMessage;

pub struct ClientSettings {
    pub model: String,
    pub embedding_model: String,
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

pub struct ChatClient {
    pub client: OpenAIClient,
    pub settings: ClientSettings,
    pub storage: MemoryStorage,
    pub tool: Tool,
}

impl ChatClient {
    pub fn new(config: &LLMConfig, user_id: UserId) -> Self {
        let api_key = config.api_key.clone();

        let client = OpenAIClient::builder()
            .with_api_key(api_key.clone())
            .with_endpoint(
                config
                    .custom_url
                    .as_ref()
                    .map_or("https://api.openai.com/v1", |v| v),
            )
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
                    "Use to recall facts, preferences, or historical context, such as things that the user told you in the past, to fill in gaps in current memory (context). The query should be similar to how you would look up things in a search engine (like a google search), for example: 'shirt color' or 'favorite movie'. The query should be a phrase, not a sentence, you aren't asking the database, you're querying it, so for example 'what shirt did james wear last week' is not a valid query, whereas 'shirt' or 'james shirt' is.",
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
            settings: ClientSettings {
                model: config.model.clone(),
                embedding_model: config.embedding_model.clone(),
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

    pub async fn prompt(
        &self,
        context: Vec<CompletionMessage>,
        recall: bool,
    ) -> Result<PromptResult> {
        let req = ChatCompletionRequest::new(
            self.settings.model.clone(),
            context.into_iter().map(|msg| msg.into()).collect(),
        )
        .tools(match recall {
            true => vec![self.tool.clone()],
            false => vec![],
        })
        .max_tokens(self.settings.max_res_tokens)
        .temperature(self.settings.temperature)
        .presence_penalty(1.0)
        .top_p(self.settings.top_p)
        .tool_choice(match recall {
            true => chat_completion::ToolChoiceType::Auto,
            false => chat_completion::ToolChoiceType::None,
        });

        println!("there is a request");

        let result = self.client.chat_completion(req).await.map_err(|e| {
            println!("error: {:?}", e);
            e
        })?;

        let first_choice = result
            .choices
            .into_iter()
            .next()
            .ok_or(anyhow::anyhow!("No choices found in response"))?;

        if let Some(tool_calls) = first_choice.message.tool_calls {
            println!("tool calls: {:?}", tool_calls);

            for tool_call in tool_calls {
                if tool_call.function.name == Some("memory_recall".to_owned()) {
                    let arguments = tool_call
                        .function
                        .arguments
                        .ok_or(anyhow::anyhow!("No arguments found in tool call"))?;

                    let query: String = serde_json::from_str::<Query>(&arguments)?.query;

                    return Ok(PromptResult::MemoryRecall(self.search(query).await?));
                }
            }
        };

        println!("there is a response");

        let msg = first_choice
            .message
            .content
            .ok_or(anyhow::anyhow!("No content"))?;

        println!("there is content");

        let regex = Regex::new(r"```.*").unwrap();
        let content = regex.replace_all(&msg, "").to_string();

        // tokio::time::sleep(std::time::Duration::from_secs(5)).await; // simulate API call latency
        Ok(PromptResult::Message(CompletionMessage {
            role: "assistant".to_string(),
            content,
        }))
    }

    async fn search(&self, query: String) -> anyhow::Result<Vec<String>> {
        let embedding = self.embed(query).await?;

        self.storage.search(embedding, self.settings.user_id).await
    }

    async fn embed(&self, context: String) -> anyhow::Result<Vec<f32>> {
        let mut req = EmbeddingRequest::new(self.settings.embedding_model.clone(), vec![context]);
        req.dimensions = Some(self.settings.vector_size);

        let result = self.client.embedding(req).await?;

        Ok(result
            .data
            .into_iter()
            .next()
            .ok_or(anyhow::anyhow!("No embedding"))?
            .embedding)
    }

    pub async fn store(
        &self,
        context: Vec<CompletionMessage>,
        user_name: String,
        assistant_name: String,
    ) -> anyhow::Result<()> {
        let summary = self
            .summarize(context.clone(), user_name, assistant_name)
            .await?;

        println!("summary: {}", summary);

        let embedding = self.embed(summary.clone()).await?;

        self.storage
            .store(summary, embedding, self.settings.user_id)
            .await
    }

    async fn summarize(
        &self,
        context: Vec<CompletionMessage>,
        user_name: String,
        assistant_name: String,
    ) -> anyhow::Result<String> {
        let ctx = vec![CompletionMessage {
            role: "system".to_string(),
            content: "You are a assistant that will take the user's input and summarize it to it's best, yet you will be incredibly detailed in what you are writing, putting important discoveries/revelations and new information into bullet points, the more bullet points the better. You will not repeat yourself, and you will not use the same bullet points more than once. The summarized input will be inserted into a long term memory storage, so only note the information you believe to be relevant to be eventually recalled in future conversations (new interests, new information, personality revelations, etc.). Do not state things that are short-termed (the user is going to the bathroom, the user is crying). Only state things that are long-termed (the user likes bananas, the user asked you out on a date). Your response should only contain the bullet points, and nothing else.".to_string(),
        }, CompletionMessage {
            role: "user".to_string(),
            content: context
                .into_iter()
                .filter(|msg| msg.role != "system" && msg.role != "tool")
                .map(|msg| format!("{}: {}\n---\n", match msg.role.as_str() {
                    "user" => user_name.clone(),
                    "assistant" => assistant_name.clone(),
                    _ => msg.role,
                }, msg.content))
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

pub enum PromptResult {
    Message(CompletionMessage),
    MemoryRecall(Vec<String>),
}
