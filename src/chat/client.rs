use anyhow::Result;
use genai::{
    Client, ClientConfig, ModelIden, ServiceTarget,
    adapter::AdapterKind,
    chat::{ChatMessage, ChatOptions, ChatRequest},
    resolver::{AuthData, AuthResolver, Endpoint, ModelMapper, ServiceTargetResolver},
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::config::structure::LLMConfig;

use super::context::CompletionMessage;

pub struct ClientSettings {
    pub temperature: f64,
    pub top_p: f64,
    pub max_res_tokens: u32,
    // pub frequency_penalty: f32,
    // pub presence_penalty: f32,
}

pub struct ChatClient {
    // pub client: Agent<CompletionModel>,
    pub client: Client,
    pub settings: ChatOptions,
    pub model: String,
}

impl ChatClient {
    pub fn new(config: &LLMConfig) -> Self {
        let auth_resolver = AuthResolver::from_resolver_fn({
            let key = config.api_key.clone();

            |model_iden: ModelIden| Ok(Some(AuthData::from_single(key)))
        });

        let model_mapper = ModelMapper::from_mapper_fn({
            let provider = config.provider.clone();
            |model_iden: ModelIden| Ok(ModelIden::new(provider.into(), model_iden.model_name))
        });

        let target_resolver = ServiceTargetResolver::from_resolver_fn({
            let endpoint = config.custom_url.clone();
            let provider = config.provider.clone();
            let key = config.api_key.clone();
            move |service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                if endpoint.is_some() && provider == ChatProvider::OpenAI {
                    let ServiceTarget { model, .. } = service_target;
                    let endpoint = Endpoint::from_owned(endpoint.unwrap());
                    let auth = AuthData::from_single(key);
                    return Ok(ServiceTarget {
                        endpoint,
                        auth,
                        model,
                    });
                } else {
                    return Ok(service_target);
                }
            }
        });

        let opts = ChatOptions::default()
            .with_max_tokens(config.max_tokens.unwrap_or(1024))
            .with_temperature(config.temperature.unwrap_or(1.0))
            .with_top_p(config.top_p.unwrap_or(0.95));

        let cfg = ClientConfig::default()
            .with_chat_options(opts.clone())
            .with_auth_resolver(auth_resolver)
            .with_model_mapper(model_mapper)
            .with_service_target_resolver(target_resolver);

        let client = Client::builder().with_config(cfg).build();

        Self {
            client,
            settings: opts,
            model: config.model.clone(),
        }
    }

    pub async fn prompt(
        &self,
        prompt: Option<String>,
        mut context: Vec<ChatMessage>,
    ) -> Result<CompletionMessage> {
        // println!("temp: {:?}", self.settings.temperature);

        if let Some(prompt) = prompt {
            context.push(ChatMessage::user(prompt));
        }

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
            .await?;

        println!("there is a response");

        let msg = response
            .content_text_into_string()
            .ok_or(anyhow::anyhow!("No content"))?;

        println!("there is content");

        let regex = Regex::new(r"```.*").unwrap();
        let content = regex.replace_all(&msg, "").to_string();

        // tokio::time::sleep(std::time::Duration::from_secs(5)).await; // simulate API call latency
        Ok(CompletionMessage {
            role: "assistant".to_string(),
            content,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatProvider {
    /// OpenAI API provider (GPT-3, GPT-4, etc.)
    #[serde(rename = "openai")]
    #[serde(alias = "openai-compatible")]
    OpenAI,
    /// Anthropic API provider (Claude models)
    #[serde(rename = "anthropic")]
    Anthropic,
    /// Ollama local LLM provider for self-hosted models
    #[serde(rename = "ollama")]
    Ollama,
    /// DeepSeek API provider for their LLM models
    #[serde(rename = "deepseek")]
    DeepSeek,
    /// X.AI (formerly Twitter) API provider
    #[serde(rename = "xai")]
    XAI,
    /// Google Gemini API provider
    #[serde(rename = "google")]
    Google,
    /// Groq API provider
    #[serde(rename = "groq")]
    Groq,
}

impl Into<AdapterKind> for ChatProvider {
    fn into(self) -> AdapterKind {
        match self {
            Self::OpenAI => AdapterKind::OpenAI,
            Self::Anthropic => AdapterKind::Anthropic,
            Self::Ollama => AdapterKind::Ollama,
            Self::DeepSeek => AdapterKind::DeepSeek,
            Self::XAI => AdapterKind::Xai,
            Self::Google => AdapterKind::Gemini,
            Self::Groq => AdapterKind::Groq,
        }
    }
}

impl Default for ChatProvider {
    fn default() -> Self {
        Self::OpenAI
    }
}
