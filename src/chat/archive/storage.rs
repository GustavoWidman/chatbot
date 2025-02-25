use std::collections::HashMap;

use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, SearchPointsBuilder, UpsertPointsBuilder,
        Value, VectorParamsBuilder, vectors_config::Config,
    },
};
use serenity::all::UserId;

use crate::config::structure::LLMConfig;

pub struct MemorySettings {
    pub vector_size: u64,
    pub similarity_threshold: f32,
}

pub struct MemoryStorage {
    client: Qdrant,
    settings: MemorySettings,
}

impl MemoryStorage {
    pub fn new(config: &LLMConfig, vector_size: u64) -> Self {
        let client = Qdrant::from_url(&format!(
            "http{}://{}:{}",
            match config.qdrant_https.unwrap_or(false) {
                true => "s",
                false => "",
            },
            config.qdrant_host,
            config.qdrant_port.unwrap_or(6334)
        ))
        .skip_compatibility_check()
        .build()
        .unwrap();

        MemoryStorage {
            client,
            settings: MemorySettings {
                vector_size,
                similarity_threshold: config.similarity_threshold.unwrap_or(0.5),
            },
        }
    }

    pub async fn health_check(&self, user_id: UserId) -> anyhow::Result<()> {
        self.client.health_check().await?;

        let collection_name = self.try_create_collection(user_id).await?;

        let collection_info = self.client.collection_info(collection_name).await?;

        let vector_size: u64 = async {
            if let Config::Params(params) = collection_info
                .result?
                .config?
                .params?
                .vectors_config?
                .config?
            {
                Some(params.size)
            } else {
                None
            }
        }
        .await
        .ok_or(anyhow::anyhow!("failed to get vector size"))?;

        if vector_size != self.settings.vector_size {
            Err(anyhow::anyhow!(
                "vector size mismatch, expected {} but got {}",
                self.settings.vector_size,
                vector_size
            ))
        } else {
            Ok(())
        }
    }

    async fn try_create_collection(&self, user_id: UserId) -> anyhow::Result<String> {
        let collection_name = format!("chatbot_{}", user_id);

        Ok(
            match self.client.collection_exists(&collection_name).await? {
                true => collection_name,
                false => {
                    self.client
                        .create_collection(
                            CreateCollectionBuilder::new(&collection_name).vectors_config(
                                VectorParamsBuilder::new(
                                    self.settings.vector_size,
                                    Distance::Cosine,
                                ),
                            ),
                        )
                        .await?;
                    collection_name
                }
            },
        )
    }

    pub async fn store(
        &self,
        text: String,
        embedding: Vec<f32>,
        user_id: UserId,
    ) -> anyhow::Result<()> {
        let collection_name = self.try_create_collection(user_id).await?;

        let points = vec![PointStruct::new(
            rand::random::<u64>(),
            embedding,
            Payload::from(HashMap::from([("memory".to_string(), Value::from(text))])),
        )];
        self.client
            .upsert_points(UpsertPointsBuilder::new(collection_name, points))
            .await?;

        Ok(())
    }

    pub async fn search(
        &self,
        embedding: Vec<impl Into<f32>>,
        user_id: UserId,
    ) -> anyhow::Result<Vec<String>> {
        let embedding = embedding
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<f32>>();

        let collection_name = self.try_create_collection(user_id).await?;

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(collection_name, embedding, 1) // todo set recall limit
                    // .filter(Filter::all([Condition::matches("bar", 12)]))
                    .with_payload(true), // .params(SearchParamsBuilder::default().exact(true)),
            )
            .await?;

        Ok(search_result
            .result
            .into_iter()
            .filter_map(|point| {
                log::info!(
                    "payload:\n{}\nscore: {:?}",
                    serde_json::to_string_pretty(&point.payload).ok()?,
                    point.score
                );
                if point.score > self.settings.similarity_threshold {
                    let payload = point.payload;
                    let memory = payload.get("memory")?.as_str()?;

                    Some(memory.to_string())
                } else {
                    None
                }
            })
            .collect())
    }
}
