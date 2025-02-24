use std::fmt::Display;

use poise::CreateReply;

use super::{Context, Error};

#[derive(Debug, poise::ChoiceParameter)]
pub enum KeyChoice {
    #[name = "API Key"]
    ApiKey,
    Model,
    #[name = "Embedding Model"]
    EmbeddingModel,
    #[name = "Embedding Provider"]
    EmbeddingProvider,
    #[name = "Embedding API Key"]
    EmbeddingApiKey,
    #[name = "Use Tools"]
    UseTools,
    #[name = "Force Lowercase"]
    ForceLowercase,
    Provider,
    #[name = "Max Tokens"]
    MaxTokens,
    Temperature,
    #[name = "Top P"]
    TopP,
    #[name = "Vector Size"]
    VectorSize,
    #[name = "Memory Similarity Threshold"]
    SimilarityThreshold,
    #[name = "QDrant Host"]
    QdrantHost,
    #[name = "QDrant Port"]
    QdrantPort,
    #[name = "Use HTTPs for QDrant"]
    QdrantHttps,
}

impl Display for KeyChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey => write!(f, "API Key"),
            Self::Model => write!(f, "Model"),
            Self::EmbeddingModel => write!(f, "Embedding Model"),
            Self::EmbeddingProvider => write!(f, "Embedding Provider"),
            Self::EmbeddingApiKey => write!(f, "Embedding API Key"),
            Self::UseTools => write!(f, "Use Tools"),
            Self::ForceLowercase => write!(f, "Force Lowercase"),
            Self::Provider => write!(f, "Provider"),
            Self::MaxTokens => write!(f, "Max Tokens"),
            Self::Temperature => write!(f, "Temperature"),
            Self::TopP => write!(f, "Top P"),
            Self::VectorSize => write!(f, "Vector Size"),
            Self::SimilarityThreshold => write!(f, "Memory Similarity Threshold"),
            Self::QdrantHost => write!(f, "QDrant Host"),
            Self::QdrantPort => write!(f, "QDrant Port"),
            Self::QdrantHttps => write!(f, "Use HTTPs for QDrant"),
        }
    }
}

