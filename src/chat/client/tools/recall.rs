use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serenity::all::UserId;

use crate::chat::{archive::storage::MemoryStorage, client::providers::DynEmbeddingModel};

#[derive(Deserialize, Serialize)]
pub struct Args {
    query: String,
    threshold: Option<f32>,
    limit: Option<u64>,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory Recall error")]
pub struct MemoryRecallError;

#[derive(Serialize)]
pub struct MemoryRecall {
    #[serde(skip)]
    model: Arc<Box<dyn DynEmbeddingModel>>,
    #[serde(skip)]
    storage: Arc<MemoryStorage>,
    #[serde(skip)]
    user_id: UserId,
    #[serde(skip)]
    user_name: String,
    #[serde(skip)]
    assistant_name: String,
}

impl MemoryRecall {
    pub fn new(
        model: Arc<Box<dyn DynEmbeddingModel>>,
        storage: Arc<MemoryStorage>,
        user_id: UserId,
        user_name: String,
        assistant_name: String,
    ) -> Self {
        Self {
            model,
            storage,
            user_id,
            user_name,
            assistant_name,
        }
    }

    fn search(&self, args: Args) -> anyhow::Result<Vec<String>> {
        println!("given args: {:?}", serde_json::to_string_pretty(&args)?);

        let embedded = tokio::task::block_in_place(|| {
            futures::executor::block_on(self.model.embed_text(&args.query))
        })?
        .vec
        .iter()
        .map(|&x| x as f32)
        .collect::<Vec<f32>>();

        tokio::task::block_in_place(|| {
            futures::executor::block_on(self.storage.search(
                embedded,
                self.user_id,
                args.limit.unwrap_or(5),
                args.threshold,
            ))
        })
        .map(|mut x| {
            x.iter_mut()
                .map(|x| {
                    x.content
                        .replace("<user>", self.user_name.as_str())
                        .replace("<assistant>", self.assistant_name.as_str())
                })
                .collect::<Vec<String>>()
        })
    }
}

impl Tool for MemoryRecall {
    const NAME: &'static str = "memory_recall";

    type Error = MemoryRecallError;
    type Args = Args;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "memory_recall",
            "description": "Use to recall facts, preferences, or historical context, such as things that the user told you in the past, to fill in gaps in current memory (context). The query should be similar to how you would look up things in a search engine (like a google search), for example: 'shirt color' or 'favorite movie'. The query should be a phrase, not a sentence, you aren't asking the database, you're querying it, so for example 'what shirt did james wear last week' is not a valid query, whereas 'shirt' or 'james shirt' is.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The query to perform on the memory archive"
                    },
                    "threshold": {
                        "type": "number",
                        "description": "The minimum similarity score to return a memory (must be a decimal between 0 and 1)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "The maximum number of memories to recall (must be bigger than 0)"
                    },
                }
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        log::info!("[memory_recall] querying vector db with \"{}\"", args.query);
        let results = self.search(args).map_err(|_| MemoryRecallError)?;
        log::info!(
            "[memory_recall] results: {:?}",
            serde_json::to_string_pretty(&results)
        );

        if results.is_empty() {
            return Ok(json!({
                "memory_recall_result": "Could not find any relevant memories"
            }));
        } else {
            return Ok(json!({
                "memory_recall_result": "Found relevant memories",
                "memories": results
            }));
        }
    }
}
