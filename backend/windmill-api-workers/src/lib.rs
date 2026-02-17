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
use uuid::Uuid;
use windmill_common::{
    db::UserDB,
    error::JsonResult,
    jobs::{HIDE_WORKERS_FOR_NON_ADMINS, TAGS_ARE_SENSITIVE},
    utils::{paginate, Pagination},
    worker::{ALL_TAGS, CUSTOM_TAGS_PER_WORKSPACE, DEFAULT_TAGS, DEFAULT_TAGS_PER_WORKSPACE},
    DB,
};

use windmill_api_auth::{require_super_admin, ApiAuthed};

pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_worker_pings))
        .route("/exists_workers_with_tags", get(exists_workers_with_tags))
        .route("/custom_tags", get(get_custom_tags))
        .route(
            "/is_default_tags_per_workspace",
            get(get_default_tags_per_workspace),
        )
        .route("/get_default_tags", get(get_default_tags))
        .route("/queue_metrics", get(get_queue_metrics))
        .route("/queue_counts", get(get_queue_counts))
        .route("/queue_running_counts", get(get_queue_running_counts))
}

pub fn workspaced_service() -> Router {
    Router::new().route("/custom_tags", get(get_custom_tags_for_workspace))
}

#[derive(FromRow, Serialize, Deserialize)]
struct WorkerPing {
    worker: String,
    worker_instance: String,
    last_ping: Option<i32>,
    started_at: chrono::DateTime<chrono::Utc>,
    ip: String,
    jobs_executed: i32,
    last_job_id: Option<Uuid>,
    last_job_workspace_id: Option<String>,
    custom_tags: Option<Vec<String>>,
    worker_group: String,
    wm_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupancy_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupancy_rate_15s: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupancy_rate_5m: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    occupancy_rate_30m: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vcpus: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_usage: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wm_memory_usage: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    job_isolation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_mode: Option<bool>,
}

// #[derive(Serialize, Deserialize)]
// struct EnableWorkerQuery {
//     disable: bool,
// }

#[derive(Deserialize)]
pub struct ListWorkerQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub ping_since: Option<i32>,
}

