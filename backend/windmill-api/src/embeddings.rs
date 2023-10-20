use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::{self, Error, Result};
use axum::{extract::Query, routing::get, Extension, Json, Router};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{api::sync::Api, Cache, Repo};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use tinyvector::{
    db::{Db, Embedding},
    similarity::Distance,
};
use tokenizers::Tokenizer;
use tokio::sync::RwLock;
use windmill_common::{error::JsonResult, utils::http_get_from_hub};

use crate::{resources::ResourceType, HTTP_CLIENT};

#[derive(Deserialize)]
struct HubScriptsQuery {
    text: String,
    limit: Option<i64>,
    kind: Option<String>,
    app: Option<String>,
}

#[derive(Serialize)]
pub struct HubScriptResult {
    ask_id: i64,
    id: i64,
    version_id: i64,
    summary: String,
    app: String,
    kind: String,
    score: f32,
}

async fn query_hub_scripts(
    Query(query): Query<HubScriptsQuery>,
    Extension(embeddings_db): Extension<Arc<RwLock<Option<EmbeddingsDb>>>>,
) -> JsonResult<Vec<HubScriptResult>> {
    let embeddings_db = embeddings_db.read().await;

    if let Some(embeddings_db) = embeddings_db.as_ref() {
        let results = embeddings_db
            .query_hub_scripts(&query.text, query.limit, query.kind, query.app)
            .await?;

        Ok(Json(results))
    } else {
        Err(windmill_common::error::Error::InternalErr(
            "Embeddings db not initialized".to_string(),
        ))
    }
}

#[derive(Deserialize)]
struct ResourceTypesQuery {
    text: String,
    limit: Option<i64>,
}

#[derive(Serialize)]
pub struct ResourceTypeResult {
    name: String,
    score: f32,
    schema: Option<serde_json::Value>,
}
async fn query_resource_types(
    Query(query): Query<ResourceTypesQuery>,
    Extension(embeddings_db): Extension<Arc<RwLock<Option<EmbeddingsDb>>>>,
) -> JsonResult<Vec<ResourceTypeResult>> {
    let embeddings_db = embeddings_db.read().await;

    if let Some(embeddings_db) = embeddings_db.as_ref() {
        let results = embeddings_db
            .query_resource_types(&query.text, query.limit)
            .await?;

        Ok(Json(results))
    } else {
        Err(windmill_common::error::Error::InternalErr(
            "Embeddings db not initialized".to_string(),
        ))
    }
}

#[derive(Deserialize, Debug, Clone)]
struct HubScript {
    ask_id: i64,
    id: i64,
    version_id: i64,
    summary: String,
    app: String,
    kind: String,
    embedding: Vec<f32>,
}

#[derive(Deserialize, Debug)]
struct HubResourceType {
    name: String,
    embedding: Vec<f32>,
}

pub struct ModelInstance {
    model: BertModel,
    tokenizer: Tokenizer,
}

impl ModelInstance {
    pub async fn load_model_files() -> Result<(PathBuf, PathBuf, PathBuf)> {
        let repo = Repo::model("thenlper/gte-small".to_string());

        let cache = Cache::default().repo(repo.clone());

        let api = Api::new()?;
        let api = api.repo(repo);

        let (config_filename, tokenizer_filename, weights_filename) = (
            cache
                .get("config.json")
                .or_else(|| api.get("config.json").ok())
                .ok_or(Error::msg("could not get config.json"))?,
            cache
                .get("tokenizer.json")
                .or_else(|| api.get("tokenizer.json").ok())
                .ok_or(Error::msg("could not get tokenizer.json"))?,
            cache
                .get("model.safetensors")
                .and_then(|p| {
                    tracing::info!("Found embedding model in cache");
                    Some(p)
                })
                .or_else(|| {
                    tracing::info!("Downloading embedding model...");
                    api.get("model.safetensors").ok().and_then(|p| {
                        tracing::info!("Downloaded embedding model");
                        Some(p)
                    })
                })
                .ok_or(Error::msg("could not get model.safetensors"))?,
        );

        Ok((config_filename, tokenizer_filename, weights_filename))
    }

