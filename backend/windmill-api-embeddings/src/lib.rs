#[cfg(feature = "embedding")]
use anyhow::{anyhow, Error, Result};
#[cfg(feature = "embedding")]
use std::{collections::HashMap, path::PathBuf, sync::Arc};
#[cfg(feature = "embedding")]
use windmill_common::DEFAULT_HUB_BASE_URL;
#[cfg(feature = "embedding")]
use windmill_common::HUB_BASE_URL;
#[cfg(feature = "embedding")]
use windmill_common::utils::HTTP_CLIENT_PERMISSIVE as HTTP_CLIENT;

use axum::Router;

#[cfg(feature = "embedding")]
use axum::{
    extract::{Path, Query},
    Json,
};

#[cfg(feature = "embedding")]
use axum::routing::get;
#[cfg(feature = "embedding")]
use candle_core::{Device, Tensor};
#[cfg(feature = "embedding")]
use candle_nn::VarBuilder;
#[cfg(feature = "embedding")]
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
#[cfg(feature = "embedding")]
use hf_hub::api::tokio::Api;
#[cfg(feature = "embedding")]
use serde::Deserialize;
#[cfg(feature = "embedding")]
use serde::Serialize;
#[cfg(feature = "embedding")]
use sqlx::{Pool, Postgres};
#[cfg(feature = "embedding")]
use tinyvector::{
    db::{Db, Embedding},
    similarity::Distance,
};
#[cfg(feature = "embedding")]
use tokenizers::Tokenizer;
#[cfg(feature = "embedding")]
use tokio::sync::RwLock;
#[cfg(feature = "embedding")]
use windmill_common::utils::http_get_from_hub;

#[cfg(feature = "embedding")]
use windmill_common::error::JsonResult;

#[cfg(feature = "embedding")]
use windmill_store::resources::ResourceType;

#[cfg(feature = "embedding")]
lazy_static::lazy_static! {
    pub static ref EMBEDDINGS_DB: Arc<RwLock<Option<EmbeddingsDb>>> = Arc::new(RwLock::new(None));
    pub static ref MODEL_INSTANCE: Arc<RwLock<Option<Arc<ModelInstance>>>> = Arc::new(RwLock::new(None));
    pub static ref HUB_EMBEDDINGS_PULLING_INTERVAL_SECS: u64 = std::env::var("HUB_EMBEDDINGS_PULLING_INTERVAL_SECS").ok().map(|x| x.parse::<u64>().ok()).flatten().unwrap_or(3600 * 24);
}

#[cfg(feature = "embedding")]
#[derive(Deserialize)]
struct HubScriptsQuery {
    text: String,
    limit: Option<i64>,
    kind: Option<String>,
    app: Option<String>,
}

#[cfg(feature = "embedding")]
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

#[cfg(feature = "embedding")]
async fn query_hub_scripts(
    Query(query): Query<HubScriptsQuery>,
) -> JsonResult<Vec<HubScriptResult>> {
    let embeddings_db = EMBEDDINGS_DB.read().await;

    if let Some(embeddings_db) = embeddings_db.as_ref() {
        let results = embeddings_db
            .query_hub_scripts(&query.text, query.limit, query.kind, query.app)
            .await?;

        Ok(Json(results))
    } else {
        Err(windmill_common::error::Error::internal_err(
            "Embeddings db not initialized".to_string(),
        ))
    }
}

#[cfg(feature = "embedding")]
#[derive(Deserialize)]
struct ResourceTypesQuery {
    text: String,
    limit: Option<i64>,
}

