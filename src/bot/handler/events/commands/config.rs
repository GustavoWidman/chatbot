use std::fmt::Display;

use poise::CreateReply;

use crate::bot::handler::{events::HandlerResult, framework::Context};

#[derive(Debug, poise::ChoiceParameter)]
pub enum KeyChoice {
    // Completion
    // Model
    Model,
    Provider,
    #[name = "API Key"]
    ApiKey,
    #[name = "Custom URL"]
    CustomUrl,
    // Reasoning
    Reason,
    #[name = "Fake Reason"]
    FakeReason,
    // Additional Parameters
    #[name = "Max Tokens"]
    MaxTokens,
    Temperature,

    // Embedding
    // Model
    #[name = "Embedding Model"]
    EmbeddingModel,
    #[name = "Embedding Provider"]
    EmbeddingProvider,
    #[name = "Embedding API Key"]
    EmbeddingApiKey,
    #[name = "Embedding Custom URL"]
    EmbeddingCustomUrl,
    #[name = "Vector Size"]
    VectorSize,
    // Vector DB
    #[name = "QDrant Host"]
    QdrantHost,
    #[name = "QDrant Port"]
    QdrantPort,
    #[name = "Use HTTPs for QDrant"]
    QdrantHttps,

    #[name = "Use Tools"]
    UseTools,
    #[name = "Force Lowercase"]
    ForceLowercase,
    #[name = "Memory Similarity Threshold"]
    SimilarityThreshold,
}

impl Display for KeyChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey => write!(f, "API Key"),
            Self::Model => write!(f, "Model"),
            Self::CustomUrl => write!(f, "Custom URL"),
            Self::EmbeddingCustomUrl => write!(f, "Embedding Custom URL"),
            Self::EmbeddingModel => write!(f, "Embedding Model"),
            Self::EmbeddingProvider => write!(f, "Embedding Provider"),
            Self::EmbeddingApiKey => write!(f, "Embedding API Key"),
            Self::UseTools => write!(f, "Use Tools"),
            Self::ForceLowercase => write!(f, "Force Lowercase"),
            Self::Provider => write!(f, "Provider"),
            Self::MaxTokens => write!(f, "Max Tokens"),
            Self::Temperature => write!(f, "Temperature"),
            Self::VectorSize => write!(f, "Vector Size"),
            Self::SimilarityThreshold => write!(f, "Memory Similarity Threshold"),
            Self::QdrantHost => write!(f, "QDrant Host"),
            Self::QdrantPort => write!(f, "QDrant Port"),
            Self::QdrantHttps => write!(f, "Use HTTPs for QDrant"),
            Self::Reason => write!(f, "Reason"),
            Self::FakeReason => write!(f, "Fake Reason"),
        }
    }
}

