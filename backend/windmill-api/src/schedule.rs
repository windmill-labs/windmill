/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    db::{ApiAuthed, DB},
    users::maybe_refresh_folders,
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sql_builder::{prelude::Bind, SqlBuilder};
use sqlx::{Postgres, Transaction};
use std::str::FromStr;
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    jobs::JobKind,
    schedule::Schedule,
    utils::{not_found_if_none, paginate, Pagination, StripPath},
};
use windmill_queue::{self, schedule::push_scheduled_job, QueueTransaction};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_schedule))
        .route("/list_with_jobs", get(list_schedule_with_jobs))
        .route("/get/*path", get(get_schedule))
        .route("/exists/*path", get(exists_schedule))
        .route("/create", post(create_schedule))
        .route("/update/*path", post(edit_schedule))
        .route("/delete/*path", delete(delete_schedule))
        .route("/setenabled/*path", post(set_enabled))
}

pub fn global_service() -> Router {
    Router::new().route("/preview", post(preview_schedule))
}

#[derive(Deserialize)]
pub struct NewSchedule {
    pub path: String,
    pub schedule: String,
    pub timezone: String,
    pub script_path: String,
    pub is_flow: bool,
    pub args: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub on_failure: Option<String>,
    pub on_failure_times: Option<i32>,
    pub on_failure_exact: Option<bool>,
    pub on_failure_extra_args: Option<serde_json::Value>,
    pub on_recovery: Option<String>,
    pub on_recovery_times: Option<i32>,
    pub on_recovery_extra_args: Option<serde_json::Value>,
}

async fn check_path_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
) -> Result<()> {
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM schedule WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);
    if exists {
        return Err(Error::BadRequest(format!(
            "Schedule {} already exists",
            path
        )));
    }
    return Ok(());
}

async fn create_schedule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path(w_id): Path<String>,
    Json(ns): Json<NewSchedule>,
) -> Result<String> {
    let authed = maybe_refresh_folders(&ns.path, &w_id, authed, &db).await;
    let mut tx: QueueTransaction<'_, _> = (rsmq, user_db.begin(&authed).await?).into();

    #[cfg(not(feature = "enterprise"))]
    if ns.on_recovery.is_some() {
        return Err(Error::BadRequest(
            "on_recovery is only available in enterprise version".to_string(),
        ));
    }

    #[cfg(not(feature = "enterprise"))]
    if ns.on_failure_times.is_some() && ns.on_failure_times.unwrap() > 1 {
        return Err(Error::BadRequest(
            "on_failure with a number of times > 1 is only available in enterprise version"
                .to_string(),
        ));
    }

    cron::Schedule::from_str(&ns.schedule).map_err(|e| Error::BadRequest(e.to_string()))?;
    check_path_conflict(tx.transaction_mut(), &w_id, &ns.path).await?;
    check_flow_conflict(
        tx.transaction_mut(),
        &w_id,
        &ns.path,
        ns.is_flow,
        &ns.script_path,
    )
    .await?;

    let schedule = sqlx::query_as!(
        Schedule,
        "INSERT INTO schedule (workspace_id, path, schedule, timezone, edited_by, script_path, \
         is_flow, args, enabled, email, on_failure, on_failure_times, on_failure_exact, on_failure_extra_args, on_recovery, on_recovery_times, on_recovery_extra_args) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17) RETURNING *",
        w_id,
        ns.path,
        ns.schedule,
        ns.timezone,
        &authed.username,
        ns.script_path,
        ns.is_flow,
        ns.args,
        ns.enabled.unwrap_or(false),
        &authed.email,
        ns.on_failure,
        ns.on_failure_times,
        ns.on_failure_exact,
        ns.on_failure_extra_args,
        ns.on_recovery,
        ns.on_recovery_times,
        ns.on_recovery_extra_args,
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("inserting schedule in {w_id}: {e}")))?;

    audit_log(
        &mut tx,
        &authed.username,
        "schedule.create",
        ActionKind::Create,
        &w_id,
        Some(&ns.path.to_string()),
        Some(
            [
                Some(("schedule", ns.schedule.as_str())),
                Some(("script_path", ns.script_path.as_str())),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
    )
    .await?;

    if ns.enabled.unwrap_or(true) {
        tx = push_scheduled_job(&db, tx, schedule).await?
    }
    tx.commit().await?;

    Ok(ns.path.to_string())
}

async fn edit_schedule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(es): Json<EditSchedule>,
) -> Result<String> {
    let path = path.to_path();

    let authed = maybe_refresh_folders(&path, &w_id, authed, &db).await;
    let mut tx: QueueTransaction<'_, rsmq_async::MultiplexedRsmq> =
        (rsmq, user_db.begin(&authed).await?).into();

    cron::Schedule::from_str(&es.schedule).map_err(|e| Error::BadRequest(e.to_string()))?;

    let is_flow = sqlx::query_scalar!(
        "SELECT is_flow FROM schedule WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?;

    let is_flow = not_found_if_none(is_flow, "Schedule", &path)?;

    clear_schedule(tx.transaction_mut(), path, is_flow, &w_id).await?;
    let schedule = sqlx::query_as!(
        Schedule,
        "UPDATE schedule SET schedule = $1, timezone = $2, args = $3, on_failure = $4, on_failure_times = $5, on_failure_exact = $6, on_failure_extra_args = $7, on_recovery = $8, on_recovery_times = $9, on_recovery_extra_args = $10 WHERE path \
         = $11 AND workspace_id = $12 RETURNING *",
        es.schedule,
        es.timezone,
        es.args,
        es.on_failure,
        es.on_failure_times,
        es.on_failure_exact,
        es.on_failure_extra_args,
        es.on_recovery,
        es.on_recovery_times,
        es.on_recovery_extra_args,
        path,
        w_id,
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("updating schedule in {w_id}: {e}")))?;

    audit_log(
        &mut tx,
        &authed.username,
        "schedule.edit",
        ActionKind::Update,
        &w_id,
        Some(&path.to_string()),
        Some(
            [Some(("schedule", es.schedule.as_str()))]
                .into_iter()
                .flatten()
                .collect(),
        ),
    )
    .await?;

    if schedule.enabled {
        tx = push_scheduled_job(&db, tx, schedule).await?;
    }
    tx.commit().await?;

    Ok(path.to_string())
}

