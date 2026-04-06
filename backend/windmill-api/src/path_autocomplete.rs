/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::{
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};

use axum::{
    extract::{Extension, Path},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use windmill_common::error::JsonResult;

use crate::db::{ApiAuthed, DB};

// Per-table row cap is inlined into the SQL as `LIMIT 5000`.
// With 6 tables, the absolute ceiling is ~30k paths pre-dedup.
/// Final cap applied after dedup/sort.
const MAX_PATHS: usize = 20_000;
/// TTL for the per-workspace path list cache.
const CACHE_TTL: Duration = Duration::from_secs(60);

/// Workspace-wide path list cache keyed by workspace_id only.
/// One entry per workspace shared across all users — autocomplete is a
/// navigation hint, not an access gate. Saves memory and warms faster.
static PATHS_CACHE: LazyLock<quick_cache::sync::Cache<String, (Arc<Vec<String>>, Instant)>> =
    LazyLock::new(|| quick_cache::sync::Cache::new(500));

pub fn workspaced_service() -> Router {
    Router::new().route("/list_paths", get(list_paths))
}

#[derive(Serialize)]
struct ListPathsResponse {
    paths: Arc<Vec<String>>,
}

async fn list_paths(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<ListPathsResponse> {
    if let Some((cached, cached_at)) = PATHS_CACHE.get(&w_id) {
        if cached_at.elapsed() < CACHE_TTL {
            return Ok(Json(ListPathsResponse { paths: cached }));
        }
        PATHS_CACHE.remove(&w_id);
    }

    let mut paths: Vec<String> = sqlx::query_scalar!(
        r#"
        SELECT path AS "path!" FROM (
            (SELECT path FROM script   WHERE workspace_id = $1 LIMIT 5000)
            UNION
            (SELECT path FROM flow     WHERE workspace_id = $1 LIMIT 5000)
            UNION
            (SELECT path FROM app      WHERE workspace_id = $1 LIMIT 5000)
            UNION
            (SELECT path FROM raw_app  WHERE workspace_id = $1 LIMIT 5000)
            UNION
            (SELECT path FROM variable WHERE workspace_id = $1 LIMIT 5000)
            UNION
            (SELECT path FROM resource WHERE workspace_id = $1 LIMIT 5000)
        ) t
        "#,
        &w_id,
    )
    .fetch_all(&db)
    .await?;

    paths.sort_unstable();
    paths.truncate(MAX_PATHS);
    let paths = Arc::new(paths);

    PATHS_CACHE.insert(w_id, (paths.clone(), Instant::now()));

    Ok(Json(ListPathsResponse { paths }))
}
