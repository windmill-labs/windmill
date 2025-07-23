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
use uuid::Uuid;
use windmill_common::worker::SpecificTagType;
use windmill_common::{
    db::UserDB,
    error::JsonResult,
    utils::{paginate, Pagination},
    worker::{ALL_TAGS, CUSTOM_TAGS_PER_WORKSPACE, DEFAULT_TAGS, DEFAULT_TAGS_PER_WORKSPACE},
    DB,
};

use crate::{db::ApiAuthed, utils::require_super_admin};

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
        .route("/queue_metrics", get(get_queue_metrics))
        .route("/queue_counts", get(get_queue_counts))
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
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Query(query): Query<ListWorkerQuery>,
) -> JsonResult<Vec<WorkerPing>> {
    let is_super_admin = require_super_admin(&db, &authed.email).await.is_ok();
    let mut tx = user_db.begin(&authed).await?;

    let (per_page, offset) = paginate(Pagination { page: query.page, per_page: query.per_page });

    let rows = sqlx::query_as!(
        WorkerPing,
        "SELECT worker, worker_instance,  EXTRACT(EPOCH FROM (now() - ping_at))::integer as last_ping, started_at, ip, jobs_executed,
        CASE WHEN $4 IS TRUE THEN current_job_id ELSE NULL END as last_job_id, CASE WHEN $4 IS TRUE THEN current_job_workspace_id ELSE NULL END as last_job_workspace_id, 
        custom_tags, worker_group, wm_version, occupancy_rate, occupancy_rate_15s, occupancy_rate_5m, occupancy_rate_30m, memory, vcpus, memory_usage, wm_memory_usage
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

#[derive(Deserialize)]
struct CustomTagQuery {
    workspace: Option<String>,
    show_workspace_restriction: Option<bool>,
}
async fn get_custom_tags(Query(query): Query<CustomTagQuery>) -> JsonResult<Vec<String>> {
    if query.show_workspace_restriction.is_some_and(|x| x) && query.workspace.is_some() {
        return Err(windmill_common::error::Error::BadRequest(
            "Cannot use both workspace and show_workspace_restriction".to_string(),
        ));
    }
    if let Some(workspace) = query.workspace {
        let tags_o = CUSTOM_TAGS_PER_WORKSPACE.read().await;
        let workspace_tags = tags_o
            .1
            .iter()
            .filter(|(_, tag_data)| tag_data.applies_to_workspace(&workspace))
            .map(|(tag, _)| tag.clone())
            .collect::<Vec<String>>();
        let all_tags = tags_o.0.clone();
        return Ok(Json(
            all_tags
                .into_iter()
                .chain(workspace_tags.into_iter())
                .collect(),
        ));
    } else if query.show_workspace_restriction.is_some_and(|x| x) {
        let tags_o = CUSTOM_TAGS_PER_WORKSPACE.read().await;
        let workspace_tags = tags_o
            .1
            .iter()
            .map(|(tag, tag_data)| {
                let separator = tag_data.tag_type.corresponding_separator();
                let workspaces = tag_data.workspaces.join(&*separator.to_string());
                match tag_data.tag_type {
                    SpecificTagType::AllExcluding => {
                        format!("{}({}{})", tag, separator, workspaces)
                    }
                    SpecificTagType::NoneExcept => {
                        format!("{}({})", tag, workspaces)
                    }
                }
            })
            .collect::<Vec<String>>();
        let all_tags = tags_o.0.clone();
        return Ok(Json(
            all_tags
                .into_iter()
                .chain(workspace_tags.into_iter())
                .collect(),
        ));
    }
    Ok(Json(ALL_TAGS.read().await.clone().into()))
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
