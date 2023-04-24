/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{db::UserDB, users::Authed};
use axum::{
    extract::{Extension, Query},
    routing::get,
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_common::{
    error::JsonResult,
    utils::{paginate, Pagination},
};

pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_worker_pings))
        .route("/custom_tags", get(get_custom_tags))
}

lazy_static::lazy_static! {
    pub static ref CUSTOM_TAGS: Vec<String> = std::env::var("CUSTOM_TAGS")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<_>>()).unwrap_or_default();

}

#[derive(FromRow, Serialize, Deserialize)]
struct WorkerPing {
    worker: String,
    worker_instance: String,
    last_ping: Option<i32>,
    started_at: chrono::DateTime<chrono::Utc>,
    ip: String,
    jobs_executed: i32,
}

async fn list_worker_pings(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<WorkerPing>> {
    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(pagination);

    let rows = sqlx::query_as!(
        WorkerPing,
        "SELECT worker, worker_instance,  EXTRACT(EPOCH FROM (now() - ping_at))::integer as last_ping, started_at, ip, jobs_executed FROM worker_ping ORDER BY ping_at desc LIMIT $1 OFFSET $2",
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn get_custom_tags() -> Json<Vec<String>> {
    Json(CUSTOM_TAGS.clone())
}
