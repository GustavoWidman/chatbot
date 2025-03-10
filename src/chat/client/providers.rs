use std::fmt::Display;

use async_trait::async_trait;

use rig::{
    OneOrMany,
    completion::{CompletionError, CompletionRequest},
    embeddings::{Embedding, EmbeddingError, EmbeddingModel},
    message::AssistantContent,
    providers::{
        anthropic, azure, cohere, deepseek, galadriel, gemini, groq, hyperbolic, moonshot, openai,
        perplexity, xai,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub enum ProviderClient {
    Anthropic(anthropic::Client),
    Azure(azure::Client),
    Cohere(cohere::Client),
    Deepseek(deepseek::Client),
    Galadriel(galadriel::Client),
    Gemini(gemini::Client),
    Groq(groq::Client),
    Hyperbolic(hyperbolic::Client),
    Moonshot(moonshot::Client),
    OpenAI(openai::Client),
    Perplexity(perplexity::Client),
    Xai(xai::Client),
}

#[async_trait]
pub trait DynEmbeddingModel: Send + Sync {
    async fn embed_text(&self, input: &str) -> Result<Embedding, EmbeddingError>;
    #[allow(unused)]
    async fn embed_texts(&self, input: Vec<String>) -> Result<Vec<Embedding>, EmbeddingError>;
    #[allow(unused)]
    fn ndims(&self) -> usize;
}

#[async_trait]
impl<T> DynEmbeddingModel for T
where
    T: rig::embeddings::EmbeddingModel + Send + Sync,
{
    async fn embed_text(&self, input: &str) -> Result<Embedding, EmbeddingError> {
        EmbeddingModel::embed_text(self, input).await
    }

    async fn embed_texts(&self, input: Vec<String>) -> Result<Vec<Embedding>, EmbeddingError> {
        EmbeddingModel::embed_texts(self, input).await
    }

    fn ndims(&self) -> usize {
        EmbeddingModel::ndims(self)
    }
}

#[async_trait]
pub trait DynCompletionModel: Send + Sync {
    async fn completion(
        &self,
        completion: CompletionRequest,
    ) -> Result<OneOrMany<AssistantContent>, CompletionError>;
}

#[async_trait]
impl<T> DynCompletionModel for T
where
    T: rig::completion::CompletionModel + Send + Sync,
{
    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<OneOrMany<AssistantContent>, CompletionError> {
        Ok(self.completion(request).await?.choice)
    }
}

impl ProviderClient {
    /// Returns a completion model wrapper for the given provider and model name.
    pub async fn completion_model(&self, model: &str) -> Box<dyn DynCompletionModel> {
        match self {
            ProviderClient::Anthropic(client) => Box::new(client.completion_model(model)),
            ProviderClient::Azure(client) => Box::new(client.completion_model(model)),
            ProviderClient::Cohere(client) => Box::new(client.completion_model(model)),
            ProviderClient::Deepseek(client) => Box::new(client.completion_model(model)),
            ProviderClient::Galadriel(client) => Box::new(client.completion_model(model)),
            ProviderClient::Gemini(client) => Box::new(client.completion_model(model)),
            ProviderClient::Groq(client) => Box::new(client.completion_model(model)),
            ProviderClient::Hyperbolic(client) => Box::new(client.completion_model(model)),
            ProviderClient::Moonshot(client) => Box::new(client.completion_model(model)),
            ProviderClient::OpenAI(client) => Box::new(client.completion_model(model)),
            ProviderClient::Perplexity(client) => Box::new(client.completion_model(model)),
            ProviderClient::Xai(client) => Box::new(client.completion_model(model)),
        }
    }

    /// Returns an embedding model wrapper for the given provider and model name.
    /// Returns `None` if the provider does not support embeddings or
    /// if improper input type is provided (cohere requires a input type).
    pub async fn embedding_model(
        &self,
        model: &str,
        input_type: Option<&str>,
    ) -> Option<Box<dyn DynEmbeddingModel>> {
        match self {
            ProviderClient::Anthropic(_) => None,
            ProviderClient::Azure(client) => Some(Box::new(client.embedding_model(model))),
            ProviderClient::Cohere(client) => input_type.map(|input_type| {
                Box::new(client.embedding_model(model, input_type)) as Box<dyn DynEmbeddingModel>
            }),
            ProviderClient::Deepseek(_) => None,
            ProviderClient::Galadriel(_) => None,
            ProviderClient::Gemini(client) => Some(Box::new(client.embedding_model(model))),
            ProviderClient::Groq(_) => None,
            ProviderClient::Hyperbolic(_) => None,
            ProviderClient::Moonshot(_) => None,
            ProviderClient::OpenAI(client) => Some(Box::new(client.embedding_model(model))),
            ProviderClient::Perplexity(_) => None,
            ProviderClient::Xai(client) => Some(Box::new(client.embedding_model(model))),
        }
    }

    pub async fn embedding_model_with_ndims(
        &self,
        model: &str,
        ndims: usize,
        input_type: Option<&str>,
    ) -> Option<Box<dyn DynEmbeddingModel>> {
        match self {
            ProviderClient::Anthropic(_) => None,
            ProviderClient::Azure(client) => {
                Some(Box::new(client.embedding_model_with_ndims(model, ndims)))
            }
            ProviderClient::Cohere(client) => input_type.map(|input_type| {
                Box::new(client.embedding_model_with_ndims(model, input_type, ndims))
                    as Box<dyn DynEmbeddingModel>
            }),
            ProviderClient::Deepseek(_) => None,
            ProviderClient::Galadriel(_) => None,
            ProviderClient::Gemini(client) => {
                Some(Box::new(client.embedding_model_with_ndims(model, ndims)))
            }
            ProviderClient::Groq(_) => None,
            ProviderClient::Hyperbolic(_) => None,
            ProviderClient::Moonshot(_) => None,
            ProviderClient::OpenAI(client) => {
                Some(Box::new(client.embedding_model_with_ndims(model, ndims)))
            }
            ProviderClient::Perplexity(_) => None,
            ProviderClient::Xai(client) => {
                Some(Box::new(client.embedding_model_with_ndims(model, ndims)))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    #[serde(rename = "anthropic")]
    Anthropic,

    #[serde(rename = "azure")]
    Azure,
    #[serde(rename = "cohere")]
    Cohere,

    #[serde(rename = "deepseek")]
    Deepseek,

    #[serde(rename = "galadriel")]
    Galadriel,

    #[serde(rename = "gemini")]
    Gemini,

    #[serde(rename = "groq")]
    Groq,

    #[serde(rename = "hyperbolic")]
    Hyperbolic,

    #[serde(rename = "moonshot")]
    Moonshot,

    #[serde(rename = "openai")]
    #[serde(alias = "openai-api")]
    #[serde(alias = "openai-compatible")]
    OpenAI,

    #[serde(rename = "perplexity")]
    Perplexity,

    #[serde(rename = "xai")]
    Xai,
}

impl Default for Provider {
    fn default() -> Self {
        Self::OpenAI
    }
}

impl TryFrom<String> for Provider {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        serde_plain::from_str(&value).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_plain::to_string(self)
            .map_err(|_| std::fmt::Error::default())?
            .fmt(f)
    }
}

impl Provider {
    pub fn client(
        &self,
        api_key: &str,
        custom_url: Option<&str>,
    ) -> anyhow::Result<ProviderClient> {
        Ok(match self {
            // todo: might be a good idea to add support for other anthropic-specific configurations
            // like `anthropic_version` and `anthropic_beta`
            Provider::Anthropic => {
                let builder = anthropic::ClientBuilder::new(api_key);
                if let Some(url) = custom_url {
                    ProviderClient::Anthropic(builder.base_url(url).build())
                } else {
                    ProviderClient::Anthropic(builder.build())
                }
            }

            // todo fix
            Provider::Azure => match custom_url {
                Some(url) => ProviderClient::Azure(azure::Client::new(api_key, "2024-10-21", url)),
                None => anyhow::bail!("Azure API requires a custom url"),
            },
            Provider::Cohere => match custom_url {
                None => ProviderClient::Cohere(cohere::Client::new(api_key)),
                Some(url) => ProviderClient::Cohere(cohere::Client::from_url(api_key, url)),
            },
            Provider::Deepseek => match custom_url {
                None => ProviderClient::Deepseek(deepseek::Client::new(api_key)),
                Some(url) => ProviderClient::Deepseek(deepseek::Client::from_url(api_key, url)),
            },

            // todo: might be a good idea to eventually add a clause for the `None` case
            // (it's meant to be the 'fine tuning' api key)
            Provider::Galadriel => match custom_url {
                None => ProviderClient::Galadriel(galadriel::Client::new(api_key, None)),
                Some(url) => {
                    ProviderClient::Galadriel(galadriel::Client::from_url(api_key, url, None))
                }
            },

            Provider::Gemini => match custom_url {
                None => ProviderClient::Gemini(gemini::Client::new(api_key)),
                Some(url) => ProviderClient::Gemini(gemini::Client::from_url(api_key, url)),
            },
            Provider::Groq => match custom_url {
                None => ProviderClient::Groq(groq::Client::new(api_key)),
                Some(url) => ProviderClient::Groq(groq::Client::from_url(api_key, url)),
            },
            Provider::Hyperbolic => match custom_url {
                None => ProviderClient::Hyperbolic(hyperbolic::Client::new(api_key)),
                Some(url) => ProviderClient::Hyperbolic(hyperbolic::Client::from_url(api_key, url)),
            },
            Provider::Moonshot => match custom_url {
                None => ProviderClient::Moonshot(moonshot::Client::new(api_key)),
                Some(url) => ProviderClient::Moonshot(moonshot::Client::from_url(api_key, url)),
            },
            Provider::OpenAI => match custom_url {
                None => ProviderClient::OpenAI(openai::Client::new(api_key)),
                Some(url) => ProviderClient::OpenAI(openai::Client::from_url(api_key, url)),
            },
            Provider::Perplexity => match custom_url {
                None => ProviderClient::Perplexity(perplexity::Client::new(api_key)),
                Some(url) => ProviderClient::Perplexity(perplexity::Client::from_url(api_key, url)),
            },
            Provider::Xai => ProviderClient::Xai(xai::Client::new(api_key)),
        })
    }
}
