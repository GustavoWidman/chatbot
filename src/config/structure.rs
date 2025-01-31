use serde::{Deserialize, Serialize};
use serenity::prelude::TypeMapKey;

use crate::chat::prompt::SystemPromptBuilder;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct ChatBotConfigTOML {
    pub config: ChatBotConfigInner,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ChatBotConfigInner {
    pub discord: DiscordConfig,
    pub llm: LLMConfig,
    pub prompt: SystemPromptBuilder,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DiscordConfig {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct LLMConfig {
    // pub provider: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
}
