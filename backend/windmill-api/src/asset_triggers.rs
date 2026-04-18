//! Read-only HTTP surface for `asset_trigger` rows (stage 3).
//!
//! v1 of asset-change triggers exposes only implicit triggers (projected
//! from `#trigger: asset` annotations in script source). Mutations go
//! through the script save path, so every write endpoint here returns
//! **409 Conflict** with a pointer back to the owning script.

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlxJson;

use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::StripPath,
};

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_asset_triggers))
        .route("/get/{*path}", get(get_asset_trigger))
        .route("/create", post(reject_mutate))
        .route("/update/{*path}", post(reject_mutate_path))
        .route("/delete/{*path}", delete(reject_mutate_path))
}

#[derive(Serialize, Deserialize, Debug, sqlx::FromRow)]
pub struct AssetTriggerRow {
    pub workspace_id: String,
    pub path: String,
    pub script_path: String,
    pub is_flow: bool,
    pub owner_script_path: Option<String>,
    pub owner_script_hash: Option<i64>,
    pub is_implicit: bool,
    pub on_event: String,
    pub subscription_set: SqlxJson<serde_json::Value>,
    pub fires: String,
    pub debounce_s: i32,
    pub partition_map: Option<SqlxJson<serde_json::Value>>,
    pub cancel_on_new: bool,
    pub backlog: String,
    pub error: Option<String>,
    pub edited_by: String,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub email: String,
}

async fn list_asset_triggers(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<AssetTriggerRow>> {
    let mut tx = user_db.begin(&authed).await?;
    let rows = sqlx::query_as::<_, AssetTriggerRow>(
        "SELECT workspace_id, path, script_path, is_flow, owner_script_path, \
                owner_script_hash, is_implicit, on_event, subscription_set, fires, \
                debounce_s, partition_map, cancel_on_new, backlog, error, edited_by, \
                edited_at, email \
         FROM asset_trigger \
         WHERE workspace_id = $1 \
         ORDER BY edited_at DESC",
    )
    .bind(&w_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn get_asset_trigger(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<AssetTriggerRow> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query_as::<_, AssetTriggerRow>(
        "SELECT workspace_id, path, script_path, is_flow, owner_script_path, \
                owner_script_hash, is_implicit, on_event, subscription_set, fires, \
                debounce_s, partition_map, cancel_on_new, backlog, error, edited_by, \
                edited_at, email \
         FROM asset_trigger \
         WHERE workspace_id = $1 AND path = $2",
    )
    .bind(&w_id)
    .bind(path)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| Error::NotFound(format!("asset_trigger {path} not found")))?;
    tx.commit().await?;
    Ok(Json(row))
}

#[derive(Deserialize)]
struct RejectBody {
    #[allow(dead_code)]
    path: Option<String>,
}

async fn reject_mutate(
    _authed: ApiAuthed,
    Extension(_user_db): Extension<UserDB>,
    Path(_w_id): Path<String>,
    Json(_body): Json<RejectBody>,
) -> Result<()> {
    Err(implicit_mutation_error(None))
}

async fn reject_mutate_path(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<()> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;
    let owner = sqlx::query_scalar!(
        "SELECT owner_script_path FROM asset_trigger \
         WHERE workspace_id = $1 AND path = $2 AND is_implicit = true",
        &w_id,
        path,
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();
    tx.commit().await?;
    Err(implicit_mutation_error(owner))
}

fn implicit_mutation_error(owner: Option<String>) -> Error {
    let msg = match owner {
        Some(script_path) => format!(
            "Implicit asset trigger — edit `#trigger: asset` annotations in script {script_path}."
        ),
        None => {
            "Asset triggers are implicit-only in v1. Edit `#trigger: asset` annotations in the \
             owning script to modify them."
                .to_string()
        }
    };
    Error::Generic(StatusCode::CONFLICT, msg)
}
