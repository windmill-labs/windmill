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
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_common::{
    db::UserDB,
    error::{self, JsonResult},
    utils::{paginate, Pagination},
    worker::ALL_TAGS,
    DB,
};

#[cfg(feature = "benchmark")]
use std::sync::atomic::Ordering;
#[cfg(feature = "benchmark")]
use windmill_queue::IDLE_WORKERS;

use crate::{db::ApiAuthed, utils::require_super_admin};

pub fn global_service() -> Router {
    use axum::routing::post;

    let router = Router::new()
        .route("/list", get(list_worker_pings))
        .route("/custom_tags", get(get_custom_tags))
        .route("/list_worker_groups", get(get_worker_groups))
        .route(
            "/worker_group/:name",
            post(update_worker_group).delete(delete_worker_group),
        );
    #[cfg(feature = "benchmark")]
    return router.route("/toggle", get(toggle));

    #[cfg(not(feature = "benchmark"))]
    return router;
}

#[derive(FromRow, Serialize, Deserialize)]
struct WorkerPing {
    worker: String,
    worker_instance: String,
    last_ping: Option<i32>,
    started_at: chrono::DateTime<chrono::Utc>,
    ip: String,
    jobs_executed: i32,
    custom_tags: Option<Vec<String>>,
    worker_group: String,
}

#[derive(Serialize, Deserialize)]
struct EnableWorkerQuery {
    disable: bool,
}

async fn list_worker_pings(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<WorkerPing>> {
    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(pagination);

    let rows = sqlx::query_as!(
        WorkerPing,
        "SELECT worker, worker_instance,  EXTRACT(EPOCH FROM (now() - ping_at))::integer as last_ping, started_at, ip, jobs_executed, custom_tags, worker_group FROM worker_ping ORDER BY ping_at desc LIMIT $1 OFFSET $2",
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

#[cfg(feature = "benchmark")]
async fn toggle(Query(query): Query<EnableWorkerQuery>) -> JsonResult<bool> {
    IDLE_WORKERS.store(query.disable, Ordering::Relaxed);
    Ok(Json(IDLE_WORKERS.load(Ordering::Relaxed)))
}

async fn get_custom_tags() -> Json<Vec<String>> {
    Json(ALL_TAGS.read().await.clone().into())
}

#[derive(Serialize, Deserialize, FromRow)]
struct WorkerGroup {
    name: String,
    config: serde_json::Value,
}

async fn get_worker_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> error::JsonResult<Vec<WorkerGroup>> {
    require_super_admin(&db, &authed.email).await?;

    let rows = sqlx::query_as!(WorkerGroup, "SELECT * FROM worker_group_config")
        .fetch_all(&db)
        .await?;
    Ok(Json(rows))
}

#[cfg(feature = "enterprise")]
async fn update_worker_group(
    Path(name): Path<String>,
    Extension(db): Extension<DB>,
    authed: ApiAuthed,
    Json(config): Json<serde_json::Value>,
) -> error::Result<String> {
    require_super_admin(&db, &authed.email).await?;

    sqlx::query!(
        "INSERT INTO worker_group_config (name, config) VALUES ($1, $2) ON CONFLICT (name) DO UPDATE SET config = $2",
        &name,
        config
    )
    .execute(&db)
    .await?;

    Ok(format!("Updated worker group {name}"))
}

#[cfg(not(feature = "enterprise"))]
async fn update_worker_group() -> String {
    "Worker groups available only in enterprise version".to_string()
}

async fn delete_worker_group(
    Path(name): Path<String>,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    authed: ApiAuthed,
) -> error::Result<String> {
    let tx = user_db.begin(&authed).await?;

    require_super_admin(&db, &authed.email).await?;
    tx.commit().await?;

    let deleted = sqlx::query!(
        "DELETE FROM worker_group_config WHERE name = $1 RETURNING name",
        name,
    )
    .fetch_all(&db)
    .await?;

    if deleted.len() == 0 {
        return Err(error::Error::NotFound(format!(
            "Worker group {name} not found",
            name = name
        )));
    }
    Ok(format!("Deleted worker group {name}"))
}
