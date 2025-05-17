use quick_cache::sync::Cache;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceGitSyncSettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
    pub repositories: Vec<GitRepositorySettings>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceDeploymentUISettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
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
    HttpTrigger,
    WebsocketTrigger,
    KafkaTrigger,
    NatsTrigger,
    PostgresTrigger,
    MqttTrigger,
    SqsTrigger,
    GcpTrigger,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitRepositorySettings {
    pub script_path: String,
    pub git_repo_resource_path: String,
    pub use_individual_branch: Option<bool>,
    pub group_by_folder: Option<bool>,
    pub exclude_types_override: Option<Vec<ObjectType>>,
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
