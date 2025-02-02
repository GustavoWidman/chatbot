use serde::{Deserialize, Serialize};

use crate::chat::{client::ChatProvider, prompt::SystemPromptBuilder};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct ChatBotConfigTOML {
    pub config: ChatBotConfigInner,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ChatBotConfigInner {
    pub discord: DiscordConfig,
    pub llm: LLMConfig,
    pub prompt: SystemPromptBuilder,
    pub freewill: FreewillConfig,
    pub retrieval: RetrievalConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DiscordConfig {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct FreewillConfig {
    pub min_time_secs: u64,
    pub max_time_secs: u64,
    pub steepness: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct LLMConfig {
    pub provider: ChatProvider,
    pub api_key: String,
    pub model: String,
    pub custom_url: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct RetrievalConfig {
    pub gemini_key: String,
    pub model: String,
    pub prompt: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
}
