use std::{collections::HashMap, path::PathBuf};

use rig_dyn::Provider;
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
    pub context: ContextConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ContextConfig {
    pub max_stm: usize,
    pub save_to_disk_folder: Option<PathBuf>,
    pub system: SystemPromptBuilder,
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
    // Completion
    pub completion: LLMCompletionConfig,

    // Embedding
    pub embedding: LLMEmbeddingConfig,

    pub additional_params: Option<HashMap<String, toml::Value>>,

    pub use_tools: Option<bool>,
    pub force_lowercase: Option<bool>,
    pub similarity_threshold: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct LLMCompletionConfig {
    // Model
    pub model: String,
    pub provider: Provider,
    pub api_key: String,
    pub custom_url: Option<String>,

    // Reasoning
    pub reason: Option<bool>,
    pub fake_reason: Option<bool>,

    // Additional Parameters
    pub max_tokens: Option<u64>,
    pub temperature: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct LLMEmbeddingConfig {
    // Model
    pub model: String,
    pub provider: Option<Provider>,
    pub api_key: Option<String>,
    pub custom_url: Option<String>,

    pub vector_size: Option<usize>,

    // Vector DB
    pub qdrant_host: String,
    pub qdrant_port: Option<u16>,
    pub qdrant_https: Option<bool>,
}
