#[cfg(feature = "embedding")]
use anyhow::{anyhow, Error, Result};
#[cfg(feature = "embedding")]
use std::{collections::HashMap, path::PathBuf, sync::Arc};
#[cfg(feature = "embedding")]
use windmill_common::utils::HTTP_CLIENT_PERMISSIVE as HTTP_CLIENT;
#[cfg(feature = "embedding")]
use windmill_common::DEFAULT_HUB_BASE_URL;
#[cfg(feature = "embedding")]
use windmill_common::HUB_BASE_URL;

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
#[cfg(feature = "embedding")]
use candle_core::{Device, Tensor};
#[cfg(feature = "embedding")]
use candle_nn::VarBuilder;
#[cfg(feature = "embedding")]
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
#[cfg(feature = "embedding")]
use hf_hub::api::tokio::Api;
use serde::{Deserialize, Serialize};
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
use windmill_common::{db::DB, error::JsonResult};

#[cfg(feature = "embedding")]
use windmill_store::resources::ResourceType;

#[cfg(feature = "embedding")]
lazy_static::lazy_static! {
    pub static ref EMBEDDINGS_DB: Arc<RwLock<Option<EmbeddingsDb>>> = Arc::new(RwLock::new(None));
    pub static ref MODEL_INSTANCE: Arc<RwLock<Option<Arc<ModelInstance>>>> = Arc::new(RwLock::new(None));
    pub static ref HUB_EMBEDDINGS_PULLING_INTERVAL_SECS: u64 = std::env::var("HUB_EMBEDDINGS_PULLING_INTERVAL_SECS").ok().map(|x| x.parse::<u64>().ok()).flatten().unwrap_or(3600 * 24);
    // On a failed init/refresh we retry after this short interval instead of the
    // full pulling interval, so a transient startup error doesn't leave the
    // embeddings DB uninitialized for a whole day.
    pub static ref HUB_EMBEDDINGS_RETRY_INTERVAL_SECS: u64 = std::env::var("HUB_EMBEDDINGS_RETRY_INTERVAL_SECS").ok().map(|x| x.parse::<u64>().ok()).flatten().unwrap_or(60);
    pub static ref RESOURCE_TYPE_EMBEDDINGS_CHECK_INTERVAL_SECS: u64 = std::env::var("RESOURCE_TYPE_EMBEDDINGS_CHECK_INTERVAL_SECS").ok().map(|x| x.parse::<u64>().ok()).flatten().unwrap_or(60);
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
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
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

/// Drop results whose score falls more than `max_relative_drop` below the best
/// match, so a strong hit isn't diluted by weakly-related entries that merely
/// clear the similarity floor. Expects `results` sorted by descending score and
/// a positive top score (guaranteed by the caller's similarity threshold).
#[cfg(feature = "embedding")]
fn trim_to_top_score<T>(
    results: Vec<T>,
    max_relative_drop: f32,
    score: impl Fn(&T) -> f32,
) -> Vec<T> {
    if results.len() <= 1 {
        return results;
    }
    let top_score = score(&results[0]);
    results
        .into_iter()
        .take_while(|r| (top_score - score(r)) / top_score <= max_relative_drop)
        .collect()
}
async fn query_resource_types(
    Extension(db): Extension<DB>,
    Query(query): Query<ResourceTypesQuery>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<ResourceTypeResult>> {
    #[cfg(feature = "embedding")]
    if let Some(embeddings_db) = EMBEDDINGS_DB.read().await.as_ref() {
        return Ok(Json(
            embeddings_db
                .query_resource_types(w_id, &query.text, query.limit)
                .await?,
        ));
    }
    Ok(Json(
        match_resource_types_by_text(&db, &w_id, &query.text, query.limit).await?,
    ))
}

/// Serves the search when there is no embeddings index: a build without the `embedding`
/// feature, or one whose index never loaded (air-gapped, `DISABLE_EMBEDDING`). The AI chat
/// has no other way to read the instance's resource types.
async fn match_resource_types_by_text(
    db: &DB,
    w_id: &str,
    text: &str,
    limit: Option<i64>,
) -> windmill_common::error::Result<Vec<ResourceTypeResult>> {
    let words: Vec<String> = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect();
    let rows: Vec<(String, Option<String>, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT name, description, schema FROM resource_type WHERE workspace_id = $1 OR workspace_id = 'admins'",
    )
    .bind(w_id)
    .fetch_all(db)
    .await?;

    let mut results: Vec<ResourceTypeResult> = rows
        .into_iter()
        .map(|(name, description, schema)| ResourceTypeResult {
            score: text_match_score(&words, &name, description.as_deref().unwrap_or_default()),
            name,
            schema,
        })
        .filter(|rt| rt.score > 0.0)
        .collect();
    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.name.cmp(&b.name))
    });
    results.truncate(limit.unwrap_or(10).max(0) as usize);
    Ok(results)
}