    pub async fn new() -> Result<Self> {
        tracing::info!("Loading embedding model...");
        let device = Device::Cpu;
        let (config_filename, tokenizer_filename, weights_filename) =
            Self::load_model_files().await?;
        let config = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config)?;
        let tokenizer = Tokenizer::from(
            Tokenizer::from_file(tokenizer_filename)
                .map_err(Error::msg)?
                .with_padding(None)
                .with_truncation(None)
                .map_err(Error::msg)?
                .to_owned(),
        );

        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };
        let model = BertModel::load(vb, &config)?;
        tracing::info!("Loaded embedding model");
        Ok(Self { model, tokenizer })
    }

    pub async fn create_embedding(self: Arc<Self>, sentence: &str) -> Result<Vec<f32>> {
        let sentence = sentence.to_owned();
        tokio::task::spawn_blocking(move || {
            let tokens = self
                .tokenizer
                .encode(sentence, true)
                .map_err(Error::msg)?
                .get_ids()
                .to_vec();

            let token_ids = Tensor::new(&tokens[..], &Device::Cpu)?.unsqueeze(0)?;
            let token_type_ids = token_ids.zeros_like()?;

            let embedding = self.model.forward(&token_ids, &token_type_ids)?;
            let embedding = (embedding.sum(1)? / embedding.dim(1)? as f64)?;
            let embedding = normalize_l2(&embedding)?;

            let embedding = embedding.get(0)?.to_vec1()?;

            Ok(embedding)
        })
        .await?
    }
}

pub struct EmbeddingsDb {
    db: Db,
    model_instance: Arc<ModelInstance>,
}

impl EmbeddingsDb {
    pub async fn new(pg_db: &Pool<Postgres>, model_instance: Arc<ModelInstance>) -> Result<Self> {
        let db = Db::new();

        let mut embeddings_db = Self { db, model_instance: model_instance.clone() };

        embeddings_db.fill_db(pg_db).await?;

        Ok(embeddings_db)
    }

    async fn fill_db(&mut self, pg_db: &Pool<Postgres>) -> Result<()> {
        if self.db.get_collection("scripts").is_some() {
            self.db.delete_collection("scripts")?;
        }

        self.db
            .create_collection("scripts".to_string(), 384, Distance::Cosine)?;

        if self.db.get_collection("resource_types").is_some() {
            self.db.delete_collection("resource_types")?;
        }

        self.db
            .create_collection("resource_types".to_string(), 384, Distance::Cosine)?;

        let response = http_get_from_hub(
            &HTTP_CLIENT,
            "https://hub.windmill.dev/scripts/embeddings",
            "todo@windmill.dev",
            false,
            None,
        )
        .await?;
        let hub_scripts = response.json::<Vec<HubScript>>().await?;

        for script in &hub_scripts {
            let mut hm = HashMap::new();
            hm.insert("ask_id".to_string(), script.ask_id.clone().to_string());
            hm.insert("summary".to_string(), script.summary.clone());
            hm.insert("app".to_string(), script.app.clone());
            hm.insert("kind".to_string(), script.kind.clone());
            hm.insert("id".to_string(), script.id.clone().to_string());
            hm.insert(
                "version_id".to_string(),
                script.version_id.clone().to_string(),
            );
            let embedding = Embedding {
                id: script.ask_id.clone().to_string(),
                vector: script.embedding.clone(),
                metadata: Some(hm),
            };
            self.db.insert_into_collection("scripts", embedding)?;
        }

        let response = http_get_from_hub(
            &HTTP_CLIENT,
            "https://hub.windmill.dev/resource_types/embeddings",
            "todo@windmill.dev",
            false,
            None,
        )
        .await?;
        let hub_resource_types = response.json::<Vec<HubResourceType>>().await?;

        let resource_types: Vec<ResourceType> = sqlx::query_as!(
            ResourceType,
            "SELECT * from resource_type ORDER \
             BY name",
        )
        .fetch_all(pg_db)
        .await?;

        for rt in resource_types {
            let mut hm = HashMap::new();
            hm.insert("name".to_string(), rt.name.clone());
            if let Some(schema) = rt.schema.clone() {
                hm.insert("schema".to_string(), serde_json::to_string(&schema)?);
            }
            let hub_rt = hub_resource_types.iter().find(|hrt| hrt.name == rt.name);

            let vector = if let Some(hub_rt) = hub_rt {
                hub_rt.embedding.clone()
            } else {
                self.model_instance
                    .clone()
                    .create_embedding(&format!(
                        "{};{}",
                        rt.name,
                        rt.description.unwrap_or_default()
                    ))
                    .await?
            };

            let embedding = Embedding {
                id: format!("{}_{}", rt.workspace_id, rt.name),
                vector,
                metadata: Some(hm),
            };

            self.db
                .insert_into_collection("resource_types", embedding)?;
        }

        Ok(())
    }

