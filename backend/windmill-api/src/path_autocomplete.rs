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
    extract::{Extension, Path, Query},
    routing::get,
    Json, Router,
};
use quick_cache::{sync::Cache, Weighter};
use serde::{Deserialize, Serialize};
use windmill_common::{db::UserDB, error::JsonResult};
use windmill_store::var_resource_cache::auth_identity;

use crate::db::ApiAuthed;

// Per-table row cap is inlined into the SQL as `LIMIT 5000`.
// With 6 tables, the absolute ceiling is ~30k paths pre-dedup.
/// Final cap applied after dedup/sort.
const MAX_PATHS: usize = 20_000;
/// TTL for the path list cache.
const CACHE_TTL: Duration = Duration::from_secs(60);

/// Total paths held across all cache entries.
const CACHE_MAX_PATHS: u64 = 2_000_000;

type CacheKey = (String, String);
type CacheValue = (Arc<Vec<String>>, Instant);

#[derive(Clone)]
struct PathCountWeighter;

impl Weighter<CacheKey, CacheValue> for PathCountWeighter {
    fn weight(&self, _key: &CacheKey, (paths, _): &CacheValue) -> u64 {
        (paths.len() as u64).max(1)
    }
}

/// Keyed by (workspace_id, [`auth_identity`]): the list is read under the caller's RLS,
/// so an entry must only ever be served back to the same authorization context.
/// Weighted by path count because the key space grows with callers, not workspaces.
/// Single-sharded: quick_cache splits the weight budget across shards and refuses an
/// entry heavier than one shard's share, which would drop the largest workspaces.
static PATHS_CACHE: LazyLock<Cache<CacheKey, CacheValue, PathCountWeighter>> =
    LazyLock::new(|| {
        let options = quick_cache::OptionsBuilder::new()
            .shards(1)
            .estimated_items_capacity(1000)
            .weight_capacity(CACHE_MAX_PATHS)
            .build()
            .expect("every cache option is set");
        Cache::with_options(
            options,
            PathCountWeighter,
            Default::default(),
            Default::default(),
        )
    });

pub fn workspaced_service() -> Router {
    Router::new().route("/list_paths", get(list_paths))
}

#[derive(Serialize)]
struct ListPathsResponse {
    paths: Arc<Vec<String>>,
}

#[derive(Deserialize)]
struct ListPathsQuery {
    /// When true, bypass the cached entry and re-query the DB, refreshing the
    /// cache. Used by clients that just mutated the workspace (e.g. a deploy)
    /// and need the new path reflected immediately.
    #[serde(default)]
    force: bool,
}

async fn list_paths(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(ListPathsQuery { force }): Query<ListPathsQuery>,
) -> JsonResult<ListPathsResponse> {
    let cache_key = (w_id, auth_identity(&authed));
    if !force {
        if let Some((cached, cached_at)) = PATHS_CACHE.get(&cache_key) {
            if cached_at.elapsed() < CACHE_TTL {
                return Ok(Json(ListPathsResponse { paths: cached }));
            }
            PATHS_CACHE.remove(&cache_key);
        }
    }

    let mut tx = user_db.begin(&authed).await?;
    let mut paths: Vec<String> = sqlx::query_scalar!(
        r#"
        SELECT path AS "path!" FROM (
            (SELECT DISTINCT path FROM script WHERE workspace_id = $1 AND archived = false AND deleted = false LIMIT 5000)
            UNION
            (SELECT path FROM flow     WHERE workspace_id = $1 AND archived = false LIMIT 5000)
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
        &cache_key.0,
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    paths.sort_unstable();
    paths.truncate(MAX_PATHS);
    let paths = Arc::new(paths);

    PATHS_CACHE.insert(cache_key, (paths.clone(), Instant::now()));

    Ok(Json(ListPathsResponse { paths }))
}