/// A word found in the name counts twice a word found in the description: names are what
/// a query for a service spells out ("postgres" in `postgresql`, "sheets" in `gsheets`).
/// A word under 3 characters ("s3", "ai") would sit inside too many names, so it counts only
/// as a whole name or one of its `_`-separated parts.
fn text_match_score(words: &[String], name: &str, description: &str) -> f32 {
    let name = name.to_lowercase();
    let description = description.to_lowercase();
    words
        .iter()
        .map(|word| {
            let short = word.len() < 3;
            let in_name = if short {
                name.split('_').any(|part| part == word)
            } else {
                name.contains(word.as_str())
            };
            if in_name {
                2.0
            } else if !short && description.contains(word.as_str()) {
                1.0
            } else {
                0.0
            }
        })
        .sum()
}

#[cfg(feature = "embedding")]
#[derive(Deserialize, Debug, Clone)]
struct HubScript {
    ask_id: i64,
    id: i64,
    version_id: i64,
    summary: String,
    // Nearly a fifth of hub scripts carry an explicit `"description": null`; a bare
    // String here fails the whole blob and takes hub search down with it.
    description: Option<String>,
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
    async fn get_hf_file(
        repo_api: &hf_hub::api::tokio::ApiRepo,
        filename: &str,
    ) -> Result<PathBuf> {
        // HuggingFace downloads have no built-in retry, so a single transient
        // network blip would fail the whole image build. Retry with exponential
        // backoff (1s, 2s, 4s, 8s, capped at 8s) up to MAX_ATTEMPTS times.
        const MAX_ATTEMPTS: u32 = 5;
        let mut attempt: u32 = 0;
        loop {
            attempt += 1;
            match repo_api.get(filename).await {
                Ok(path) => return Ok(path),
                Err(e) => {
                    if attempt >= MAX_ATTEMPTS {
                        return Err(anyhow!(
                            "Failed to get {} from hugging face after {} attempts: {}",
                            filename,
                            attempt,
                            e
                        ));
                    }
                    let delay_secs = 1u64 << (attempt - 1).min(3);
                    tracing::warn!(
                        "Failed to get {} from hugging face (attempt {}/{}): {}. Retrying in {}s...",
                        filename,
                        attempt,
                        MAX_ATTEMPTS,
                        e,
                        delay_secs
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                }
            }
        }
    }

    pub async fn load_model_files() -> Result<(PathBuf, PathBuf, PathBuf)> {
        let api = Api::new()?;
        let repo_api = api.model("thenlper/gte-small".to_string());

        let (config_filename, tokenizer_filename, weights_filename) = (
            Self::get_hf_file(&repo_api, "config.json").await?,
            Self::get_hf_file(&repo_api, "tokenizer.json").await?,
            Self::get_hf_file(&repo_api, "model.safetensors").await?,
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
    /// Kept from the last full fill, so the resource types can be re-indexed between fills
    /// without downloading the hub's embeddings again.
    hub_resource_type_embeddings: Arc<HashMap<String, Vec<f32>>>,
    /// The `resource_types_fingerprint` the resource types collection was built from.
    resource_types_fingerprint: Option<String>,
    /// See `ResourceTypeIndex::custom_vectors`.
    custom_resource_type_vectors: Arc<HashMap<String, Vec<f32>>>,
}

/// A built resource types collection, with what a later refresh needs to tell it is stale and
/// to reuse its vectors.
#[cfg(feature = "embedding")]
struct ResourceTypeIndex {
    fingerprint: Option<String>,
    embeddings: Vec<Embedding>,
    /// Vectors of the types the hub has none for, by the `name;description` they embed: a
    /// refresh runs the model only on types that are new or changed, not on every custom type.
    custom_vectors: Arc<HashMap<String, Vec<f32>>>,
}

#[cfg(feature = "embedding")]
impl EmbeddingsDb {
    fn empty(model_instance: Arc<ModelInstance>) -> Self {
        Self {
            db: Db::new(),
            model_instance,
            hub_resource_type_embeddings: Arc::new(HashMap::new()),
            resource_types_fingerprint: None,
            custom_resource_type_vectors: Arc::new(HashMap::new()),
        }
    }

    pub async fn new(pg_db: &Pool<Postgres>, model_instance: Arc<ModelInstance>) -> Result<Self> {
        let mut embeddings_db = Self::empty(model_instance);
        embeddings_db.fill_db(pg_db).await?;
        Ok(embeddings_db)
    }

    /// The resource types alone, all embedded locally, for when the full fill cannot download
    /// the hub's embeddings: an air-gapped instance, or a private hub serving none. Hub script
    /// search keeps failing, since it has no collection to search.
    async fn resource_types_only(
        pg_db: &Pool<Postgres>,
        model_instance: Arc<ModelInstance>,
    ) -> Result<Self> {
        let mut embeddings_db = Self::empty(model_instance);
        let index = resource_type_index(
            pg_db,
            &embeddings_db.model_instance,
            &embeddings_db.hub_resource_type_embeddings,
            &embeddings_db.custom_resource_type_vectors,
        )
        .await?;
        embeddings_db.replace_resource_types(index)?;
        Ok(embeddings_db)
    }

    async fn fill_db(&mut self, pg_db: &Pool<Postgres>) -> Result<()> {
        if self.db.get_collection("scripts").is_some() {
            self.db.delete_collection("scripts")?;
        }

        self.db
            .create_collection("scripts".to_string(), 384, Distance::Cosine)?;

        let hub_base_url = (**HUB_BASE_URL.load()).clone();

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
            hm.insert(
                "description".to_string(),
                script.description.clone().unwrap_or_default(),
            );
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
        self.hub_resource_type_embeddings = Arc::new(
            hub_resource_types
                .into_iter()
                .map(|rt| (rt.name, rt.embedding))
                .collect(),
        );

        let index = resource_type_index(
            pg_db,
            &self.model_instance,
            &self.hub_resource_type_embeddings,
            &self.custom_resource_type_vectors,
        )
        .await?;
        self.replace_resource_types(index)
    }

    fn replace_resource_types(&mut self, index: ResourceTypeIndex) -> Result<()> {
        if self.db.get_collection("resource_types").is_some() {
            self.db.delete_collection("resource_types")?;
        }
        self.db
            .create_collection("resource_types".to_string(), 384, Distance::Cosine)?;
        for embedding in index.embeddings {
            self.db
                .insert_into_collection("resource_types", embedding)?;
        }
        self.resource_types_fingerprint = index.fingerprint;
        self.custom_resource_type_vectors = index.custom_vectors;
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
                    description: metadata
                        .get("description")
                        .filter(|d| !d.is_empty())
                        .map(|d| d.to_owned()),
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

        let results = trim_to_top_score(results?, 0.05, |r| r.score);

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

        let results: Result<Vec<ResourceTypeResult>> = results
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

        let results = trim_to_top_score(results?, 0.05, |r| r.score);

        Ok(results)
    }
}

/// Covers everything an indexed resource type is built from, whichever path wrote it: the
/// create and update routes, a hub sync's direct upserts, a delete.
///
/// A sum of per-row hashes needs no sort. Don't go back to an ORDER BY aggregate: sorting
/// every schema spills to disk once they exceed work_mem, on every check of every server.
#[cfg(feature = "embedding")]
async fn resource_types_fingerprint(pg_db: &Pool<Postgres>) -> Result<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT count(*)::text || ':' || coalesce(sum(('x' || left(md5(workspace_id || '/' || name || '/' || coalesce(description, '') || '/' || coalesce(schema::text, '')), 15))::bit(60)::bigint), 0)::text FROM resource_type",
    )
    .fetch_one(pg_db)
    .await?)
}

