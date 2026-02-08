/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Path, Query},
    routing::get,
    Router,
};
use windmill_api_auth::ApiAuthed;
use windmill_common::{
    db::UserDB,
    error::JsonResult,
    utils::{paginate, Pagination},
};

use serde::Serialize;
use sqlx::FromRow;

pub fn workspaced_service() -> Router {
    Router::new().route("/get/:name", get(get_folder_permission_history))
}

#[derive(Serialize, FromRow)]
pub struct FolderPermissionChange {
    pub id: i64,
    pub changed_by: String,
    pub changed_at: chrono::DateTime<chrono::Utc>,
    pub change_type: String,
    pub affected: Option<String>,
}

async fn get_folder_permission_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<FolderPermissionChange>> {
    // Check if user is owner of the folder (before starting transaction for performance)
    crate::folders::require_is_owner(&authed, &name)?;

    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(pagination);

    let history = sqlx::query_as!(
        FolderPermissionChange,
        "SELECT id, changed_by, changed_at, change_type, affected
         FROM folder_permission_history
         WHERE workspace_id = $1 AND folder_name = $2
         ORDER BY id DESC
         LIMIT $3 OFFSET $4",
        w_id,
        name,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(axum::Json(history))
}
