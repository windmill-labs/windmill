use async_recursion::async_recursion;
use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::AsRefStr;

use crate::{
    error::{to_anyhow, Error, Result},
    get_database_url, parse_postgres_url,
    s3_helpers::{
        format_duckdb_connection_settings, lfs_to_object_store_resource,
        DuckdbConnectionSettingsResponse, LargeFileStorage,
    },
    variables::{build_crypt, decrypt},
    DB,
};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceGitSyncSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_type: Option<Vec<ObjectType>>,
    pub repositories: Vec<GitRepositorySettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_include_path: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceDeploymentUISettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ObjectType {
    Script,
    Flow,
    App,
    Folder,
    Resource,
    Variable,
    Secret,
    Schedule,
    ResourceType,
    User,
    Group,
    Trigger,
    Settings,
    Key,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitRepositorySettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_types_override: Option<Vec<ObjectType>>,
    pub script_path: String,
    pub git_repo_resource_path: String,
    pub use_individual_branch: Option<bool>,
    pub group_by_folder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<GitSyncSettings>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitSyncSettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_include_path: Option<Vec<String>>,
}

impl Default for GitSyncSettings {
    fn default() -> Self {
        Self {
            include_path: Vec::new(),
            include_type: Vec::new(),
            exclude_path: None,
            extra_include_path: None,
        }
    }
}

lazy_static::lazy_static! {
    pub static ref IS_PREMIUM_CACHE: Cache<String, bool> = Cache::new(5000);
}

#[cfg(feature = "cloud")]
pub async fn is_premium_workspace(_db: &crate::DB, _w_id: &str) -> bool {
    let cached = IS_PREMIUM_CACHE.get(_w_id);
    if let Some(cached) = cached {
        return cached;
    }
    let premium = sqlx::query_scalar!("SELECT premium FROM workspace WHERE id = $1", _w_id)
        .fetch_one(_db)
        .await
        .unwrap_or(false);
    IS_PREMIUM_CACHE.insert(_w_id.to_string(), premium);
    premium
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Ducklake {
    pub catalog: DucklakeCatalog,
    pub storage: DucklakeStorage,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DucklakeCatalog {
    pub resource_type: DucklakeCatalogResourceType,
    pub resource_path: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DucklakeStorage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    pub path: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum DucklakeCatalogResourceType {
    #[strum(serialize = "postgres")]
    Postgresql,
    Mysql,
    Instance,
}

#[derive(Deserialize, Serialize)]
pub struct DucklakeWithConnData {
    pub catalog: DucklakeCatalog,
    pub catalog_resource: serde_json::Value,
    pub storage: DucklakeStorage,
    pub storage_settings: DuckdbConnectionSettingsResponse,
}

pub async fn get_ducklake_from_db_unchecked(
    name: &str,
    w_id: &str,
    db: &DB,
) -> Result<DucklakeWithConnData> {
    let ducklake = sqlx::query_scalar!(
        r#"
            SELECT ws.ducklake->'ducklakes'->$2 AS config
            FROM workspace_settings ws
            WHERE ws.workspace_id = $1
        "#,
        &w_id,
        name
    )
    .fetch_one(db)
    .await
    .map_err(|err| Error::internal_err(format!("getting ducklake {name}: {err}")))?
    .ok_or_else(|| Error::internal_err(format!("ducklake {name} not found")))?;

    let ducklake = serde_json::from_value::<Ducklake>(ducklake)?;

    let lfs = if let Some(storage) = &ducklake.storage.storage {
        sqlx::query_scalar!("SELECT large_file_storage->'secondary_storage'->$2 FROM workspace_settings WHERE workspace_id = $1", w_id, storage)
    } else {
        sqlx::query_scalar!("SELECT large_file_storage FROM workspace_settings WHERE workspace_id = $1", w_id)
    }.fetch_optional(db)
    .await?
    .flatten()
    .map(serde_json::from_value::<LargeFileStorage>)
    .ok_or_else(|| Error::ExecutionErr("Ducklake storage not found".to_string()))??;

    let s3_resource = transform_json_unchecked(
        &serde_json::Value::String(lfs.get_s3_resource_path().to_string()),
        w_id,
        db,
    )
    .await?;
    let object_store_resource = lfs_to_object_store_resource(&lfs, s3_resource)?;

    let catalog_resource =
        if ducklake.catalog.resource_type == DucklakeCatalogResourceType::Instance {
            let pg_creds = parse_postgres_url(&get_database_url().await?)?;
            json!({
                "dbname": ducklake.catalog.resource_path,
                "host": pg_creds.host,
                "port": pg_creds.port,
                "user": "ducklake_user",
                "sslmode": pg_creds.ssl_mode,
                "password": get_ducklake_instance_pg_catalog_password(&db).await?,
            })
        } else {
            transform_json_unchecked(
                &serde_json::Value::String(format!("$res:{}", ducklake.catalog.resource_path)),
                w_id,
                db,
            )
            .await?
        };
    let ducklake = DucklakeWithConnData {
        catalog_resource,
        storage_settings: format_duckdb_connection_settings(object_store_resource)?,
        catalog: ducklake.catalog,
        storage: ducklake.storage,
    };
    Ok(ducklake)
}

pub async fn get_ducklake_instance_pg_catalog_password(db: &DB) -> Result<String> {
    sqlx::query_scalar!(
        "SELECT trim(both '\"' from value::text) FROM global_settings WHERE name = 'ducklake_user_pg_pwd';"
    )
    .fetch_optional(db)
    .await?
    .flatten().ok_or_else(||
        Error::BadRequest(format!(
            "Ducklake instance catalog password not found, did you run migrations ?"
        ))
    )
}

// This does not check for any permission. Should never be displayed to a user.
#[async_recursion]
async fn transform_json_unchecked(
    value: &serde_json::Value,
    w_id: &str,
    db: &DB,
) -> Result<serde_json::Value> {
    let value = match value {
        serde_json::Value::Object(map) => {
            let mut transformed_map = serde_json::Map::new();
            for (key, val) in map {
                let transformed_val = transform_json_unchecked(val, w_id, db).await?;
                transformed_map.insert(key.clone(), serde_json::to_value(transformed_val)?);
            }
            serde_json::Value::Object(transformed_map)
        }
        serde_json::Value::Array(arr) => {
            let mut transformed_array = Vec::new();
            for val in arr {
                let transformed_val = transform_json_unchecked(val, w_id, db).await?;
                transformed_array.push(serde_json::to_value(transformed_val)?);
            }
            serde_json::Value::Array(transformed_array)
        }
        serde_json::Value::String(s) if s.starts_with("$res:") => {
            let resource = sqlx::query_scalar!(
                "SELECT value AS \"value!: _\" FROM resource WHERE workspace_id = $1 AND path = $2",
                &w_id,
                &s[5..]
            )
            .fetch_one(db)
            .await
            .map_err(to_anyhow)?;
            transform_json_unchecked(&resource, w_id, db).await?
        }
        serde_json::Value::String(s) if s.starts_with("$var:") => {
            let variable = sqlx::query_scalar!(
                "SELECT value FROM variable WHERE workspace_id = $1 AND path = $2",
                &w_id,
                &s[5..]
            )
            .fetch_one(db)
            .await
            .map_err(to_anyhow)?;
            let mc = build_crypt(&db, &w_id).await?;
            let variable = decrypt(&mc, variable)?;
            serde_json::Value::String(variable)
        }
        s @ serde_json::Value::String(_) => s.clone(),
        x => x.clone(),
    };

    Ok(value)
}
