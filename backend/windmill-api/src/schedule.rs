/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::str::FromStr;

use crate::{
    db::{UserDB, DB},
    users::Authed,
};
use axum::{
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, FixedOffset};
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{Error, JsonResult, Result},
    schedule::Schedule,
    utils::{not_found_if_none, paginate, Pagination, StripPath},
};
use windmill_queue::{self, schedule::push_scheduled_job, JobKind};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_schedule))
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
    pub offset: i32,
    pub script_path: String,
    pub is_flow: bool,
    pub args: Option<serde_json::Value>,
    pub enabled: Option<bool>,
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
    .fetch_one(tx)
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
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ns): Json<NewSchedule>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    cron::Schedule::from_str(&ns.schedule).map_err(|e| Error::BadRequest(e.to_string()))?;
    check_path_conflict(&mut tx, &w_id, &ns.path).await?;
    check_flow_conflict(&mut tx, &w_id, &ns.path, ns.is_flow, &ns.script_path).await?;

    let schedule = sqlx::query_as!(
        Schedule,
        "INSERT INTO schedule (workspace_id, path, schedule, offset_, edited_by, script_path, \
         is_flow, args, enabled, email) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
        w_id,
        ns.path,
        ns.schedule,
        ns.offset,
        &authed.username,
        ns.script_path,
        ns.is_flow,
        ns.args,
        ns.enabled.unwrap_or(false),
        &authed.email
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
        tx = push_scheduled_job(tx, schedule).await?
    }
    tx.commit().await?;

    Ok(ns.path.to_string())
}

async fn edit_schedule(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(es): Json<EditSchedule>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();

    cron::Schedule::from_str(&es.schedule).map_err(|e| Error::BadRequest(e.to_string()))?;

    let is_flow = sqlx::query_scalar!(
        "SELECT is_flow FROM schedule WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .fetch_one(&mut tx)
    .await?;

    clear_schedule(&mut tx, path, is_flow).await?;
    let schedule = sqlx::query_as!(
        Schedule,
        "UPDATE schedule SET schedule = $1, args = $2 WHERE path \
         = $3 AND workspace_id = $4 RETURNING *",
        es.schedule,
        es.args,
        path,
        w_id,
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("updating schedule in {w_id}: {e}")))?;

    if schedule.enabled {
        tx = push_scheduled_job(tx, schedule).await?;
    }

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
    tx.commit().await?;

    Ok(path.to_string())
}

async fn list_schedule(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> JsonResult<Vec<Schedule>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(pagination);
    let rows = sqlx::query_as!(
        Schedule,
        "SELECT * FROM schedule WHERE workspace_id = $1 ORDER BY edited_at desc LIMIT $2 OFFSET $3",
        w_id,
        per_page as i64,
        offset as i64
    )
    .fetch_all(&mut tx)
    .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

async fn get_schedule(
    authed: Authed,
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

pub async fn preview_schedule(
    Json(payload): Json<PreviewPayload>,
) -> JsonResult<Vec<DateTime<chrono::Utc>>> {
    let schedule = cron::Schedule::from_str(&payload.schedule)
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    let upcoming: Vec<DateTime<chrono::Utc>> = schedule
        .upcoming(get_offset(payload.offset))
        .take(10)
        .map(|x| x.into())
        .collect();

    Ok(Json(upcoming))
}

pub async fn set_enabled(
    authed: Authed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
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

    clear_schedule(&mut tx, path, schedule.is_flow).await?;

    if payload.enabled {
        tx = push_scheduled_job(tx, schedule).await?;
    }
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
    tx.commit().await?;
    Ok(format!(
        "succesfully updated schedule at path {} to status {}",
        path, payload.enabled
    ))
}

async fn delete_schedule(
    authed: Authed,
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
    .execute(&mut tx)
    .await?;

    audit_log(
        &mut tx,
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
        .fetch_one(tx)
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
    pub args: Option<serde_json::Value>,
}

pub async fn clear_schedule<'c>(
    db: &mut Transaction<'c, Postgres>,
    path: &str,
    is_flow: bool,
) -> Result<()> {
    let job_kind = if is_flow {
        JobKind::Flow
    } else {
        JobKind::Script
    };
    sqlx::query!(
        "DELETE FROM queue WHERE schedule_path = $1 AND running = false AND job_kind = $2",
        path,
        job_kind: JobKind
    )
    .execute(db)
    .await?;
    Ok(())
}

#[derive(Deserialize)]
pub struct PreviewPayload {
    pub schedule: String,
    pub offset: Option<i32>,
}

fn get_offset(offset: Option<i32>) -> FixedOffset {
    FixedOffset::west_opt(offset.unwrap_or(0) * 60).expect("Invalid offset")
}

#[derive(Deserialize)]
pub struct SetEnabled {
    pub enabled: bool,
}
