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

use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use windmill_common::{
    db::UserDB,
    error::JsonResult,
    utils::{paginate, Pagination},
};

use std::collections::HashMap;
#[cfg(feature = "benchmark")]
use std::sync::atomic::Ordering;
#[cfg(feature = "benchmark")]
use windmill_queue::IDLE_WORKERS;

use crate::{db::ApiAuthed, utils::require_super_admin};

#[cfg(not(feature = "benchmark"))]
pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_worker_pings))
        .route("/custom_tags", get(get_custom_tags))
}

#[cfg(feature = "benchmark")]
pub fn global_service() -> Router {
    Router::new()
        .route("/toggle", get(toggle))
        .route("/list", get(list_worker_pings))
        .route("/custom_tags", get(get_custom_tags))
        .route("/list_worker_groups", get(get_worker_groups))
        .route("/updated_worker_groups/:name", post(update_worker_group))
}

lazy_static::lazy_static! {
    pub static ref CUSTOM_TAGS: Vec<String> = std::env::var("CUSTOM_TAGS")
        .ok()
        .map(|x| x.split(',').map(|x| x.to_string()).collect::<Vec<_>>()).unwrap_or_default();

    pub static ref CUSTOM_TAGS_PER_WORKSPACE: (Vec<String>, HashMap<String, Vec<String>>) =  process_custom_tags(std::env::var("CUSTOM_TAGS")
        .ok());

    pub static ref ALL_TAGS: Vec<String> = [CUSTOM_TAGS_PER_WORKSPACE.0.clone(), CUSTOM_TAGS_PER_WORKSPACE.1.keys().map(|x| x.to_string()).collect_vec()].concat();

}

fn process_custom_tags(o: Option<String>) -> (Vec<String>, HashMap<String, Vec<String>>) {
    let regex = Regex::new(r"^(\w+)\(((?:\w+)\+?)+\)$").unwrap();
    if let Some(s) = o {
        let mut global = vec![];
        let mut specific: HashMap<String, Vec<String>> = HashMap::new();
        for e in s.split(",") {
            if let Some(cap) = regex.captures(e) {
                let tag = cap.get(1).unwrap().as_str().to_string();
                let workspaces = cap.get(2).unwrap().as_str().split("+");
                specific.insert(tag, workspaces.map(|x| x.to_string()).collect_vec());
            } else {
                global.push(e.to_string());
            }
        }
        (global, specific)
    } else {
        (vec![], HashMap::new())
    }
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
        "SELECT worker, worker_instance,  EXTRACT(EPOCH FROM (now() - ping_at))::integer as last_ping, started_at, ip, jobs_executed, custom_tags FROM worker_ping ORDER BY ping_at desc LIMIT $1 OFFSET $2",
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
    Json(ALL_TAGS.clone())
}

#[derive(Serialize, Deserialize)]
struct WorkerGroup {
    name: String,
    config: serde_json::Value,
}

async fn get_worker_groups(
    Extension(user_db): Extension<UserDB>,
    authed: Authed,
) -> Json<Vec<WorkerGroup>> {
    let mut tx = user_db.begin(&authed).await?;

    require_super_admin(user_db, authed.email)?;

    let rows = sqlx::query_as!(WorkerGroup, "SELECT * FROM worker_group")
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}
