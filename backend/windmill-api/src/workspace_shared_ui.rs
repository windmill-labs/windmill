/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{Extension, Json, Path},
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    utils::require_admin,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/get", get(get_shared_ui))
        .route("/list", get(list_shared_ui))
        .route("/version", get(get_version))
        .route("/", put(update_shared_ui))
}

#[derive(Serialize, Deserialize)]
pub struct SharedUi {
    pub files: HashMap<String, String>,
    pub version: i64,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub edited_by: String,
}

#[derive(Serialize)]
pub struct SharedUiListing {
    pub paths: Vec<String>,
    pub sizes: HashMap<String, i64>,
    pub version: i64,
    pub edited_at: chrono::DateTime<chrono::Utc>,
    pub edited_by: String,
}

#[derive(Serialize)]
pub struct SharedUiVersion {
    pub version: i64,
}

#[derive(Deserialize)]
pub struct UpdateSharedUi {
    pub files: HashMap<String, String>,
}

async fn get_shared_ui(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<SharedUi> {
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT files, version, edited_at, edited_by FROM workspace_shared_ui WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let result = match row {
        Some(r) => SharedUi {
            files: serde_json::from_value(r.files).unwrap_or_default(),
            version: r.version,
            edited_at: r.edited_at,
            edited_by: r.edited_by,
        },
        None => SharedUi {
            files: HashMap::new(),
            version: 0,
            edited_at: chrono::Utc::now(),
            edited_by: String::new(),
        },
    };
    Ok(Json(result))
}

async fn list_shared_ui(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<SharedUiListing> {
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT files, version, edited_at, edited_by FROM workspace_shared_ui WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    let (files, version, edited_at, edited_by) = match row {
        Some(r) => {
            let files: HashMap<String, String> =
                serde_json::from_value(r.files).unwrap_or_default();
            (files, r.version, r.edited_at, r.edited_by)
        }
        None => (HashMap::new(), 0, chrono::Utc::now(), String::new()),
    };

    let mut paths: Vec<String> = files.keys().cloned().collect();
    paths.sort();
    let sizes: HashMap<String, i64> = files
        .iter()
        .map(|(k, v)| (k.clone(), v.len() as i64))
        .collect();

    Ok(Json(SharedUiListing {
        paths,
        sizes,
        version,
        edited_at,
        edited_by,
    }))
}

async fn get_version(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
) -> JsonResult<SharedUiVersion> {
    let mut tx = user_db.begin(&authed).await?;
    let version = sqlx::query_scalar!(
        "SELECT version FROM workspace_shared_ui WHERE workspace_id = $1",
        &w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(SharedUiVersion { version: version.unwrap_or(0) }))
}

async fn update_shared_ui(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(payload): Json<UpdateSharedUi>,
) -> Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let files_json = serde_json::to_value(&payload.files)
        .map_err(|e| Error::internal_err(format!("serializing files: {e}")))?;

    let mut tx = db.begin().await?;
    sqlx::query!(
        r#"INSERT INTO workspace_shared_ui (workspace_id, files, version, edited_at, edited_by)
           VALUES ($1, $2, 1, now(), $3)
           ON CONFLICT (workspace_id) DO UPDATE
           SET files = EXCLUDED.files,
               version = workspace_shared_ui.version + 1,
               edited_at = now(),
               edited_by = EXCLUDED.edited_by"#,
        &w_id,
        files_json,
        &authed.username,
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed,
        "workspace_shared_ui.update",
        ActionKind::Update,
        &w_id,
        Some(&authed.email),
        Some([("file_count", &payload.files.len().to_string()[..])].into()),
    )
    .await?;
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Settings { setting_type: "shared_ui".to_string() },
        None,
        false,
        None,
    )
    .await?;

    Ok(format!("Updated shared UI for workspace {}", &w_id))
}