    pub async fn query_hub_scripts(
        &self,
        query: &str,
        limit: Option<i64>,
        kind: Option<String>,
        app: Option<String>,
    ) -> Result<Vec<HubScriptResult>> {
        let model_instance = self.model_instance.clone();
        let query_embedding = model_instance.create_embedding(query).await?;

        let collection = self.db.get_collection("scripts");

        let collection = collection.ok_or(Error::msg("no collection found"))?;

        let filter = |embedding: &Embedding| {
            if let Some(metadata) = embedding.metadata.as_ref() {
                match (
                    metadata.get("kind"),
                    kind.clone(),
                    metadata.get("app"),
                    app.clone(),
                ) {
                    (Some(script_kind), Some(kind), Some(script_app), Some(app)) => {
                        &kind == script_kind && &app == script_app
                    }
                    (Some(script_kind), Some(kind), _, _) => &kind == script_kind,
                    (_, _, Some(script_app), Some(app)) => &app == script_app,
                    (_, None, _, None) => true,
                    _ => false,
                }
            } else {
                false
            }
        };

        let results = collection.get_similarity(
            &query_embedding,
            limit.unwrap_or(10) as usize,
            Some(&filter),
            Some(0.75),
        );

        let results: Result<Vec<_>> = results
            .iter()
            .map(|r| {
                let metadata = r
                    .embedding
                    .metadata
                    .as_ref()
                    .ok_or(Error::msg("no metadata"))?;

                Ok(HubScriptResult {
                    ask_id: metadata
                        .get("ask_id")
                        .ok_or(Error::msg("no ask_id"))?
                        .parse::<i64>()?,
                    summary: metadata
                        .get("summary")
                        .ok_or(Error::msg("no summary"))?
                        .to_owned(),
                    app: metadata.get("app").ok_or(Error::msg("no app"))?.to_owned(),
                    kind: metadata
                        .get("kind")
                        .ok_or(Error::msg("no kind"))?
                        .to_owned(),
                    id: metadata
                        .get("id")
                        .ok_or(Error::msg("no id"))?
                        .parse::<i64>()?,
                    version_id: metadata
                        .get("version_id")
                        .ok_or(Error::msg("no version_id"))?
                        .parse::<i64>()?,
                    score: r.score,
                })
            })
            .collect();

        results
    }

    pub async fn query_resource_types(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<ResourceTypeResult>> {
        let model_instance = self.model_instance.clone();
        let query_embedding = model_instance.create_embedding(query).await?;

        let collection = self.db.get_collection("resource_types");

        if collection.is_none() {
            return Ok(vec![]);
        }

        let collection = collection.ok_or(Error::msg("no collection found"))?;
        let results = collection.get_similarity(
            &query_embedding,
            limit.unwrap_or(10) as usize,
            None,
            Some(0.75),
        );

        let results: Result<_> = results
            .iter()
            .map(|r| {
                let metadata = r
                    .embedding
                    .metadata
                    .as_ref()
                    .ok_or(Error::msg("no metadata"))?;
                Ok(ResourceTypeResult {
                    name: metadata
                        .get("name")
                        .ok_or(Error::msg("no name"))?
                        .to_owned(),
                    schema: match metadata.get("schema") {
                        Some(schema) => serde_json::from_str(schema)?,
                        None => None,
                    },
                    score: r.score,
                })
            })
            .collect();

        results
    }
}

fn normalize_l2(v: &Tensor) -> Result<Tensor> {
    Ok(v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)?)
}

pub fn global_service(db: &Pool<Postgres>) -> Router {
    let embeddings_db: Arc<RwLock<Option<EmbeddingsDb>>> = Arc::new(RwLock::new(None));

    let disable_embedding = std::env::var("DISABLE_EMBEDDING")
        .ok()
        .map(|x| x.parse::<bool>().unwrap_or(false))
        .unwrap_or(false);

    if !disable_embedding {
        let db_clone = db.clone();
        let embeddings_clone: Arc<RwLock<Option<EmbeddingsDb>>> = embeddings_db.clone();
        tokio::spawn(async move {
            let model_instance = ModelInstance::new().await;

            if let Ok(model_instance) = model_instance {
                let model_instance = Arc::new(model_instance);
                loop {
                    tracing::info!("Creating embeddings DB...");
                    let new_embeddings_db =
                        EmbeddingsDb::new(&db_clone, model_instance.clone()).await;
                    if let Err(e) = new_embeddings_db.as_ref() {
                        tracing::error!("Failed to create embeddings db: {}", e);
                    } else {
                        let mut embeddings_db = embeddings_clone.write().await;
                        *embeddings_db = new_embeddings_db.ok();
                        tracing::info!("Created embeddings DB");
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(3600 * 24)).await;
                }
            } else {
                tracing::error!(
                    "Failed to initialize model instance: {}",
                    model_instance.err().unwrap()
                );
            }
        });
    }

    Router::new()
        .route("/query_hub_scripts", get(query_hub_scripts))
        .route("/query_resource_types", get(query_resource_types))
        .layer(Extension(embeddings_db))
}
