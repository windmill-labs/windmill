/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::ApiAuthed;
use axum::{
    extract::{Extension, Path, Query},
    routing::get,
    Router,
};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult},
    utils::{paginate, Pagination},
};

use serde::Serialize;
use sqlx::FromRow;

pub fn workspaced_service() -> Router {
    Router::new().route("/get/:name", get(get_group_permission_history))
}

#[derive(Serialize, FromRow)]
pub struct GroupPermissionChange {
    pub id: i64,
    pub changed_by: String,
    pub changed_at: chrono::DateTime<chrono::Utc>,
    pub change_type: String,
    pub member_affected: Option<String>,
}

async fn get_group_permission_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, name)): Path<(String, String)>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<GroupPermissionChange>> {
    // Only workspace admins can view group permission history
    if !authed.is_admin {
        return Err(Error::NotAuthorized(
            "Only workspace administrators can view group permission history".to_string(),
        ));
    }

    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(pagination);

    let history = sqlx::query_as!(
        GroupPermissionChange,
        "SELECT id, changed_by, changed_at, change_type, member_affected
         FROM group_permission_history
         WHERE workspace_id = $1 AND group_name = $2
         ORDER BY changed_at DESC
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
