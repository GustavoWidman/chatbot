use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        Condition, CreateCollectionBuilder, Distance, FieldCondition, Filter, PointStruct, Range,
        ScrollPointsBuilder, SearchPointsBuilder, UpsertPointsBuilder, Value, VectorParamsBuilder,
        condition::ConditionOneOf, point_id::PointIdOptions, vectors_config::Config,
    },
};
use serde::{Deserialize, Serialize};
use serenity::all::UserId;

use crate::config::structure::LLMConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct Memory {
    pub id: u64,
    pub content: String,
    // pub topic: String,
    pub date: DateTime<Utc>,
}
impl Memory {
    pub fn new(content: String) -> Self {
        Self {
            id: rand::random(),
            content,
            date: Utc::now(),
        }
    }
    pub fn into(self) -> Payload {
        Payload::from(HashMap::from([
            ("content".to_string(), Value::from(self.content)),
            // ("topic".to_string(), Value::from(self.topic)),
            (
                "date".to_string(),
                Value::from(self.date.timestamp_millis()),
            ),
        ]))
    }
    pub fn try_from(id: u64, payload: HashMap<String, Value>) -> Option<Self> {
        Some(Self {
            id,
            content: payload.get("content")?.to_string(),
            // topic: payload.get("topic")?.to_string(),
            date: Utc
                .timestamp_millis_opt(payload.get("date")?.as_integer()?)
                .single()?,
        })
    }
}

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
        memory: Memory,
        embedding: Vec<f32>,
        user_id: UserId,
    ) -> anyhow::Result<()> {
        let collection_name = self.try_create_collection(user_id).await?;

        let points = vec![PointStruct::new(memory.id, embedding, memory.into())];
        self.client
            .upsert_points(UpsertPointsBuilder::new(collection_name, points))
            .await?;

        Ok(())
    }

    pub async fn search(
        &self,
        embedding: Vec<impl Into<f32>>,
        user_id: UserId,
        limit: u64,
        threshold: Option<f32>,
    ) -> anyhow::Result<Vec<Memory>> {
        let threshold = threshold.unwrap_or(self.settings.similarity_threshold);

        let embedding = embedding
            .into_iter()
            .map(|x| x.into())
            .collect::<Vec<f32>>();

        let collection_name = self.try_create_collection(user_id).await?;

        let search_result = self
            .client
            .search_points(
                SearchPointsBuilder::new(collection_name, embedding, limit)
                    // .filter(Filter::all([Condition::matches("bar", 12)]))
                    .with_payload(true), // .params(SearchParamsBuilder::default().exact(true)),
            )
            .await?;

        Ok(search_result
            .result
            .into_iter()
            .enumerate()
            .filter_map(|(i, point)| {
                let id = if let PointIdOptions::Num(id) = point.id?.point_id_options? {
                    id
                } else {
                    return None;
                };

                if point.score > threshold {
                    log::debug!(
                        "payload #{i}:\n{}\nscore: {}",
                        serde_json::to_string_pretty(&point.payload).ok()?,
                        point.score
                    );

                    Some(Memory::try_from(id, point.payload)?)
                } else {
                    None
                }
            })
            .collect())
    }

    #[allow(unused)]
    pub async fn find_recent(
        &self,
        user_id: UserId,
        limit: u32,
        range: Option<chrono::Duration>,
    ) -> anyhow::Result<Vec<Memory>> {
        let collection_name = self.try_create_collection(user_id).await?;

        let range = range.unwrap_or_else(|| chrono::Duration::days(1));
        let lower_bound_ts = (Utc::now() - range).timestamp_millis();

        // Build a filter: only return points whose "date" field is >= lower_bound_ts.
        let filter = Filter {
            must: vec![Condition {
                condition_one_of: Some(ConditionOneOf::Field(FieldCondition {
                    key: "date".to_string(),
                    range: Some(Range {
                        gt: None,
                        gte: Some(lower_bound_ts as f64),
                        lt: None,
                        lte: None,
                    }),
                    ..Default::default()
                })),
            }],
            ..Default::default()
        };

        // Use scroll_points to get points matching the filter.
        let scroll_result = self
            .client
            .scroll(
                ScrollPointsBuilder::new(collection_name)
                    .with_payload(true)
                    .filter(filter)
                    .limit(limit),
            )
            .await?;

        Ok(scroll_result
            .result
            .into_iter()
            .filter_map(|point| {
                let id = if let PointIdOptions::Num(id) = point.id?.point_id_options? {
                    id
                } else {
                    return None;
                };

                Some(Memory::try_from(id, point.payload)?)
            })
            .collect())
    }
}