/// Rewrite keys of the LLM config
#[poise::command(slash_command, prefix_command)]
pub(super) async fn config(
    ctx: Context<'_>,
    #[description = "Config property"] key: KeyChoice,

    #[description = "New value (if not provided, will print the current key value)"] value: Option<
        String,
    >,
) -> Result<(), Error> {
    let data = ctx.data().clone();

    if let Some(value) = value {
        let mut config = data.config.write().await;
        config.update();

        if let Err(e) = {
            match key {
                KeyChoice::ApiKey => {
                    config.llm.api_key = value.clone();
                }
                KeyChoice::Model => {
                    config.llm.model = value.clone();
                }
                KeyChoice::EmbeddingModel => {
                    config.llm.embedding_model = value.clone();
                }
                KeyChoice::EmbeddingProvider => {
                    if value.trim().is_empty() {
                        config.llm.embedding_provider = None;
                    } else {
                        config.llm.embedding_provider = Some(value.clone().try_into()?);
                    }
                }
                KeyChoice::EmbeddingApiKey => {
                    if value.trim().is_empty() {
                        config.llm.embedding_api_key = None;
                    } else {
                        config.llm.embedding_api_key = Some(value.clone());
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
                    config.llm.provider = value.clone().try_into()?;
                }
                KeyChoice::MaxTokens => {
                    if value.trim().is_empty() {
                        config.llm.max_tokens = None;
                    } else {
                        config.llm.max_tokens = Some(value.parse::<u64>().map_err(|_| {
                            anyhow::anyhow!(
                                "Invalid value \"{value}\", please provide a valid number"
                            )
                        })?);
                    }
                }
                KeyChoice::Temperature => {
                    if value.trim().is_empty() {
                        config.llm.temperature = None;
                    } else {
                        config.llm.temperature = Some(value.parse::<f64>().map_err(|_| {
                            anyhow::anyhow!(
                                "Invalid value \"{value}\", please provide a valid number"
                            )
                        })?);
                    }
                }
                KeyChoice::TopP => {
                    if value.trim().is_empty() {
                        config.llm.top_p = None;
                    } else {
                        config.llm.top_p = Some(value.parse::<f64>().map_err(|_| {
                            anyhow::anyhow!(
                                "Invalid value \"{value}\", please provide a valid number"
                            )
                        })?);
                    }
                }
                KeyChoice::VectorSize => {
                    if value.trim().is_empty() {
                        config.llm.vector_size = None;
                    } else {
                        config.llm.vector_size = Some(value.parse::<usize>().map_err(|_| {
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
                            Some(value.parse::<f32>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid number"
                                )
                            })?);
                    }
                }
                KeyChoice::QdrantHost => {
                    config.llm.qdrant_host = value.clone();
                }
                KeyChoice::QdrantPort => {
                    if value.trim().is_empty() {
                        config.llm.qdrant_port = None;
                    } else {
                        config.llm.qdrant_port = Some(value.parse::<u16>().map_err(|_| {
                            anyhow::anyhow!(
                                "Invalid value \"{value}\", please provide a valid number"
                            )
                        })?);
                    }
                }
                KeyChoice::QdrantHttps => {
                    if value.trim().is_empty() {
                        config.llm.qdrant_https = None;
                    } else {
                        config.llm.qdrant_https =
                            Some(value.to_lowercase().parse::<bool>().map_err(|_| {
                                anyhow::anyhow!(
                                    "Invalid value \"{value}\", please provide a valid boolean"
                                )
                            })?);
                    }
                }
            }

            config.async_save().await?;

            Ok::<(), anyhow::Error>(())
        } {
            ctx.send(
                CreateReply::default()
                    .content(format!("Error updating the {key}: {}", e.to_string()))
                    .ephemeral(true),
            )
            .await?;

            return Ok(());
        }

        ctx.send(
            CreateReply::default()
                .content(format!("Successfully updated the {key} to \"{value}\""))
                .ephemeral(true),
        )
        .await?;
    } else {
        let config = data.config.read().await;

        let value: Option<String> = match key {
            KeyChoice::ApiKey => Some(format!("||{}||", config.llm.api_key)),
            KeyChoice::Model => Some(config.llm.model.clone()),
            KeyChoice::EmbeddingModel => Some(config.llm.embedding_model.clone()),
            KeyChoice::EmbeddingProvider => config
                .llm
                .embedding_provider
                .map(|provider| provider.to_string()),
            KeyChoice::EmbeddingApiKey => config
                .llm
                .embedding_api_key
                .clone()
                .map(|api_key| format!("||{}||", api_key)),
            KeyChoice::ForceLowercase => config
                .llm
                .force_lowercase
                .map(|force_lowercase| force_lowercase.to_string()),
            KeyChoice::UseTools => config.llm.use_tools.map(|use_tools| use_tools.to_string()),
            KeyChoice::Provider => Some(config.llm.provider.to_string()),
            KeyChoice::MaxTokens => config
                .llm
                .max_tokens
                .map(|max_tokens| max_tokens.to_string()),
            KeyChoice::Temperature => config
                .llm
                .temperature
                .map(|temperature| temperature.to_string()),
            KeyChoice::TopP => config.llm.top_p.map(|top_p| top_p.to_string()),
            KeyChoice::VectorSize => config
                .llm
                .vector_size
                .map(|vector_size| vector_size.to_string()),
            KeyChoice::SimilarityThreshold => config
                .llm
                .similarity_threshold
                .map(|similarity_threshold| similarity_threshold.to_string()),
            KeyChoice::QdrantHost => Some(config.llm.qdrant_host.clone()),
            KeyChoice::QdrantPort => config
                .llm
                .qdrant_port
                .map(|qdrant_port| qdrant_port.to_string()),
            KeyChoice::QdrantHttps => config
                .llm
                .qdrant_https
                .map(|qdrant_https| qdrant_https.to_string()),
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
