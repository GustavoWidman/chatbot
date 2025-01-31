use anyhow::{Result, anyhow};
use rig::{
    agent::Agent,
    completion::{Chat, Message},
    providers::{
        anthropic, cohere, deepseek,
        gemini::{self, completion::CompletionModel},
        hyperbolic, openai, perplexity, xai,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct ClientSettings {
    pub temperature: f64,
    pub top_p: f64,
    pub max_res_tokens: u64,
    // pub frequency_penalty: f32,
    // pub presence_penalty: f32,
    pub model: String,
}

pub struct ChatClient {
    pub client: Agent<CompletionModel>,
    pub settings: ClientSettings,
}

impl ChatClient {
    pub fn new() -> Self {
        let settings = ClientSettings {
            temperature: 0.7,
            top_p: 0.7,
            max_res_tokens: 1024,
            // frequency_penalty: 0.0,
            // presence_penalty: 0.0,
            model: dotenv!("CHAT_MODEL").to_string(),
        };

        let client = gemini::Client::new(dotenv!("CHAT_API_KEY"))
            .agent(dotenv!("CHAT_MODEL"))
            .max_tokens(settings.max_res_tokens)
            .temperature(settings.temperature)
            .additional_params(json!({"top_p": settings.top_p}))
            .build();

        Self { client, settings }
    }

    pub async fn prompt(&self, prompt: &str, context: Vec<Message>) -> Result<Message> {
        // tokio::time::sleep(std::time::Duration::from_secs(5)).await; // simulate API call latency
        Ok(Message {
            role: "assistant".to_string(),
            content: self.client.chat(prompt, context).await?,
        })
    }
}
