use std::any;
use std::fmt::Display;

use poise::serenity_prelude as serenity;
use poise::CreateReply;
use serde::Deserialize;

use super::{Context, Error};
use crate::chat;

#[derive(Debug, poise::ChoiceParameter)]
pub enum KeyChoice {
    #[name = "API Key"]
    ApiKey,
    Model,
    #[name = "Custom URL"]
    CustomUrl,
    #[name = "Max Tokens"]
    MaxTokens,
    Temperature,
    #[name = "Top P"]
    TopP,
}

impl Display for KeyChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiKey => write!(f, "API Key"),
            Self::Model => write!(f, "Model"),
            Self::CustomUrl => write!(f, "Custom URL"),
            Self::MaxTokens => write!(f, "Max Tokens"),
            Self::Temperature => write!(f, "Temperature"),
            Self::TopP => write!(f, "Top P"),
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
                KeyChoice::CustomUrl => {
                    if value.trim().is_empty() {
                        config.llm.custom_url = None;
                    } else {
                        config.llm.custom_url = Some(value.clone());
                    }
                }
                KeyChoice::MaxTokens => {
                    config.llm.max_tokens = Some(value.parse::<i64>().map_err(|_| {
                        anyhow::anyhow!("Invalid value \"{value}\", please provide a valid number")
                    })?);
                }
                KeyChoice::Temperature => {
                    config.llm.temperature = Some(value.parse::<f64>().map_err(|_| {
                        anyhow::anyhow!("Invalid value \"{value}\", please provide a valid number")
                    })?);
                }
                KeyChoice::TopP => {
                    config.llm.top_p = Some(value.parse::<f64>().map_err(|_| {
                        anyhow::anyhow!("Invalid value \"{value}\", please provide a valid number")
                    })?);
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
            KeyChoice::CustomUrl => config.llm.custom_url.clone(),
            KeyChoice::MaxTokens => config
                .llm
                .max_tokens
                .map(|max_tokens| max_tokens.to_string()),
            KeyChoice::Temperature => config
                .llm
                .temperature
                .map(|temperature| temperature.to_string()),
            KeyChoice::TopP => config.llm.top_p.map(|top_p| top_p.to_string()),
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
