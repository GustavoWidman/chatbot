use anyhow::{Result, anyhow};
use regex::Regex;
use rig::{
    agent::Agent,
    completion::{Chat, Completion, CompletionResponse, Message, ModelChoice},
    providers::{
        anthropic, cohere, deepseek,
        gemini::{self, completion::CompletionModel},
        hyperbolic, openai, perplexity, xai,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::structure::LLMConfig;

pub struct ClientSettings {
    pub temperature: f64,
    pub top_p: f64,
    pub max_res_tokens: u64,
    // pub frequency_penalty: f32,
    // pub presence_penalty: f32,
}

pub struct ChatClient {
    pub client: Agent<CompletionModel>,
    pub settings: ClientSettings,
}

impl ChatClient {
    pub fn new(config: &LLMConfig) -> Self {
        let settings = ClientSettings {
            temperature: config.temperature.unwrap_or(1.0),
            top_p: config.top_p.unwrap_or(0.95),
            max_res_tokens: config.max_tokens.unwrap_or(1024),
        };

        let client = gemini::Client::new(&config.api_key)
            .agent(&config.model)
            .max_tokens(settings.max_res_tokens)
            .temperature(settings.temperature)
            .additional_params(json!({"top_p": settings.top_p}))
            .build();

        Self { client, settings }
    }

    pub async fn prompt(&self, prompt: &str, context: Vec<Message>) -> Result<Message> {
        println!("temp: {:?}", self.settings.temperature);

        let (msg, response) = match self
            .client
            .completion(prompt, context)
            .await?
            .send()
            .await?
        {
            CompletionResponse {
                choice: ModelChoice::Message(msg),
                raw_response: response,
            } => Some((msg, response)),
            CompletionResponse {
                choice: ModelChoice::ToolCall(_, _, _),
                ..
            } => None,
        }
        .ok_or(anyhow::anyhow!("No choices found"))?;

        // println!("{:?}", response);

        let regex = Regex::new(r"```.*").unwrap();
        let content = regex.replace_all(&msg, "").to_string();

        // tokio::time::sleep(std::time::Duration::from_secs(5)).await; // simulate API call latency
        Ok(Message {
            role: "assistant".to_string(),
            content: content,
        })
    }
}