#[cfg(feature = "embedding")]
#[derive(Serialize)]
pub struct ResourceTypeResult {
    name: String,
    score: f32,
    schema: Option<serde_json::Value>,
}
#[cfg(feature = "embedding")]
async fn query_resource_types(
    Query(query): Query<ResourceTypesQuery>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ResourceTypeResult>> {
    let embeddings_db = EMBEDDINGS_DB.read().await;

    if let Some(embeddings_db) = embeddings_db.as_ref() {
        let results = embeddings_db
            .query_resource_types(w_id, &query.text, query.limit)
            .await?;

        Ok(Json(results))
    } else {
        Err(windmill_common::error::Error::internal_err(
            "Embeddings db not initialized".to_string(),
        ))
    }
}

#[cfg(feature = "embedding")]
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

#[cfg(feature = "embedding")]
#[derive(Deserialize, Debug)]
struct HubResourceType {
    name: String,
    embedding: Vec<f32>,
}

#[cfg(feature = "embedding")]
pub struct ModelInstance {
    model: BertModel,
    tokenizer: Tokenizer,
}

#[cfg(feature = "embedding")]
impl ModelInstance {
    pub async fn load_model_files() -> Result<(PathBuf, PathBuf, PathBuf)> {
        let api = Api::new()?;
        let repo_api = api.model("thenlper/gte-small".to_string());

        let (config_filename, tokenizer_filename, weights_filename) =
            (
                repo_api
                    .get("config.json")
                    .await
                    .map_err(|e| anyhow!("Failed to get config.json from hugging face: {}", e))?,
                repo_api.get("tokenizer.json").await.map_err(|e| {
                    anyhow!("Failed to get tokenizer.json from hugging face: {}", e)
                })?,
                repo_api.get("model.safetensors").await.map_err(|e| {
                    anyhow!("Failed to get model.safetensors from hugging face: {}", e)
                })?,
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

            let embedding = self.model.forward(&token_ids, &token_type_ids, None)?;
            let embedding = (embedding.sum(1)? / embedding.dim(1)? as f64)?;
            let embedding = normalize_l2(&embedding)?;

            let embedding = embedding.get(0)?.to_vec1()?;

            Ok(embedding)
        })
        .await?
    }
}

#[cfg(feature = "embedding")]
pub struct EmbeddingsDb {
    db: Db,
    model_instance: Arc<ModelInstance>,
}

