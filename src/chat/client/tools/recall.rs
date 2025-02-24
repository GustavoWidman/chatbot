use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serenity::all::UserId;

use crate::{archive::storage::MemoryStorage, chat::client::providers::DynEmbeddingModel};

#[derive(Deserialize)]
pub struct Args {
    query: String,
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
}

impl MemoryRecall {
    pub fn new(
        model: Arc<Box<dyn DynEmbeddingModel>>,
        storage: Arc<MemoryStorage>,
        user_id: UserId,
    ) -> Self {
        Self {
            model,
            storage,
            user_id,
        }
    }

    fn search(&self, recall: &str) -> anyhow::Result<Vec<String>> {
        let embedded = tokio::task::block_in_place(|| {
            futures::executor::block_on(self.model.embed_text(recall))
        })?
        .vec
        .iter()
        .map(|&x| x as f32)
        .collect::<Vec<f32>>();

        tokio::task::block_in_place(|| {
            futures::executor::block_on(self.storage.search(embedded, self.user_id))
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
                }
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        log::info!("[memory_recall] querying vector db with \"{}\"", args.query);
        let results = self.search(&args.query).map_err(|_| MemoryRecallError)?;
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