/// The fingerprint is read before the rows: a write landing in between leaves it stale, so
/// the next check indexes again rather than keeping an index that misses the write.
#[cfg(feature = "embedding")]
async fn resource_type_index(
    pg_db: &Pool<Postgres>,
    model_instance: &Arc<ModelInstance>,
    hub_embeddings: &HashMap<String, Vec<f32>>,
    known_custom_vectors: &HashMap<String, Vec<f32>>,
) -> Result<ResourceTypeIndex> {
    let fingerprint = resource_types_fingerprint(pg_db).await?;
    let resource_types: Vec<ResourceType> =
        sqlx::query_as!(ResourceType, "SELECT workspace_id, name, schema, description, created_by, edited_at, format_extension, is_fileset, display_name from resource_type ORDER BY name",)
            .fetch_all(pg_db)
            .await?;

    let mut embeddings = Vec::with_capacity(resource_types.len());
    let mut custom_vectors = HashMap::new();
    for rt in resource_types {
        let mut hm = HashMap::new();
        hm.insert("name".to_string(), rt.name.clone());
        if let Some(schema) = rt.schema.clone() {
            hm.insert("schema".to_string(), serde_json::to_string(&schema)?);
        }
        hm.insert("workspace".to_string(), rt.workspace_id.clone());

        let vector = match hub_embeddings.get(&rt.name) {
            Some(vector) => vector.clone(),
            None => {
                let text = format!("{};{}", rt.name, rt.description.unwrap_or_default());
                let vector = match known_custom_vectors
                    .get(&text)
                    .or_else(|| custom_vectors.get(&text))
                {
                    Some(vector) => vector.clone(),
                    None => model_instance.clone().create_embedding(&text).await?,
                };
                custom_vectors.insert(text, vector.clone());
                vector
            }
        };

        embeddings.push(Embedding {
            id: format!("{}_{}", rt.workspace_id, rt.name),
            vector,
            metadata: Some(hm),
        });
    }
    Ok(ResourceTypeIndex { fingerprint, embeddings, custom_vectors: Arc::new(custom_vectors) })
}

