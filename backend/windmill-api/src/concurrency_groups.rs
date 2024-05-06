#[cfg(feature = "enterprise")]
use crate::db::{ApiAuthed, DB};
#[cfg(feature = "enterprise")]
use axum::extract::Path;
#[cfg(feature = "enterprise")]
use axum::routing::{delete, get};
#[cfg(feature = "enterprise")]
use axum::{extract::Query, Extension, Json};
use serde::Deserialize;

use axum::Router;

#[cfg(feature = "enterprise")]
use serde::Serialize;
#[cfg(feature = "enterprise")]
use std::collections::HashMap;
#[cfg(feature = "enterprise")]
use uuid::Uuid;
#[cfg(feature = "enterprise")]
use windmill_common::error::Error::{InternalErr, PermissionDenied};
#[cfg(feature = "enterprise")]
use windmill_common::error::JsonResult;

#[cfg(feature = "enterprise")]
pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_concurrency_groups))
        .route("/*id", delete(delete_concurrency_group))
}

#[cfg(not(feature = "enterprise"))]
pub fn global_service() -> Router {
    Router::new()
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/intervals", get(get_concurrent_intervals))
        .route("/job_concurrency_key/:job_id", get(get_concurrency_key))
}

#[cfg(feature = "enterprise")]
#[derive(Serialize)]
pub struct ConcurrencyGroups {
    concurrency_key: String,
    total_running: usize,
    total_completed_within_time_window: usize,
}

#[cfg(feature = "enterprise")]
async fn list_concurrency_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<ConcurrencyGroups>> {
    if !authed.is_admin {
        return Err(PermissionDenied( "Only administrators can see concurrency groups".to_string(),
        ));
    }

    // let concurrency_time_window_s = concurrency.time_window;
    let concurrency_time_window_s = 51;
    let concurrency_groups_raw = sqlx::query_as::<_, (String, serde_json::Value)>(
        "SELECT concurrency_id, job_uuids FROM concurrency_counter ORDER BY concurrency_id ASC",
    )
    .fetch_all(&db)
    .await?;

    let completed_count = sqlx::query!(
        "SELECT key, COUNT(*) as count FROM concurrency_key
            WHERE ended_at IS NOT NULL AND ended_at >=  (now() - INTERVAL '1 second' * $1) GROUP BY key",
        f64::from(concurrency_time_window_s)
    )
    .fetch_all(&db)
    .await
    .map_err(|e| {
        InternalErr(format!(
            "Error getting concurrency limited completed jobs count: {e}"
        ))
    })?;

    let completed_by_key = completed_count
        .iter()
        .fold(HashMap::new(), |mut acc, entry| {
            *acc.entry(entry.key.clone()).or_insert(0) += entry.count.unwrap_or(0);
            acc
        });
    let mut concurrency_groups: Vec<ConcurrencyGroups> = vec![];
    for (concurrency_key, job_uuids_json) in concurrency_groups_raw {
        let job_uuids_map = serde_json::from_value::<HashMap<String, serde_json::Value>>(
            job_uuids_json,
        )
        .map_err(|err| {
            InternalErr(format!(
                "Error deserializing concurrency_counter table content: {}",
                err.to_string()
            ))
        })?;
        concurrency_groups.push(ConcurrencyGroups {
            concurrency_key: concurrency_key.clone(),
            total_running: job_uuids_map.len().into(),
            total_completed_within_time_window: completed_by_key
                .get(&concurrency_key)
                .cloned()
                .unwrap_or(0)
                .try_into()
                .unwrap_or(0),
        })
    }

    return Ok(Json(concurrency_groups));
}

#[cfg(feature = "enterprise")]
async fn delete_concurrency_group(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(concurrency_key): Path<String>,
) -> JsonResult<()> {
    if !authed.is_admin {
        return Err(PermissionDenied(
            "Only administrators can delete concurrency groups".to_string(),
        ));
    }
    let mut tx = db.begin().await?;

    let concurrency_group = sqlx::query_as::<_, (String, i64)>(
        "SELECT concurrency_id, (select COUNT(*) from jsonb_object_keys(job_uuids)) as n_job_uuids FROM concurrency_counter WHERE concurrency_id = $1 FOR UPDATE",
    )
    .bind(concurrency_key.clone())
    .fetch_optional(&mut *tx)
    .await?;

    let n_job_uuids = concurrency_group.map(|cg| cg.1).unwrap_or_default();

    if n_job_uuids > 0 {
        tx.commit().await?;
        return Err(InternalErr(
            "Concurrency group is currently in use, unable to remove it. Retry later.".to_string(),
        ));
    }

    sqlx::query!(
        "DELETE FROM concurrency_counter WHERE concurrency_id = $1",
        concurrency_key.clone(),
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM concurrency_key WHERE key = $1",
        concurrency_key.clone(),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(()))
}


