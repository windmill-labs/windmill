use crate::db::{ApiAuthed, DB};
use crate::jobs::{
    filter_list_completed_query, filter_list_queue_query, ListCompletedQuery, ListQueueQuery,
};
use crate::users::check_scopes;
use axum::extract::Path;
use axum::routing::{delete, get};
use axum::{extract::Query, Extension, Json};
use serde::Deserialize;

use axum::Router;

use serde::Serialize;
use sql_builder::bind::Bind;
use sql_builder::SqlBuilder;
use sqlx::postgres::PgRow;
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use uuid::Uuid;
use windmill_common::db::UserDB;
use windmill_common::error::Error::{InternalErr, PermissionDenied};
use windmill_common::error::JsonResult;

pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_concurrency_groups))
        .route("/prune/id", delete(prune_concurrency_group))
        .route("/:job_id/key", get(get_concurrency_key))
}

pub fn workspaced_service() -> Router {
    Router::new().route("/intervals", get(get_concurrent_intervals))
}

#[derive(Serialize)]
pub struct ConcurrencyGroups {
    concurrency_key: String,
    total_running: usize,
    total_completed_within_time_window: usize,
}

async fn list_concurrency_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<ConcurrencyGroups>> {
    if !authed.is_admin {
        return Err(PermissionDenied(
            "Only administrators can see concurrency groups".to_string(),
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

async fn prune_concurrency_group(
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
    job_id: Option<Uuid>,
    concurrency_key: Option<String>,
    started_at: chrono::DateTime<chrono::Utc>,
    ended_at: chrono::DateTime<chrono::Utc>,
}

impl<'r> FromRow<'r, PgRow> for CompletedJobDuration {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let duration_ms: i64 = row.try_get("duration_ms")?;
        let started_at = row.try_get("started_at")?;
        let ended_at: chrono::DateTime<chrono::Utc> =
            started_at + std::time::Duration::from_millis(duration_ms.try_into().unwrap());
        Ok(Self {
            job_id: row.try_get("id")?,
            concurrency_key: row.try_get("key")?,
            started_at,
            ended_at,
        })
    }
}

#[derive(Serialize, FromRow)]
struct RunningJobDuration {
    #[sqlx(rename = "id")]
    job_id: Option<Uuid>,
    #[sqlx(rename = "key")]
    concurrency_key: Option<String>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
struct ConcurrentIntervalsParams {
    concurrency_key: Option<String>,
    row_limit: Option<i64>,
}

fn join_concurrency_key<'c>(concurrency_key: Option<&String>, mut sqlb: SqlBuilder) -> SqlBuilder {
    match concurrency_key {
        Some(key) => sqlb
            .join("concurrency_key")
            .on_eq("id", "job_id")
            .and_where_eq("key", "?".bind(key)),
        None => sqlb.left().join("concurrency_key").on_eq("id", "job_id"),
    };

    sqlb
}

async fn get_concurrent_intervals(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(iq): Query<ConcurrentIntervalsParams>,
    Query(lq): Query<ListCompletedQuery>,
) -> JsonResult<ConcurrencyIntervals> {
    check_scopes(&authed, || format!("listjobs"))?;

    let row_limit = iq.row_limit.unwrap_or(1000);
    let concurrency_key = iq.concurrency_key;

    let lq_copy = lq.clone();

    let lqq = ListQueueQuery {
        script_path_start: lq_copy.script_path_start,
        script_path_exact: lq_copy.script_path_exact,
        script_hash: lq_copy.script_hash,
        created_by: lq_copy.created_by,
        started_before: lq_copy.started_before,
        started_after: lq_copy.started_after,
        created_before: lq_copy.created_before,
        created_after: lq_copy.created_after,
        created_or_started_before: lq_copy.created_or_started_before,
        created_or_started_after: lq_copy.created_or_started_after,
        running: lq_copy.running,
        parent_job: lq_copy.parent_job,
        order_desc: Some(true),
        job_kinds: lq_copy.job_kinds,
        suspended: lq_copy.suspended,
        args: lq_copy.args,
        tag: lq_copy.tag,
        schedule_path: lq_copy.schedule_path,
        scheduled_for_before_now: lq_copy.scheduled_for_before_now,
        all_workspaces: lq_copy.all_workspaces,
        is_flow_step: lq_copy.is_flow_step,
        has_null_parent: lq_copy.has_null_parent,
        is_not_schedule: lq_copy.is_not_schedule,
    };
    let mut sqlb_q = SqlBuilder::select_from("queue")
        .fields(&["id", "key", "started_at"])
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();
    let mut sqlb_c = SqlBuilder::select_from("completed_job")
        .fields(&["id", "key", "started_at", "duration_ms"])
        .order_by("created_at", lq.order_desc.unwrap_or(true))
        .limit(row_limit)
        .clone();

    sqlb_q = join_concurrency_key(concurrency_key.as_ref(), sqlb_q);
    sqlb_c = join_concurrency_key(concurrency_key.as_ref(), sqlb_c);

    sqlb_q.and_where_is_not_null("started_at");

    let sqlb_all_workspaces: Option<(SqlBuilder, SqlBuilder)> =
        if concurrency_key.is_some() && w_id != "admins" {
            Some((
                filter_list_queue_query(
                    sqlb_q.clone(),
                    &ListQueueQuery { all_workspaces: Some(true), ..lqq.clone() },
                    "admins",
                ),
                filter_list_completed_query(
                    sqlb_c.clone(),
                    &ListCompletedQuery { all_workspaces: Some(true), ..lq.clone() },
                    "admins",
                ),
            ))
        } else {
            None
        };

    sqlb_q = filter_list_queue_query(sqlb_q, &lqq, w_id.as_str());
    sqlb_c = filter_list_completed_query(sqlb_c, &lq, w_id.as_str());

    let sql_q = sqlb_q.query()?;
    let sql_c = sqlb_c.query()?;

    let mut tx = user_db.begin(&authed).await?;
    let running_jobs_user: Vec<RunningJobDuration> = if lq.success.is_none() {
        sqlx::query_as(&sql_q).fetch_all(&mut *tx).await?
    } else {
        vec![]
    };
    let completed_jobs_user: Vec<CompletedJobDuration> = if lq.running.is_none() {
        sqlx::query_as(&sql_c).fetch_all(&mut *tx).await?
    } else {
        vec![]
    };
    tx.commit().await?;

    // To avoid infering information through filtering, don't return obscured
    // jobs if the filters are too specific
    let should_fetch_obscured_jobs = match lq {
        ListCompletedQuery {
            script_path_start: None,
            script_path_exact: None,
            script_hash: None,
            created_by: None,
            success: None,
            running: None,
            parent_job: None,
            is_skipped: None | Some(false),
            suspended: None,
            schedule_path: None,
            args: None,
            result: None,
            tag: None,
            scheduled_for_before_now: None,
            has_null_parent: None,
            label: None,
            is_not_schedule: None,
            started_before: _,
            started_after: _,
            created_before: _,
            created_after: _,
            created_or_started_before: _,
            created_or_started_after: _,
            order_desc: _,
            job_kinds: _,
            is_flow_step: _,
            all_workspaces: _,
        } => true,
        _ => false,
    };

    // This second transaction using the raw db lets us get info for jobs that
    // the user has no access to. Before returning that, we will hide the ids
    if should_fetch_obscured_jobs && concurrency_key.is_some() {
        let (sql_q, sql_c) = if let Some(sqlb) = sqlb_all_workspaces {
            (sqlb.0.query()?, sqlb.1.query()?)
        } else {
            (sql_q, sql_c)
        };

        let running_jobs_db: Vec<RunningJobDuration> = if lq.success.is_none() {
            sqlx::query_as(&sql_q).fetch_all(&db).await?
        } else {
            vec![]
        };
        let completed_jobs_db: Vec<CompletedJobDuration> = if lq.running.is_none() {
            sqlx::query_as(&sql_c).fetch_all(&db).await?
        } else {
            vec![]
        };

        let running_jobs = running_jobs_db
            .into_iter()
            .map(|r| {
                if running_jobs_user
                    .iter()
                    .any(|u| u.job_id.unwrap() == r.job_id.unwrap())
                {
                    RunningJobDuration { ..r }
                } else {
                    RunningJobDuration { job_id: None, ..r }
                }
            })
            .collect();
        let completed_jobs = completed_jobs_db
            .into_iter()
            .map(|r| {
                if completed_jobs_user
                    .iter()
                    .any(|u| u.job_id.unwrap() == r.job_id.unwrap())
                {
                    CompletedJobDuration { ..r }
                } else {
                    CompletedJobDuration { job_id: None, ..r }
                }
            })
            .collect();

        return Ok(Json(ConcurrencyIntervals {
            concurrency_key,
            running_jobs,
            completed_jobs,
        }));
    }

    Ok(Json(ConcurrencyIntervals {
        concurrency_key,
        running_jobs: running_jobs_user,
        completed_jobs: completed_jobs_user,
    }))
}

async fn get_concurrency_key(
    Extension(db): Extension<DB>,
    Path(job_id): Path<Uuid>,
) -> JsonResult<Option<String>> {
    let key = sqlx::query_scalar!("SELECT key FROM concurrency_key WHERE job_id = $1", job_id)
        .fetch_optional(&db)
        .await?;
    Ok(Json(key))
}
