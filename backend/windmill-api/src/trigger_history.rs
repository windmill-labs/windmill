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
use windmill_api_auth::{build_scope_path_filter, check_scopes, ApiAuthed, ScopePathFilter};
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

/// Two gates, because they answer different questions: the RLS policies on
/// `trigger_history` bound the rows to what the *user* may read, and
/// `triggers_history:read:<path>` bounds them further to what this *token* may
/// read. Without the second, a token scoped to one path could read the diffs of
/// every trigger its user can see, and a `create` row quotes the whole trigger
/// row, a schedule's `args` included.
async fn list_trigger_history(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListTriggerHistoryQuery>,
) -> JsonResult<Vec<TriggerHistoryEntry>> {
    if let Some(path) = query.path.as_deref() {
        check_scopes(&authed, || format!("triggers_history:read:{}", path))?;
    }

    // In the WHERE, not a retain after the fetch: the result is paginated, and a
    // post-fetch filter would let a page's size report how many rows the token
    // may not read — and return short pages that read as "no history".
    let (scope_all, scope_exact, scope_prefix) =
        match build_scope_path_filter(&authed, "triggers_history", "read") {
            ScopePathFilter::AllowAll => (true, Vec::new(), Vec::new()),
            ScopePathFilter::Restricted { exact, prefix } => (false, exact, prefix),
        };

    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(Pagination { page: query.page, per_page: query.per_page });

    let history = sqlx::query_as!(
        TriggerHistoryEntry,
        "SELECT id, trigger_kind, path, operation, source, username, created_at, changes
         FROM trigger_history
         WHERE workspace_id = $1
           AND ($2::TEXT IS NULL OR trigger_kind = $2)
           AND ($3::TEXT IS NULL OR path = $3)
           AND ( $6
                 OR path = ANY($7)
                 OR EXISTS ( SELECT 1 FROM unnest($8::text[]) AS pfx
                             WHERE path = pfx
                                OR left(path, length(pfx) + 1) = pfx || '/' ) )
         ORDER BY id DESC
         LIMIT $4 OFFSET $5",
        w_id,
        query.trigger_kind,
        query.path,
        per_page as i64,
        offset as i64,
        scope_all,
        &scope_exact[..],
        &scope_prefix[..],
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(axum::Json(history))
}