#[derive(Deserialize)]
pub struct ListScheduleQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
}

async fn list_schedule(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lsq): Query<ListScheduleQuery>,
) -> JsonResult<Vec<Schedule>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lsq.per_page, page: lsq.page });
    let mut sqlb = SqlBuilder::select_from("schedule")
        .field("*")
        .order_by("edited_at", true)
        .and_where("workspace_id = ?".bind(&w_id))
        .offset(offset)
        .limit(per_page)
        .clone();
    if let Some(path) = lsq.path {
        sqlb.and_where_eq("script_path", "?".bind(&path));
    }
    if let Some(is_flow) = lsq.is_flow {
        sqlb.and_where_eq("is_flow", "?".bind(&is_flow));
    }
    let sql = sqlb.sql().map_err(|e| Error::InternalErr(e.to_string()))?;
    let rows = sqlx::query_as::<_, Schedule>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduleWJobs {
    pub workspace_id: String,
    pub path: String,
    pub edited_by: String,
    pub edited_at: DateTime<chrono::Utc>,
    pub schedule: String,
    pub timezone: String,
    pub enabled: bool,
    pub script_path: String,
    pub is_flow: bool,
    pub args: Option<serde_json::Value>,
    pub extra_perms: serde_json::Value,
    pub email: String,
    pub error: Option<String>,
    pub on_failure: Option<String>,
    pub on_failure_times: Option<i32>,
    pub on_failure_exact: Option<bool>,
    pub on_failure_extra_args: Option<serde_json::Value>,
    pub on_recovery: Option<String>,
    pub on_recovery_times: Option<i32>,
    pub on_recovery_extra_args: Option<serde_json::Value>,
    pub jobs: Option<Vec<serde_json::Value>>,
}

async fn list_schedule_with_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<ScheduleWJobs>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(pagination);
    let rows = sqlx::query_as!(ScheduleWJobs,
        "SELECT schedule.*, t.jobs FROM schedule, LATERAL ( SELECT ARRAY (SELECT json_build_object('id', id, 'success', success, 'duration_ms', duration_ms) FROM completed_job WHERE
        completed_job.schedule_path = schedule.path AND completed_job.workspace_id = $1 AND parent_job IS NULL ORDER BY started_at DESC LIMIT 20) AS jobs ) t
        WHERE schedule.workspace_id = $1 ORDER BY schedule.edited_at desc LIMIT $2 OFFSET $3",
        w_id,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

// SELECT id, title AS item_title, t.tag_array
// FROM   items i, LATERAL (  -- this is an implicit CROSS JOIN
//    SELECT ARRAY (
//       SELECT t.title
//       FROM   items_tags it
//       JOIN   tags       t  ON t.id = it.tag_id
//       WHERE  it.item_id = i.id
//       ) AS tag_array
//    ) t;

