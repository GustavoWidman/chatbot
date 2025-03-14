use rig::{completion::ToolDefinition, embeddings::Embedding, tool::Tool};
use rig_dyn::EmbeddingModel;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serenity::all::UserId;
use std::sync::Arc;

use crate::chat::archive::storage::{Memory, MemoryStorage};

#[derive(Debug, thiserror::Error)]
#[error("Memory Store error")]
pub struct MemoryStoreError;

#[derive(Deserialize)]
pub struct Args {
    memory: String,
}

#[derive(Serialize)]
pub struct MemoryStore {
    #[serde(skip)]
    model: Arc<Box<dyn EmbeddingModel>>,
    #[serde(skip)]
    storage: Arc<MemoryStorage>,
    #[serde(skip)]
    user_id: UserId,
    #[serde(skip)]
    user_name: String,
    #[serde(skip)]
    assistant_name: String,
}

impl MemoryStore {
    pub fn new(
        model: Arc<Box<dyn EmbeddingModel>>,
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

    fn store(&self, memory: &str) -> anyhow::Result<()> {
        let memory = memory
            .replace(self.user_name.as_str(), "<user>")
            .replace(self.assistant_name.as_str(), "<assistant>");

        let Embedding { document, vec } = tokio::task::block_in_place(|| {
            futures::executor::block_on(self.model.embed_text(&memory))
        })?;

        let vec = vec.into_iter().map(|x| x as f32).collect::<Vec<f32>>();

        tokio::task::block_in_place(|| {
            futures::executor::block_on(self.storage.store(
                Memory::new(document),
                vec,
                self.user_id,
            ))
        })
    }
}

impl Tool for MemoryStore {
    const NAME: &'static str = "memory_store";

    type Error = MemoryStoreError;
    type Args = Args;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "memory_store",
            "description": "Use to store facts, preferences, or historical context, such as things that the user tells you that you judge should be remembered. Thoroughly describe the context, including the time and place, and the details of the facts you're storing. Preferably, use a list format, such as bullet points, to make it easier to recall the information later. Do not use the same bullet points more than once. The response should only contain the bullet points, and nothing else. Do not remember things that are temporary, such as current actions or events, simply record facts that should be remembered for the long term, like \"user likes apples\" instead of \"user is in the kitchen\"",
            "parameters": {
                "type": "object",
                "properties": {
                    "memory": {
                        "type": "string",
                        "description": "The memory to store (in bullet points)"
                    },
                }
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        log::info!("[memory_store] saving memory:\n\"{}\"", args.memory);
        let result = self.store(&args.memory).map_err(|_| MemoryStoreError)?;
        log::info!(
            "[memory_store] result: {:?}",
            serde_json::to_string_pretty(&result)
        );
        Ok(json!({
            "memory_store_result": "Memory store successful!"
        }))
    }
}
