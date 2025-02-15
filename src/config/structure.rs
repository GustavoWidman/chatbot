use serde::{Deserialize, Serialize};

use crate::chat::prompt::SystemPromptBuilder;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct ChatBotConfigTOML {
    pub config: ChatBotConfigInner,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ChatBotConfigInner {
    pub discord: DiscordConfig,
    pub llm: LLMConfig,
    pub freewill: FreewillConfig,
    pub prompt: SystemPromptBuilder,
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
    pub api_key: String,
    pub model: String,
    pub embedding_model: String,
    pub embedding_custom_url: Option<String>,
    pub embedding_api_key: Option<String>,
    pub use_tools: Option<bool>,
    pub force_lowercase: Option<bool>,
    pub custom_url: Option<String>,
    pub max_tokens: Option<i64>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub vector_size: Option<u64>,
    pub similarity_threshold: Option<f32>,
    pub qdrant_host: String,
    pub qdrant_port: Option<u16>,
    pub qdrant_https: Option<bool>,
}