pub async fn config(ctx: Context<'_>, key: KeyChoice, value: Option<String>) -> HandlerResult<()> {
    let data = ctx.data().clone();

    let result: anyhow::Result<()> = async {
        if let Some(mut value) = value {
            let mut config = data.config.write().await;
            config.update();

            if value.to_lowercase() == "null" || value.to_lowercase() == "none" {
                value = "".to_string();
            }

            match key {
                KeyChoice::ApiKey => {
                    config.llm.completion.api_key = value.clone();
                }
                KeyChoice::Model => {
                    config.llm.completion.model = value.clone();
                }
                KeyChoice::EmbeddingModel => {
                    config.llm.embedding.model = value.clone();
                }
                KeyChoice::EmbeddingProvider => {
                    if value.trim().is_empty() {
                        config.llm.embedding.provider = None;
                    } else {
                        config.llm.embedding.provider = Some(value.clone().try_into()?);
                    }
                }
                KeyChoice::EmbeddingApiKey => {
                    if value.trim().is_empty() {
                        config.llm.embedding.api_key = None;
                    } else {
                        config.llm.embedding.api_key = Some(value.clone());
                    }
                }
                KeyChoice::EmbeddingCustomUrl => {
                    if value.trim().is_empty() {
                        config.llm.embedding.custom_url = None;
                    } else {
                        config.llm.embedding.custom_url = Some(value.clone());
                    }
                }
                KeyChoice::CustomUrl => {
                    if value.trim().is_empty() {
                        config.llm.completion.custom_url = None;
                    } else {
                        config.llm.completion.custom_url = Some(value.clone());
                    }
                }
                KeyChoice::UseTools => {
                    if value.trim().is_empty() {
                        config.llm.use_tools = None;
                    } else {
                        config.llm.use_tools =
                            Some(value.to_lowercase().parse::<bool>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid boolean"
                                )
                            })?);
                    }
                }
                KeyChoice::ForceLowercase => {
                    if value.trim().is_empty() {
                        config.llm.force_lowercase = None;
                    } else {
                        config.llm.force_lowercase =
                            Some(value.to_lowercase().parse::<bool>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid boolean"
                                )
                            })?);
                    }
                }
                KeyChoice::Provider => {
                    config.llm.completion.provider = value.clone().try_into()?;
                }
                KeyChoice::MaxTokens => {
                    if value.trim().is_empty() {
                        config.llm.completion.max_tokens = None;
                    } else {
                        config.llm.completion.max_tokens =
                            Some(value.parse::<u64>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid number"
                                )
                            })?);
                    }
                }
                KeyChoice::Temperature => {
                    if value.trim().is_empty() {
                        config.llm.completion.temperature = None;
                    } else {
                        config.llm.completion.temperature =
                            Some(value.parse::<f64>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid number"
                                )
                            })?);
                    }
                }
                KeyChoice::VectorSize => {
                    if value.trim().is_empty() {
                        config.llm.embedding.vector_size = None;
                    } else {
                        config.llm.embedding.vector_size =
                            Some(value.parse::<usize>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid number"
                                )
                            })?);
                    }
                }
                KeyChoice::SimilarityThreshold => {
                    if value.trim().is_empty() {
                        config.llm.similarity_threshold = None;
                    } else {
                        config.llm.similarity_threshold =
                            Some(value.parse::<f64>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid number"
                                )
                            })?);
                    }
                }
                KeyChoice::QdrantHost => {
                    config.llm.embedding.qdrant_host = value.clone();
                }
                KeyChoice::QdrantPort => {
                    if value.trim().is_empty() {
                        config.llm.embedding.qdrant_port = None;
                    } else {
                        config.llm.embedding.qdrant_port =
                            Some(value.parse::<u16>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid number"
                                )
                            })?);
                    }
                }
                KeyChoice::QdrantHttps => {
                    if value.trim().is_empty() {
                        config.llm.embedding.qdrant_https = None;
                    } else {
                        config.llm.embedding.qdrant_https =
                            Some(value.to_lowercase().parse::<bool>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid boolean"
                                )
                            })?);
                    }
                }
                KeyChoice::Reason => {
                    if value.trim().is_empty() {
                        config.llm.completion.reason = None;
                    } else {
                        config.llm.completion.reason =
                            Some(value.to_lowercase().parse::<bool>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid boolean"
                                )
                            })?);
                    }
                }
                KeyChoice::FakeReason => {
                    if value.trim().is_empty() {
                        config.llm.completion.fake_reason = None;
                    } else {
                        config.llm.completion.fake_reason =
                            Some(value.to_lowercase().parse::<bool>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid boolean"
                                )
                            })?);
                    }
                }
            }

            config.async_save().await?;

            if value.trim().is_empty() {
                ctx.send(
                    CreateReply::default()
                        .content(format!("Successfully unset the {key}"))
                        .ephemeral(true),
                )
                .await?;
            } else {
                ctx.send(
                    CreateReply::default()
                        .content(format!("Successfully updated the {key} to `{value}`"))
                        .ephemeral(true),
                )
                .await?;
            };
        } else {
            let config = data.config.read().await;

            let (value, sensitive): (Option<String>, bool) = match key {
                KeyChoice::ApiKey => (Some(config.llm.completion.api_key.clone()), true),
                KeyChoice::Model => (Some(config.llm.completion.model.clone()), false),
                KeyChoice::EmbeddingModel => (Some(config.llm.embedding.model.clone()), false),
                KeyChoice::EmbeddingCustomUrl => (config.llm.embedding.custom_url.clone(), false),
                KeyChoice::CustomUrl => (config.llm.completion.custom_url.clone(), false),
                KeyChoice::EmbeddingProvider => (
                    config
                        .llm
                        .embedding
                        .provider
                        .map(|provider| provider.to_string()),
                    false,
                ),
                KeyChoice::EmbeddingApiKey => (config.llm.embedding.api_key.clone(), true),
                KeyChoice::ForceLowercase => (
                    config
                        .llm
                        .force_lowercase
                        .map(|force_lowercase| force_lowercase.to_string()),
                    false,
                ),
                KeyChoice::UseTools => (
                    config.llm.use_tools.map(|use_tools| use_tools.to_string()),
                    false,
                ),
                KeyChoice::Provider => (Some(config.llm.completion.provider.to_string()), false),
                KeyChoice::MaxTokens => (
                    config
                        .llm
                        .completion
                        .max_tokens
                        .map(|max_tokens| max_tokens.to_string()),
                    false,
                ),
                KeyChoice::Temperature => (
                    config
                        .llm
                        .completion
                        .temperature
                        .map(|temperature| temperature.to_string()),
                    false,
                ),
                KeyChoice::VectorSize => (
                    config
                        .llm
                        .embedding
                        .vector_size
                        .map(|vector_size| vector_size.to_string()),
                    false,
                ),
                KeyChoice::SimilarityThreshold => (
                    config
                        .llm
                        .similarity_threshold
                        .as_ref()
                        .map(|similarity_threshold| similarity_threshold.to_string()),
                    false,
                ),
                KeyChoice::QdrantHost => (Some(config.llm.embedding.qdrant_host.clone()), false),
                KeyChoice::QdrantPort => (
                    config
                        .llm
                        .embedding
                        .qdrant_port
                        .map(|qdrant_port| qdrant_port.to_string()),
                    false,
                ),
                KeyChoice::QdrantHttps => (
                    config
                        .llm
                        .embedding
                        .qdrant_https
                        .map(|qdrant_https| qdrant_https.to_string()),
                    false,
                ),
                KeyChoice::Reason => (
                    config
                        .llm
                        .completion
                        .reason
                        .map(|reason| reason.to_string()),
                    false,
                ),
                KeyChoice::FakeReason => (
                    config
                        .llm
                        .completion
                        .fake_reason
                        .map(|fake_reason| fake_reason.to_string()),
                    false,
                ),
            };

            let value = if let Some(value) = value {
                Some(if sensitive {
                    format!("||`{}`||", value)
                } else {
                    format!("`{}`", value)
                })
            } else {
                None
            };

            let content = match value {
                Some(value) => format!("The value for {key} is {value}"),
                None => format!("The value for {key} is not set"),
            };

            ctx.send(CreateReply::default().content(content).ephemeral(true))
                .await?;
        }

        Ok(())
    }
    .await;

    match result {
        Ok(_) => HandlerResult::ok(()),
        Err(why) => HandlerResult::err(why, ctx),
    }
}
