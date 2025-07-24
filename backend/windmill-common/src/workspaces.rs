use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};
use strum::AsRefStr;

use crate::s3_helpers::DuckdbConnectionSettingsResponse;

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

#[derive(Deserialize, Serialize, Debug)]
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