/// The full fill runs once a day, so a resource type created or synced in between would stay
/// unsearchable until then; this re-indexes the resource types alone when the table changed.
#[cfg(feature = "embedding")]
pub async fn refresh_resource_type_embeddings(pg_db: &Pool<Postgres>) -> Result<()> {
    if EMBEDDINGS_DB.read().await.is_none() {
        return Ok(());
    }
    let fingerprint = resource_types_fingerprint(pg_db).await?;
    let (model_instance, hub_embeddings, custom_vectors) = match EMBEDDINGS_DB.read().await.as_ref()
    {
        Some(db) if db.resource_types_fingerprint != fingerprint => (
            db.model_instance.clone(),
            db.hub_resource_type_embeddings.clone(),
            db.custom_resource_type_vectors.clone(),
        ),
        _ => return Ok(()),
    };
    let index =
        resource_type_index(pg_db, &model_instance, &hub_embeddings, &custom_vectors).await?;
    if let Some(db) = EMBEDDINGS_DB.write().await.as_mut() {
        db.replace_resource_types(index)?;
    }
    Ok(())
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
        let resource_types_db = db.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(
                    *RESOURCE_TYPE_EMBEDDINGS_CHECK_INTERVAL_SECS,
                ))
                .await;
                if let Err(e) = refresh_resource_type_embeddings(&resource_types_db).await {
                    tracing::warn!("Failed to refresh resource type embeddings: {e:#}");
                }
            }
        });

        let db_clone = db.clone();
        tokio::spawn(async move {
            // Keep retrying model init: a transient failure here must not
            // permanently disable embeddings until the next process restart.
            // Backoff decays to the pulling interval so an environment where it
            // can never succeed (e.g. air-gapped, embeddings left enabled)
            // settles into ~1 attempt/interval rather than a tight error loop.
            let mut backoff_secs = *HUB_EMBEDDINGS_RETRY_INTERVAL_SECS;
            loop {
                match ModelInstance::new().await {
                    Ok(model_instance) => {
                        let mut model_instance_lock = MODEL_INSTANCE.write().await;
                        *model_instance_lock = Some(Arc::new(model_instance));
                        break;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to initialize model instance: {}. Retrying in {}s...",
                            e,
                            backoff_secs
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                        backoff_secs = backoff_secs
                            .saturating_mul(2)
                            .min(*HUB_EMBEDDINGS_PULLING_INTERVAL_SECS);
                    }
                }
            }
            let mut backoff_secs = *HUB_EMBEDDINGS_RETRY_INTERVAL_SECS;
            loop {
                let sleep_secs = if update_embeddings_db(&db_clone).await {
                    backoff_secs = *HUB_EMBEDDINGS_RETRY_INTERVAL_SECS;
                    *HUB_EMBEDDINGS_PULLING_INTERVAL_SECS
                } else {
                    if let Err(e) = index_resource_types_alone(&db_clone).await {
                        tracing::warn!("Failed to index the resource types alone: {e:#}");
                    }
                    let secs = backoff_secs;
                    backoff_secs = backoff_secs
                        .saturating_mul(2)
                        .min(*HUB_EMBEDDINGS_PULLING_INTERVAL_SECS);
                    secs
                };
                tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
            }
        });
    }
}