async fn get_schedule(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<Schedule> {
    let path = path.to_path();
    let mut tx = user_db.begin(&authed).await?;

    let schedule_o = windmill_queue::schedule::get_schedule_opt(&mut tx, &w_id, path).await?;
    let schedule = not_found_if_none(schedule_o, "Schedule", path)?;
    tx.commit().await?;
    Ok(Json(schedule))
}

async fn exists_schedule(
    Extension(db): Extension<DB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> JsonResult<bool> {
    let mut tx = db.begin().await?;
    let res = windmill_queue::schedule::exists_schedule(&mut tx, w_id, path).await?;
    tx.commit().await?;
    Ok(Json(res))
}

#[derive(Deserialize)]
pub struct PreviewPayload {
    pub schedule: String,
    pub timezone: String,
}

pub async fn preview_schedule(
    Json(payload): Json<PreviewPayload>,
) -> JsonResult<Vec<DateTime<Utc>>> {
    let schedule = cron::Schedule::from_str(&payload.schedule)
        .map_err(|e| Error::BadRequest(e.to_string()))?;

    let tz =
        chrono_tz::Tz::from_str(&payload.timezone).map_err(|e| Error::BadRequest(e.to_string()))?;

    let upcoming: Vec<DateTime<Utc>> = schedule
        .upcoming(tz)
        .take(5)
        // Convert back to UTC for a standardised API response. The client will convert to the local timezone.
        .map(|x| x.with_timezone(&Utc))
        .collect();

    Ok(Json(upcoming))
}

pub async fn set_enabled(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Extension(rsmq): Extension<Option<rsmq_async::MultiplexedRsmq>>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> Result<String> {
    let mut tx: QueueTransaction<'_, rsmq_async::MultiplexedRsmq> =
        (rsmq, user_db.begin(&authed).await?).into();
    let path = path.to_path();
    let schedule_o = sqlx::query_as!(
        Schedule,
        "UPDATE schedule SET enabled = $1, email = $2 WHERE path = $3 AND workspace_id = $4 RETURNING *",
        &payload.enabled,
        authed.email,
        path,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?;

    let schedule = not_found_if_none(schedule_o, "Schedule", path)?;

    clear_schedule(tx.transaction_mut(), path, schedule.is_flow, &w_id).await?;

    audit_log(
        &mut tx,
        &authed.username,
        "schedule.setenabled",
        ActionKind::Update,
        &w_id,
        Some(path),
        Some([("enabled", payload.enabled.to_string().as_ref())].into()),
    )
    .await?;

    if payload.enabled {
        tx = push_scheduled_job(&db, tx, schedule).await?;
    }
    tx.commit().await?;

    Ok(format!(
        "succesfully updated schedule at path {} to status {}",
        path, payload.enabled
    ))
}

async fn delete_schedule(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();

    sqlx::query!(
        "DELETE FROM schedule WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .execute(&mut *tx)
    .await?;

    audit_log(
        &mut *tx,
        &authed.username,
        "schedule.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    Ok(format!("schedule {} deleted", path))
}

async fn check_flow_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
    is_flow: bool,
    script_path: &str,
) -> Result<()> {
    if path != script_path || !is_flow {
        let exists_flow = sqlx::query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM flow WHERE path = $1 AND workspace_id = $2)",
            path,
            w_id
        )
        .fetch_one(&mut **tx)
        .await?
        .unwrap_or(false);
        if exists_flow {
            return Err(Error::BadRequest(format!(
                "The path is the same as a flow, it can only trigger that flow.
            However the provided path is: {script_path} and is_flow is {is_flow}"
            )));
        };
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct EditSchedule {
    pub schedule: String,
    pub timezone: String,
    pub args: Option<serde_json::Value>,
    pub on_failure: Option<String>,
    pub on_failure_times: Option<i32>,
    pub on_failure_exact: Option<bool>,
    pub on_failure_extra_args: Option<serde_json::Value>,
    pub on_recovery: Option<String>,
    pub on_recovery_times: Option<i32>,
    pub on_recovery_extra_args: Option<serde_json::Value>,
}

pub async fn clear_schedule<'c>(
    db: &mut Transaction<'c, Postgres>,
    path: &str,
    is_flow: bool,
    w_id: &str,
) -> Result<()> {
    let job_kind = if is_flow {
        JobKind::Flow
    } else {
        JobKind::Script
    };
    sqlx::query!(
        "DELETE FROM queue WHERE schedule_path = $1 AND running = false AND job_kind = $2 AND workspace_id = $3",
        path,
        job_kind as JobKind,
        w_id
    )
    .execute(&mut **db)
    .await?;
    Ok(())
}

#[derive(Deserialize)]
pub struct SetEnabled {
    pub enabled: bool,
}