#[derive(Serialize)]
struct ConcurrencyIntervals {
    concurrency_key: Option<String>,
    running_jobs: Vec<RunningJobDuration>,
    completed_jobs: Vec<CompletedJobDuration>,
}

#[derive(Serialize)]
struct CompletedJobDuration {
    started_at: chrono::DateTime<chrono::Utc>,
    ended_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
struct RunningJobDuration {
    started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct ConcurrentIntervalsParams {
    concurrency_key: Option<String>,
    row_limit: Option<i64>,
}

async fn get_concurrent_intervals(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(iq): Query<ConcurrentIntervalsParams>,
) -> JsonResult<ConcurrencyIntervals> {
    let row_limit = iq.row_limit.unwrap_or(1000);
    let concurrency_key = iq.concurrency_key;

    let running_jobs = match &concurrency_key {
        Some(key) => {
            sqlx::query!(
                "SELECT id, started_at FROM queue
                    JOIN concurrency_key ON concurrency_key.job_id = queue.id
                    WHERE started_at IS NOT NULL AND workspace_id = $2 AND key = $1
                    LIMIT $3",
                // "SELECT started_at FROM queue JOIN (
                //         SELECT uuid(jsonb_object_keys(job_uuids)) AS job_id
                //         FROM concurrency_counter WHERE concurrency_id = $1
                //         )
                //     AS expanded_concurrency_table
                //     ON queue.id = expanded_concurrency_table.job_id
                //     WHERE started_at IS NOT NULL
                //     LIMIT $2",
                key,
                w_id,
                row_limit,
            )
            .fetch_all(&db)
            .await?
            .iter()
            .map(|row| RunningJobDuration { started_at: row.started_at.unwrap() })
            .collect()
        }

        None => sqlx::query!(
            "SELECT started_at FROM queue WHERE started_at IS NOT NULL AND workspace_id = $1 AND script_path IS NOT NULL LIMIT $2",
            w_id,
            row_limit,
        )
        .fetch_all(&db)
        .await?
        .iter()
        .map(|row| RunningJobDuration { started_at: row.started_at.unwrap() })
        .collect(),
    };

    let completed_jobs = match &concurrency_key {
        Some(key) => {
            sqlx::query!(
                "SELECT job_id, ended_at, started_at FROM concurrency_key JOIN completed_job ON concurrency_key.job_id = completed_job.id
                    WHERE workspace_id = $1 AND key = $2 AND ended_at IS NOT NULL LIMIT $3",
                w_id,
                key,
                row_limit,
            )
            .fetch_all(&db)
            .await?
            .iter()
            .map(|row| CompletedJobDuration{started_at: row.started_at, ended_at: row.ended_at.unwrap()})
            .collect()
        }
        None => {
            sqlx::query!(
                "SELECT started_at, duration_ms FROM completed_job WHERE workspace_id = $1 LIMIT $2",
                w_id,
                row_limit,
            )
            .fetch_all(&db)
            .await?
            .iter()
            .map(|row| CompletedJobDuration{started_at: row.started_at, ended_at: row.started_at + std::time::Duration::from_millis(row.duration_ms.try_into().unwrap()) })
            .collect()
        }
    };

    Ok(Json(ConcurrencyIntervals {
        concurrency_key,
        running_jobs,
        completed_jobs,
    }))
}

async fn get_concurrency_key(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, job_id)): Path<(String, Uuid)>,
) -> JsonResult<Option<String>> {
    let job = crate::jobs::get_job_internal(&db, &w_id, job_id, true).await?;

    Ok(Json(job.concurrency_key(&db).await?))
}

// async fn get_concurrency_keys_for_jobs(
//     authed: ApiAuthed,
//     Extension(db): Extension<DB>,
//     Path((w_id, job_id)): Path<(String, Uuid)>,
//     ) -> JsonResult<Vec<String>> {
//
//
//     let ret = vec![]
// }