#[cfg(feature = "embedding")]
pub async fn update_embeddings_db(db: &Pool<Postgres>) -> bool {
    if let Some(model_instance) = MODEL_INSTANCE.read().await.as_ref() {
        tracing::info!("Creating embeddings DB...");
        let new_embeddings_db = EmbeddingsDb::new(&db, model_instance.clone()).await;
        if let Err(e) = new_embeddings_db.as_ref() {
            tracing::error!("Failed to create embeddings db: {}", e);
            false
        } else {
            let mut embeddings_db = EMBEDDINGS_DB.write().await;
            *embeddings_db = new_embeddings_db.ok();
            tracing::info!("Created embeddings DB");
            true
        }
    } else {
        tracing::error!("Could not update embeddings DB, model instance not initialized");
        false
    }
}

/// After a failed full fill, so a fill still downloading at startup is not raced. A full fill
/// that succeeds later replaces this index.
#[cfg(feature = "embedding")]
async fn index_resource_types_alone(pg_db: &Pool<Postgres>) -> Result<()> {
    if EMBEDDINGS_DB.read().await.is_some() {
        return Ok(());
    }
    let Some(model_instance) = MODEL_INSTANCE.read().await.clone() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let embeddings_db = EmbeddingsDb::resource_types_only(pg_db, model_instance).await?;
    let mut current = EMBEDDINGS_DB.write().await;
    if current.is_none() {
        *current = Some(embeddings_db);
        tracing::info!(
            "Indexed the resource types alone in {:?}, without the hub's embeddings",
            started.elapsed()
        );
    }
    Ok(())
}