async fn list_worker_pings(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Query(query): Query<ListWorkerQuery>,
) -> JsonResult<Vec<WorkerPing>> {
    let is_super_admin = require_super_admin(&db, &authed.email).await.is_ok();
    if *HIDE_WORKERS_FOR_NON_ADMINS && !is_super_admin {
        return Ok(Json(vec![]));
    }
    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(Pagination { page: query.page, per_page: query.per_page });

    let rows = sqlx::query_as!(
        WorkerPing,
        "SELECT worker, worker_instance,  EXTRACT(EPOCH FROM (now() - ping_at))::integer as last_ping, started_at, ip, jobs_executed,
        CASE WHEN $4 IS TRUE THEN current_job_id ELSE NULL END as last_job_id, CASE WHEN $4 IS TRUE THEN current_job_workspace_id ELSE NULL END as last_job_workspace_id,
        custom_tags, worker_group, wm_version, occupancy_rate, occupancy_rate_15s, occupancy_rate_5m, occupancy_rate_30m, memory, vcpus, memory_usage, wm_memory_usage, job_isolation, native_mode
        FROM worker_ping
        WHERE ($1::integer IS NULL AND ping_at > now() - interval '5 minute') OR (ping_at > now() - ($1 || ' seconds')::interval)
        ORDER BY ping_at desc LIMIT $2 OFFSET $3",
        query.ping_since,
        per_page as i64,
        offset as i64,
        is_super_admin
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    let rows = if *TAGS_ARE_SENSITIVE && !is_super_admin {
        rows.into_iter()
            .map(|mut w| {
                w.custom_tags = None;
                w
            })
            .collect()
    } else {
        rows
    };

    Ok(Json(rows))
}

#[derive(Serialize, Deserialize)]
struct TagsQuery {
    tags: String,
    workspace: Option<String>,
}

async fn exists_workers_with_tags(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Query(tags_query): Query<TagsQuery>,
) -> JsonResult<std::collections::HashMap<String, bool>> {
    // Create a list of requested tags
    let mut tags: Vec<String> = tags_query.tags.split(',').map(|s| s.to_string()).collect();

    // When TAGS_ARE_SENSITIVE is enabled, filter tags based on workspace visibility
    if *TAGS_ARE_SENSITIVE {
        let is_super_admin = require_super_admin(&db, &authed.email).await.is_ok();
        if !is_super_admin {
            if let Some(ref workspace) = tags_query.workspace {
                // Filter to only tags visible in this workspace
                let custom_tags = CUSTOM_TAGS_PER_WORKSPACE.read().await;
                let allowed_tags = custom_tags.to_string_vec(Some(workspace.clone()));
                tags.retain(|t| allowed_tags.contains(t));
            } else {
                // No workspace provided and not superadmin - return empty
                return Ok(Json(std::collections::HashMap::new()));
            }
        }
    }

    if tags.is_empty() {
        return Ok(Json(std::collections::HashMap::new()));
    }

    let mut tx = user_db.begin(&authed).await?;
    let mut result = std::collections::HashMap::new();

    // Create a query that checks all tags at once using unnest
    let rows = sqlx::query!(
        "SELECT tag::text, EXISTS(SELECT 1 FROM worker_ping WHERE custom_tags @> ARRAY[tag] AND ping_at > now() - interval '1 minute') as exists
         FROM unnest($1::text[]) as tag",
        tags.as_slice()
    )
    .fetch_all(&mut *tx)
    .await?;

    for row in rows {
        result.insert(row.tag.unwrap_or_default(), row.exists.unwrap_or(false));
    }

    tx.commit().await?;
    Ok(Json(result))
}

#[derive(Deserialize)]
struct CustomTagQuery {
    show_workspace_restriction: Option<bool>,
}

async fn get_custom_tags(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Query(query): Query<CustomTagQuery>,
) -> JsonResult<Vec<String>> {
    if query.show_workspace_restriction.is_some_and(|x| x) {
        let tags_o = CUSTOM_TAGS_PER_WORKSPACE.read().await;
        let all_tags = tags_o.to_string_vec(None);
        return Ok(Json(all_tags));
    }
    if *TAGS_ARE_SENSITIVE {
        let is_super_admin = require_super_admin(&db, &authed.email).await.is_ok();
        if !is_super_admin {
            return Ok(Json(vec![]));
        }
    }
    Ok(Json(ALL_TAGS.read().await.clone().into()))
}

async fn get_custom_tags_for_workspace(
    _authed: ApiAuthed,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<String>> {
    let tags_o = CUSTOM_TAGS_PER_WORKSPACE.read().await;
    let all_tags = tags_o.to_string_vec(Some(w_id));
    Ok(Json(all_tags))
}

async fn get_default_tags_per_workspace() -> JsonResult<bool> {
    Ok(Json(
        DEFAULT_TAGS_PER_WORKSPACE.load(std::sync::atomic::Ordering::Relaxed),
    ))
}

async fn get_default_tags() -> JsonResult<Vec<String>> {
    Ok(Json(DEFAULT_TAGS.clone()))
}

#[derive(Serialize)]
struct QueueMetric {
    id: String,
    values: Vec<serde_json::Value>,
}

async fn get_queue_metrics(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<QueueMetric>> {
    require_super_admin(&db, &authed.email).await?;

    let queue_metrics = sqlx::query_as!(
        QueueMetric,
        "WITH queue_metrics as (
            SELECT id, value, created_at
            FROM metrics
            WHERE id LIKE 'queue_%'
                AND created_at > now() - interval '14 day'
        )
        SELECT id, array_agg(json_build_object('value', value, 'created_at', created_at) ORDER BY created_at ASC) as \"values!\"
        FROM queue_metrics
        GROUP BY id
        ORDER BY id ASC"
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(queue_metrics))
}

async fn get_queue_counts(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<std::collections::HashMap<String, u32>> {
    require_super_admin(&db, &authed.email).await?;
    let queue_counts = windmill_common::queue::get_queue_counts(&db).await;
    Ok(Json(queue_counts))
}

async fn get_queue_running_counts(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<std::collections::HashMap<String, u32>> {
    require_super_admin(&db, &authed.email).await?;
    let queue_running_counts = windmill_common::queue::get_queue_running_counts(&db).await;
    Ok(Json(queue_running_counts))
}
