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
    // not available in genai
    // pub frequency_penalty: f32,
    // pub presence_penalty: f32,
}

pub struct ChatClient {
    // old library, didnt have system prompt during chat, only at the start of a chat, which makes it difficult to dynamically add context
    // pub client: Agent<CompletionModel>,
    pub client: Client,
    pub settings: ChatOptions,
    pub model: String,
    // pub memory_tool: Tool,
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
                if endpoint.is_some() {
                    let ServiceTarget { model, .. } = service_target;
                    let ModelIden { model_name, .. } = model;
                    let endpoint = Endpoint::from_owned(endpoint.unwrap());
                    let auth = AuthData::from_single(key);
                    let model = ModelIden {
                        adapter_kind: provider.into(),
                        model_name,
                    };
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

        // let memory_tool = Tool::new("memory_recall")
        //     .with_description(
        //         "Searches long-term memory using Tantivy query syntax. Use for recalling facts, user preferences, or historical context. Always specify both query and threshold.",
        //     )
        //     .with_schema(json!({
        //         "type": "object",
        //         "properties": {
        //             "query": {
        //                 "type": "string",
        //                 "description": "Tantivy query string using proper field syntax. \
        //                 Examples: 'content:hello', 'content:\"exact phrase\"', \
        //                 'content:(important AND concept)'. MUST prefix with 'content:'.",
        //                 "examples": [
        //                     "content:birthday",
        //                     "content:\"dark mode\"~2",
        //                     "content:(preference OR setting)^2"
        //                 ]
        //             },
        //             "threshold": {
        //                 "type": "number",
        //                 "minimum": 0.0,
        //                 "maximum": 1.0,
        //                 "description": "Similarity score cutoff (0.1=loose, 0.5=strict). \
        //                 Use lower values for fuzzy matches, higher for exact recalls.",
        //                 "default": 0.3
        //             }
        //         },
        //         "required": ["query", "threshold"]
        //     }));

        Self {
            client,
            settings: opts,
            model: config.model.clone(),
            // memory_tool,
        }
    }

    pub async fn prompt(&self, mut context: Vec<ChatMessage>) -> Result<CompletionMessage> {
        // old debug code
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
            .await;

        let response = match response {
            Ok(response) => Ok(response),
            Err(err) => {
                println!("there is an error");
                println!("{:?}", err);
                Err(err)
            }
        }?;

        println!("there is a response");

        // match response.tool_calls() {
        //     Some(tool_calls) => {
        //         println!("tool calls: {:?}", tool_calls);
        //         for tool_call in tool_calls {
        //             println!("tool call: {:?}", tool_call);
        //         }
        //     }
        //     None => {
        //         println!("no tool calls");
        //     }
        // }

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
