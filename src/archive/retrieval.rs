use std::collections::HashMap;

use crate::config::structure::RetrievalConfig;

use anyhow::Result;
use genai::{
    Client, ClientConfig, ModelIden, ServiceTarget,
    adapter::AdapterKind,
    chat::{ChatMessage, ChatOptions, ChatRequest, Tool},
    resolver::{AuthData, AuthResolver, Endpoint, ModelMapper, ServiceTargetResolver},
};
use openai_api_rs::v1::{
    api::OpenAIClient,
    chat_completion::{self, ChatCompletionRequest},
    types,
};
use regex::Regex;
// use rig::providers::openai;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::futures::StreamExt;

use super::storage::MemoryStorage;

pub struct RetrievalSettings {
    pub temperature: f64,
    pub top_p: f64,
    pub max_res_tokens: u32,
}

pub struct RetrievalClient {
    pub client: Client,
    pub settings: ChatOptions,
    pub prompt: String,
    pub storage: MemoryStorage,
    pub model: String,
}

impl RetrievalClient {
    pub async fn new(config: &RetrievalConfig) -> anyhow::Result<()> {
        let api_key = config.gemini_key.clone();
        let client = OpenAIClient::builder()
            // .with_endpoint("https://generativelanguage.googleapis.com/v1beta/openai")
            .with_endpoint("http://127.0.0.1:8080")
            .with_api_key(api_key.clone())
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build client: {:?}", e))?;

        let mut properties = HashMap::new();
        properties.insert(
            "query".to_string(),
            Box::new(types::JSONSchemaDefine {
                schema_type: Some(types::JSONSchemaType::String),
                description: Some("The query to perform on the memory archive".to_string()),
                ..Default::default()
            }),
        );

        let req = ChatCompletionRequest::new(config.model.clone(), vec![
            chat_completion::ChatCompletionMessage {
                role: chat_completion::MessageRole::user,
                content: chat_completion::Content::Text(String::from(
                    "What did I tell you about my shirt?",
                )),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ])
        .temperature(0.0)
        .top_p(1.0)
        .tools(vec![chat_completion::Tool {
            r#type: chat_completion::ToolType::Function,
            function: types::Function {
                name: String::from("memory_recall"),
                description: Some(String::from("Recall a memory from the memory archive")),
                parameters: types::FunctionParameters {
                    schema_type: types::JSONSchemaType::Object,
                    properties: Some(properties),
                    required: Some(vec![String::from("query")]),
                },
            },
        }])
        .tool_choice(chat_completion::ToolChoiceType::Auto);

        // debug request json
        let serialized = serde_json::to_string(&req).unwrap();
        println!("{}", serialized);

        let result = client.chat_completion(req).await.map_err(|e| {
            println!("error: {:?}", e);
            anyhow::anyhow!("error")
        })?;

        match result.choices[0].finish_reason {
            None => {
                println!("No finish_reason");
                println!("{:?}", result.choices[0].message.content);
            }
            Some(chat_completion::FinishReason::stop) => {
                println!("Stop");
                println!("{:?}", result.choices[0].message.content);
            }
            Some(chat_completion::FinishReason::length) => {
                println!("Length");
            }
            Some(chat_completion::FinishReason::tool_calls) => {
                println!("ToolCalls");
                #[derive(Deserialize, Serialize)]
                struct MemoryCall {
                    query: String,
                }
                let tool_calls = result.choices[0].message.tool_calls.as_ref().unwrap();
                for tool_call in tool_calls {
                    let name = tool_call.function.name.clone().unwrap();
                    let arguments = tool_call.function.arguments.clone().unwrap();
                    let memory_call: MemoryCall = serde_json::from_str(&arguments)?;
                    let query = memory_call.query;
                    if name == "memory_recall" {
                        println!("recalling: {:?}", query);
                    }
                }
            }
            Some(chat_completion::FinishReason::content_filter) => {
                println!("ContentFilter");
            }
            Some(chat_completion::FinishReason::null) => {
                println!("Null");
            }
        }

        Ok(())
    }

    pub async fn recall(&self, last_prompt: ChatMessage) -> Vec<String> {
        // let system_prompt = "memory_recall"
        // .with_description(
        //     "Searches long-term memory using Tantivy query syntax. Use for recalling facts, user preferences, or historical context. Always specify both query and threshold.",
        // )
        // .with_schema(json!({
        //     "type": "object",
        //     "properties": {
        //         "query": {
        //             "type": "string",
        //             "description": "Tantivy query string using proper field syntax. \
        //             Examples: 'content:hello', 'content:\"exact phrase\"', \
        //             'content:(important AND concept)'. MUST prefix with 'content:'.",
        //             "examples": [
        //                 "content:birthday",
        //                 "content:\"dark mode\"~2",
        //                 "content:(preference OR setting)^2"
        //             ]
        //         },
        //         "threshold": {
        //             "type": "number",
        //             "minimum": 0.0,
        //             "maximum": 1.0,
        //             "description": "Similarity score cutoff (0.1=loose, 0.5=strict). \
        //             Use lower values for fuzzy matches, higher for exact recalls.",
        //             "default": 0.3
        //         }
        //     },
        //     "required": ["query", "threshold"]
        // }));
        let system_prompt = ChatMessage::system(self.prompt.clone());
        let context = vec![system_prompt, last_prompt];

        // old debug code
        // for message in context.iter() {
        //     match message.role {
        //         ChatRole::System => {}
        //         _ => {
        //             println!("{:?}", message);
        //         }
        //     }
        // }

        let request = ChatRequest::new(context);

        println!("there is a request");

        let response = self
            .client
            .exec_chat(&self.model, request, Some(&self.settings))
            .await;

        println!("recall result: {:?}", response);

        let msg = response
            .unwrap()
            .content_text_into_string()
            .ok_or(anyhow::anyhow!("No content"))
            .unwrap();

        println!("there is content");

        println!("Message: {}", msg);

        let result = self.storage.search(&msg, 0.3).unwrap();

        println!("recall result: {:?}", result);

        todo!()
    }

    // pub async fn summarize(&self, mut context: Vec<ChatMessage>) -> Result<CompletionMessage> {
    //     context.retain(|message| {
    //         if let ChatRole::System = message.role {
    //             false
    //         } else {
    //             println!("{:?}", message);
    //             true
    //         }
    //     });

    //     let request = ChatRequest::new(context);

    //     println!("there is a request");

    //     let response = self
    //         .client
    //         .exec_chat(&self.model, request, Some(&self.settings))
    //         .await?;

    //     println!("there is a response");

    //     let msg = response
    //         .content_text_into_string()
    //         .ok_or(anyhow::anyhow!("No content"))?;

    //     println!("there is content");

    //     let regex = Regex::new(r"```.*").unwrap();
    //     let content = regex.replace_all(&msg, "").to_string();

    //     // tokio::time::sleep(std::time::Duration::from_secs(5)).await; // simulate API call latency
    //     Ok(CompletionMessage {
    //         role: "assistant".to_string(),
    //         content,
    //     })
    // }
}