pub fn workspaced_service() -> Router {
    Router::new().route("/query_resource_types", get(query_resource_types))
}

#[cfg(feature = "embedding")]
pub fn global_service() -> Router {
    Router::new().route("/query_hub_scripts", get(query_hub_scripts))
}

#[cfg(not(feature = "embedding"))]
pub fn global_service() -> Router {
    Router::new()
}

#[cfg(all(test, feature = "embedding"))]
mod tests {
    use super::trim_to_top_score;

    // The blob carries an explicit `"description": null` for roughly a fifth of hub
    // scripts, and an older hub omits the key entirely. A bare String here fails the
    // whole 155 MB array and takes hub search down with it.
    #[test]
    fn reads_a_hub_script_whether_or_not_it_has_a_description() {
        let present = r#"{"ask_id":1,"id":2,"version_id":3,"summary":"s","description":"d","app":"a","kind":"script","embedding":[]}"#;
        let null = r#"{"ask_id":1,"id":2,"version_id":3,"summary":"s","description":null,"app":"a","kind":"script","embedding":[]}"#;
        let missing = r#"{"ask_id":1,"id":2,"version_id":3,"summary":"s","app":"a","kind":"script","embedding":[]}"#;

        assert_eq!(
            serde_json::from_str::<super::HubScript>(present)
                .unwrap()
                .description,
            Some("d".to_string())
        );
        for without in [null, missing] {
            assert_eq!(
                serde_json::from_str::<super::HubScript>(without)
                    .unwrap()
                    .description,
                None
            );
        }
    }

    #[test]
    fn trims_scores_more_than_5pct_below_top() {
        // top=1.0, cutoff at 0.95: 0.96 stays (0.04 drop), 0.93 is the first
        // beyond the cutoff so take_while stops there and drops the tail.
        let kept = trim_to_top_score(vec![1.0f32, 0.97, 0.96, 0.93, 0.9], 0.05, |s| *s);
        assert_eq!(kept, vec![1.0, 0.97, 0.96]);
    }

    #[test]
    fn keeps_all_when_tightly_clustered() {
        let kept = trim_to_top_score(vec![0.9f32, 0.89, 0.88], 0.05, |s| *s);
        assert_eq!(kept, vec![0.9, 0.89, 0.88]);
    }

    #[test]
    fn passes_through_zero_or_one_result() {
        assert_eq!(
            trim_to_top_score(Vec::<f32>::new(), 0.05, |s| *s),
            Vec::<f32>::new()
        );
        assert_eq!(trim_to_top_score(vec![0.42f32], 0.05, |s| *s), vec![0.42]);
    }
}

#[cfg(test)]
mod text_match_tests {
    use super::text_match_score;

    // A query names the service, which a type's name only contains ("postgres" in
    // `postgresql`): an exact comparison finds nothing for "postgres database".
    #[test]
    fn ranks_a_name_match_above_a_description_match() {
        let words = vec!["postgres".to_string(), "database".to_string()];
        assert_eq!(
            text_match_score(&words, "PostgreSQL", "PostgreSQL connection"),
            2.0
        );
        assert_eq!(
            text_match_score(&words, "mysql", "MySQL database credentials"),
            1.0
        );
        assert_eq!(text_match_score(&words, "slack", "Slack bot token"), 0.0);
    }

    // Many services go by a short name; matching it inside other names would rank `ms365`
    // with `s3`.
    #[test]
    fn matches_a_short_word_only_as_a_whole_name_part() {
        let words = vec!["s3".to_string()];
        assert_eq!(text_match_score(&words, "s3", "Amazon S3 bucket"), 2.0);
        assert_eq!(text_match_score(&words, "aws_s3", ""), 2.0);
        assert_eq!(text_match_score(&words, "ms365", "mentions s3"), 0.0);
    }
}
