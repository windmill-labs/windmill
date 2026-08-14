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
use serde::{Deserialize, Serialize};
use windmill_api_auth::ApiAuthed;
use windmill_common::{
    db::UserDB,
    error::JsonResult,
    utils::{paginate, Pagination},
};

pub fn workspaced_service() -> Router {
    Router::new().route("/list", get(list_trigger_history))
}

#[derive(Serialize)]
pub struct TriggerHistoryEntry {
    pub id: i64,
    pub trigger_kind: String,
    pub path: String,
    pub operation: String,
    pub source: String,
    pub username: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub changes: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct ListTriggerHistoryQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    /// `"schedule"` or a trigger type (`"http"`, `"kafka"`, …).
    pub trigger_kind: Option<String>,
    pub path: Option<String>,
}

/// Visibility is the RLS policies on `trigger_history`: the path half of what
/// gates the live trigger, so a row that quotes a schedule's `args` never
/// reaches someone who could not read the schedule itself.
async fn list_trigger_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListTriggerHistoryQuery>,
) -> JsonResult<Vec<TriggerHistoryEntry>> {
    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(Pagination { page: query.page, per_page: query.per_page });

    let history = sqlx::query_as!(
        TriggerHistoryEntry,
        "SELECT id, trigger_kind, path, operation, source, username, created_at, changes
         FROM trigger_history
         WHERE workspace_id = $1
           AND ($2::TEXT IS NULL OR trigger_kind = $2)
           AND ($3::TEXT IS NULL OR path = $3)
         ORDER BY id DESC
         LIMIT $4 OFFSET $5",
        w_id,
        query.trigger_kind,
        query.path,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(axum::Json(history))
}
