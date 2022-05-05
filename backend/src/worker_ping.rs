/*
* Author & Copyright: Ruben Fiszel 2021
 * This file and its contents are licensed under the AGPL License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{db::UserDB, error::JsonResult, users::Authed, utils::Pagination};
use axum::{
    extract::{Extension, Query},
    routing::get,
    Json, Router,
};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

pub fn global_service() -> Router {
    Router::new().route("/list", get(list_worker_pings))
}

#[derive(FromRow, Serialize, Deserialize)]
struct WorkerPing {
    worker: String,
    worker_instance: String,
    ping_at: chrono::DateTime<chrono::Utc>,
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

    let (per_page, offset) = crate::utils::paginate(pagination);

    let rows = sqlx::query_as!(
        WorkerPing,
        "SELECT * FROM worker_ping ORDER BY ping_at desc LIMIT $1 OFFSET $2",
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}