#[cfg(feature = "embedding")]
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

        let hub_base_url = HUB_BASE_URL.read().await.clone();

        let response = match hub_base_url.as_str() {
            DEFAULT_HUB_BASE_URL => {
                let response = HTTP_CLIENT
                    .get("https://bucket.windmillhub.com/embeddings/scripts_embeddings.json")
                    .send()
                    .await;

                if response.is_err() || response.as_ref().unwrap().error_for_status_ref().is_err() {
                    tracing::warn!("Failed to get scripts embeddings from bucket, trying hub...");
                    http_get_from_hub(
                        &HTTP_CLIENT,
                        &format!("{}/scripts/embeddings", hub_base_url),
                        false,
                        None,
                        Some(pg_db),
                    )
                    .await?
                } else {
                    response.unwrap()
                }
            }
            _ => {
                http_get_from_hub(
                    &HTTP_CLIENT,
                    &format!("{}/scripts/embeddings", hub_base_url),
                    false,
                    None,
                    Some(pg_db),
                )
                .await?
            }
        };

        if response.error_for_status_ref().is_err() {
            return Err(anyhow!(
                "Failed to get scripts embeddings from hub with error code: {}",
                response.status()
            ));
        }

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

        let response = match hub_base_url.as_str() {
            DEFAULT_HUB_BASE_URL => {
                let response = HTTP_CLIENT
                    .get("https://bucket.windmillhub.com/embeddings/resource_types_embeddings.json")
                    .send()
                    .await;
                if response.is_err() || response.as_ref().unwrap().error_for_status_ref().is_err() {
                    tracing::warn!(
                        "Failed to get resource types embeddings from bucket, trying hub..."
                    );
                    http_get_from_hub(
                        &HTTP_CLIENT,
                        &format!("{}/resource_types/embeddings", hub_base_url),
                        false,
                        None,
                        Some(pg_db),
                    )
                    .await?
                } else {
                    response.unwrap()
                }
            }
            _ => {
                http_get_from_hub(
                    &HTTP_CLIENT,
                    &format!("{}/resource_types/embeddings", hub_base_url),
                    false,
                    None,
                    Some(pg_db),
                )
                .await?
            }
        };

        if response.error_for_status_ref().is_err() {
            return Err(anyhow!(
                "Failed to get resource types embeddings from hub with error code: {}",
                response.status()
            ));
        }
        let hub_resource_types = response.json::<Vec<HubResourceType>>().await?;

        let resource_types: Vec<ResourceType> =
            sqlx::query_as!(ResourceType, "SELECT * from resource_type ORDER BY name",)
                .fetch_all(pg_db)
                .await?;

        for rt in resource_types {
            let mut hm = HashMap::new();
            hm.insert("name".to_string(), rt.name.clone());
            if let Some(schema) = rt.schema.clone() {
                hm.insert("schema".to_string(), serde_json::to_string(&schema)?);
            }
            hm.insert("workspace".to_string(), rt.workspace_id.clone());
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
            Some(0.8),
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

        let mut results = results?;

        if results.len() > 1 {
            let top_score = results[0].score;
            results = results
                .into_iter()
                .take_while(|r| (top_score - r.score) / top_score <= 0.05)
                .collect();
        }

        Ok(results)
    }

    pub async fn query_resource_types(
        &self,
        workspace: String,
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

        let filter = |embedding: &Embedding| {
            if let Some(metadata) = embedding.metadata.as_ref() {
                match metadata.get("workspace").map(|x| x.as_str()) {
                    Some("admins") => true,
                    Some(rt_workspace) => &workspace == rt_workspace,
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

#[cfg(feature = "embedding")]
fn normalize_l2(v: &Tensor) -> Result<Tensor> {
    Ok(v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)?)
}

#[cfg(feature = "embedding")]
pub fn load_embeddings_db(db: &Pool<Postgres>) -> () {
    let disable_embedding = std::env::var("DISABLE_EMBEDDING")
        .ok()
        .map(|x| x.parse::<bool>().unwrap_or(false))
        .unwrap_or(false);

    if !disable_embedding {
        let db_clone = db.clone();
        tokio::spawn(async move {
            let model_instance = ModelInstance::new().await;
            if let Ok(model_instance) = model_instance {
                let mut model_instance_lock = MODEL_INSTANCE.write().await;
                *model_instance_lock = Some(Arc::new(model_instance));
                drop(model_instance_lock);
                loop {
                    update_embeddings_db(&db_clone).await;
                    tokio::time::sleep(std::time::Duration::from_secs(
                        *HUB_EMBEDDINGS_PULLING_INTERVAL_SECS,
                    ))
                    .await;
                }
            } else {
                tracing::error!(
                    "Failed to initialize model instance: {}",
                    model_instance.err().unwrap()
                );
            }
        });
    }
}

#[cfg(feature = "embedding")]
pub async fn update_embeddings_db(db: &Pool<Postgres>) -> () {
    if let Some(model_instance) = MODEL_INSTANCE.read().await.as_ref() {
        tracing::info!("Creating embeddings DB...");
        let new_embeddings_db = EmbeddingsDb::new(&db, model_instance.clone()).await;
        if let Err(e) = new_embeddings_db.as_ref() {
            tracing::error!("Failed to create embeddings db: {}", e);
        } else {
            let mut embeddings_db = EMBEDDINGS_DB.write().await;
            *embeddings_db = new_embeddings_db.ok();
            tracing::info!("Created embeddings DB");
        }
    } else {
        tracing::error!("Could not update embeddings DB, model instance not initialized");
    }
}

#[cfg(feature = "embedding")]
pub fn workspaced_service() -> Router {
    Router::new().route("/query_resource_types", get(query_resource_types))
}

#[cfg(feature = "embedding")]
pub fn global_service() -> Router {
    Router::new().route("/query_hub_scripts", get(query_hub_scripts))
}

#[cfg(not(feature = "embedding"))]
pub fn workspaced_service() -> Router {
    Router::new()
}

#[cfg(not(feature = "embedding"))]
pub fn global_service() -> Router {
    Router::new()
}
