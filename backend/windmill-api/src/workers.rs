/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Extension, Query},
    routing::get,
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_common::{
    db::UserDB,
    error::JsonResult,
    utils::{paginate, Pagination},
    worker::{ALL_TAGS, DEFAULT_TAGS, DEFAULT_TAGS_PER_WORKSPACE},
};

use crate::db::ApiAuthed;

pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_worker_pings))
        .route("/exists_worker_with_tag", get(exists_worker_with_tag))
        .route("/custom_tags", get(get_custom_tags))
        .route(
            "/is_default_tags_per_workspace",
            get(get_default_tags_per_workspace),
        )
        .route("/get_default_tags", get(get_default_tags))
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
    wm_version: String,
}

#[derive(Serialize, Deserialize)]
struct EnableWorkerQuery {
    disable: bool,
}

#[derive(Deserialize)]
pub struct ListWorkerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub ping_since: Option<i32>,
}

async fn list_worker_pings(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Query(query): Query<ListWorkerQuery>,
) -> JsonResult<Vec<WorkerPing>> {
    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(Pagination { page: query.page, per_page: query.per_page });

    let rows = sqlx::query_as!(
        WorkerPing,
        "SELECT worker, worker_instance,  EXTRACT(EPOCH FROM (now() - ping_at))::integer as last_ping, started_at, ip, jobs_executed, custom_tags, worker_group, wm_version FROM worker_ping
         WHERE ($1::integer IS NULL AND ping_at > now() - interval '5 minute') OR (ping_at > now() - ($1 || ' seconds')::interval)
         ORDER BY ping_at desc LIMIT $2 OFFSET $3",
        query.ping_since,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Serialize, Deserialize)]
struct TagQuery {
    tag: String,
}

async fn exists_worker_with_tag(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Query(tag_query): Query<TagQuery>,
) -> JsonResult<bool> {
    let mut tx = user_db.begin(&authed).await?;
    let row = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM worker_ping WHERE custom_tags @> $1 AND ping_at > now() - interval '1 minute')",
        &[tag_query.tag]
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(row.exists.unwrap_or(false)))
}

async fn get_custom_tags() -> Json<Vec<String>> {
    Json(ALL_TAGS.read().await.clone().into())
}

async fn get_default_tags_per_workspace() -> JsonResult<bool> {
    Ok(Json(
        DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed),
    ))
}

async fn get_default_tags() -> JsonResult<Vec<String>> {
    Ok(Json(DEFAULT_TAGS.clone()))
}
