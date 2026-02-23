/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

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
use windmill_api_auth::{check_scopes, maybe_refresh_folders, require_super_admin, ApiAuthed};
use windmill_audit::audit_oss::audit_log;
use windmill_audit::ActionKind;
use windmill_common::DB;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult, Result},
    schedule::Schedule,
    utils::{not_found_if_none, paginate, Pagination, ScheduleType, StripPath},
    worker::to_raw_value,
};
use windmill_git_sync::{handle_deployment_metadata, DeployedObject};
use windmill_queue::schedule::push_scheduled_job;

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
        .route("/setdefaulthandler", post(set_default_error_handler))
    // .route("/catchup/*path", post(do_catchup).get(list_catchup))
}

pub fn global_service() -> Router {
    Router::new().route("/preview", post(preview_schedule))
}

#[derive(Deserialize)]
pub struct NewSchedule {
    pub path: String,
    pub schedule: String,
    pub timezone: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub no_flow_overlap: Option<bool>,
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
    pub on_success: Option<String>,
    pub on_success_extra_args: Option<serde_json::Value>,
    pub ws_error_handler_muted: Option<bool>,
    pub retry: Option<serde_json::Value>,
    pub tag: Option<String>,
    pub paused_until: Option<DateTime<Utc>>,
    pub cron_version: Option<String>,
    pub dynamic_skip: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorOrRecoveryHandler {
    pub handler_type: HandlerType,
    pub override_existing: bool,

    pub path: Option<String>,
    pub extra_args: Option<serde_json::Value>,
    pub number_of_occurence: Option<i32>,
    pub number_of_occurence_exact: Option<bool>,
    pub workspace_handler_muted: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum HandlerType {
    Error,
    Recovery,
    Success,
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

fn to_json_raw_opt(
    value: Option<&serde_json::Value>,
) -> Option<sqlx::types::Json<Box<serde_json::value::RawValue>>> {
    value.map(|v| sqlx::types::Json(to_raw_value(&v)))
}

/// Validate that a dynamic skip handler (script or flow) exists
async fn validate_dynamic_skip<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    handler_path: &str,
) -> Result<()> {
    // Check for script only (flows are not supported in the UI)
    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(
            SELECT 1 FROM script
            WHERE workspace_id = $1 AND path = $2 AND archived = false AND deleted = false
        )",
        w_id,
        handler_path
    )
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);

    if exists {
        Ok(())
    } else {
        Err(Error::BadRequest(format!(
            "Dynamic skip handler '{}' not found. The handler must be an existing, non-archived script at schedule creation time.",
            handler_path
        )))
    }
}

async fn create_schedule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(ns): Json<NewSchedule>,
) -> Result<String> {
    check_scopes(&authed, || format!("schedules:write:{}", ns.path))?;

    let authed = maybe_refresh_folders(&ns.path, &w_id, authed, &db).await;

    #[cfg(not(feature = "enterprise"))]
    if ns.on_recovery.is_some() {
        return Err(Error::BadRequest(
            "on_recovery is only available in enterprise version".to_string(),
        ));
    }

    #[cfg(not(feature = "enterprise"))]
    if ns.on_success.is_some() {
        return Err(Error::BadRequest(
            "on_success is only available in enterprise version".to_string(),
        ));
    }

    #[cfg(not(feature = "enterprise"))]
    if ns.on_failure_times.is_some() && ns.on_failure_times.unwrap() > 1 {
        return Err(Error::BadRequest(
            "on_failure with a number of times > 1 is only available in enterprise version"
                .to_string(),
        ));
    }

    let mut tx: Transaction<'_, Postgres> = user_db.begin(&authed).await?;

    // Check schedule for error
    ScheduleType::from_str(&ns.schedule, ns.cron_version.as_deref(), true)?;

    check_path_conflict(&mut tx, &w_id, &ns.path).await?;
    check_flow_conflict(&mut tx, &w_id, &ns.path, ns.is_flow, &ns.script_path).await?;

    // Validate dynamic_skip if provided
    if let Some(handler_path) = &ns.dynamic_skip {
        validate_dynamic_skip(&mut tx, &w_id, handler_path).await?;
    }

    let schedule = sqlx::query_as!(
        Schedule,
        r#"
        INSERT INTO schedule (
            workspace_id, path, schedule, timezone, edited_by, script_path,
            is_flow, args, enabled, email,
            on_failure, on_failure_times, on_failure_exact, on_failure_extra_args,
            on_recovery, on_recovery_times, on_recovery_extra_args,
            on_success, on_success_extra_args,
            ws_error_handler_muted, retry, summary, no_flow_overlap,
            tag, paused_until, cron_version, description, dynamic_skip
        ) VALUES (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10,
            $11, $12, $13, $14,
            $15, $16, $17,
            $18, $19,
            $20, $21, $22, $23,
            $24, $25, $26, $27, $28
        )
        RETURNING
            workspace_id,
            path,
            edited_by,
            edited_at,
            schedule,
            timezone,
            enabled,
            script_path,
            is_flow,
            args AS "args: _",
            extra_perms,
            email,
            error,
            on_failure,
            on_failure_times,
            on_failure_exact,
            on_failure_extra_args AS "on_failure_extra_args: _",
            on_recovery,
            on_recovery_times,
            on_recovery_extra_args AS "on_recovery_extra_args: _",
            on_success,
            on_success_extra_args  AS "on_success_extra_args: _",
            ws_error_handler_muted,
            retry,
            no_flow_overlap,
            summary,
            description,
            tag,
            paused_until,
            cron_version,
            dynamic_skip
        "#,
        w_id,
        ns.path,
        ns.schedule,
        ns.timezone,
        authed.username,
        ns.script_path,
        ns.is_flow,
        to_json_raw_opt(ns.args.as_ref())
            as Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
        ns.enabled.unwrap_or(false),
        authed.email,
        ns.on_failure,
        ns.on_failure_times,
        ns.on_failure_exact,
        to_json_raw_opt(ns.on_failure_extra_args.as_ref())
            as Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
        ns.on_recovery,
        ns.on_recovery_times,
        to_json_raw_opt(ns.on_recovery_extra_args.as_ref())
            as Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
        ns.on_success,
        to_json_raw_opt(ns.on_success_extra_args.as_ref())
            as Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
        ns.ws_error_handler_muted.unwrap_or(false),
        ns.retry,
        ns.summary,
        ns.no_flow_overlap.unwrap_or(false),
        ns.tag,
        ns.paused_until,
        ns.cron_version.clone().unwrap_or_else(|| "v2".to_string()),
        ns.description,
        ns.dynamic_skip
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("inserting schedule in {w_id}: {e:#}")))?;

    audit_log(
        &mut *tx,
        &authed,
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
        tx = push_scheduled_job(&db, tx, &schedule, Some(&authed.clone().into()), None).await?
    }
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Schedule { path: ns.path.clone() },
        Some(format!("Schedule '{}' created", ns.path.clone())),
        true,
        None,
    )
    .await?;

    Ok(ns.path.to_string())
}

async fn edit_schedule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(es): Json<EditSchedule>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("schedules:write:{}", path))?;

    let authed = maybe_refresh_folders(&path, &w_id, authed, &db).await;
    let mut tx = user_db.begin(&authed).await?;

    // Check schedule for error
    ScheduleType::from_str(&es.schedule, es.cron_version.as_deref(), true)?;

    // Validate dynamic_skip if provided
    if let Some(handler_path) = &es.dynamic_skip {
        validate_dynamic_skip(&mut tx, &w_id, handler_path).await?;
    }

    clear_schedule(&mut tx, path, &w_id).await?;
    let schedule = sqlx::query_as!(
        Schedule,
        r#"
        UPDATE schedule SET
            schedule                = $1,
            timezone                = $2,
            args                    = $3,
            on_failure              = $4,
            on_failure_times        = $5,
            on_failure_exact        = $6,
            on_failure_extra_args   = $7,
            on_recovery             = $8,
            on_recovery_times       = $9,
            on_recovery_extra_args  = $10,
            on_success              = $11,
            on_success_extra_args   = $12,
            ws_error_handler_muted  = $13,
            retry                   = $14,
            summary                 = $15,
            no_flow_overlap         = $16,
            tag                     = $17,
            paused_until            = $18,
            path                    = $19,
            workspace_id            = $20,
            cron_version            = COALESCE($21, cron_version),
            description             = $22,
            dynamic_skip        = $23
        WHERE path = $19 AND workspace_id = $20
        RETURNING
            workspace_id,
            path,
            edited_by,
            edited_at,
            schedule,
            timezone,
            enabled,
            script_path,
            is_flow,
            args AS "args: _",
            extra_perms,
            email,
            error,
            on_failure,
            on_failure_times,
            on_failure_exact,
            on_failure_extra_args AS "on_failure_extra_args: _",
            on_recovery,
            on_recovery_times,
            on_recovery_extra_args AS "on_recovery_extra_args: _",
            on_success,
            on_success_extra_args AS "on_success_extra_args: _",
            ws_error_handler_muted,
            retry,
            no_flow_overlap,
            summary,
            description,
            tag,
            paused_until,
            cron_version,
            dynamic_skip
        "#,
        es.schedule,
        es.timezone,
        to_json_raw_opt(es.args.as_ref())
            as Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
        es.on_failure,
        es.on_failure_times,
        es.on_failure_exact,
        to_json_raw_opt(es.on_failure_extra_args.as_ref())
            as Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
        es.on_recovery,
        es.on_recovery_times,
        to_json_raw_opt(es.on_recovery_extra_args.as_ref())
            as Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
        es.on_success,
        to_json_raw_opt(es.on_success_extra_args.as_ref())
            as Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
        es.ws_error_handler_muted.unwrap_or(false),
        es.retry,
        es.summary,
        es.no_flow_overlap.unwrap_or(false),
        es.tag,
        es.paused_until,
        path,
        w_id,
        es.cron_version,
        es.description,
        es.dynamic_skip
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| Error::internal_err(format!("updating schedule in {w_id}: {e:#}")))?;

    audit_log(
        &mut *tx,
        &authed,
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
        tx = push_scheduled_job(&db, tx, &schedule, None, None).await?;
    }
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Schedule { path: path.to_string() },
        None,
        true,
        None,
    )
    .await?;

    Ok(path.to_string())
}

#[derive(Deserialize)]
pub struct ListScheduleQuery {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub path: Option<String>,
    pub is_flow: Option<bool>,
    // filter by matching a subset of the args using base64 encoded json subset
    pub args: Option<String>,
    pub path_start: Option<String>,
    // exact match on schedule path
    pub schedule_path: Option<String>,
    // filter on description (pattern match)
    pub description: Option<String>,
    // filter on summary (pattern match)
    pub summary: Option<String>,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct ScheduleLight {
    pub workspace_id: String,
    pub path: String,
    pub edited_by: String,
    pub edited_at: DateTime<chrono::Utc>,
    pub schedule: String,
    pub timezone: String,
    pub enabled: bool,
    pub script_path: String,
    pub is_flow: bool,
    pub summary: Option<String>,
    pub extra_perms: serde_json::Value,
}
async fn list_schedule(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(lsq): Query<ListScheduleQuery>,
) -> JsonResult<Vec<ScheduleLight>> {
    let mut tx = user_db.begin(&authed).await?;
    let (per_page, offset) = paginate(Pagination { per_page: lsq.per_page, page: lsq.page });
    let mut sqlb = SqlBuilder::select_from("schedule")
        .fields(&[
            "workspace_id",
            "path",
            "edited_by",
            "edited_at",
            "schedule",
            "timezone",
            "enabled",
            "script_path",
            "is_flow",
            "summary",
            "extra_perms",
        ])
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
    if let Some(args) = &lsq.args {
        sqlb.and_where("args @> ?".bind(&args.replace("'", "''")));
    }
    if let Some(path_start) = &lsq.path_start {
        sqlb.and_where_like_left("path", path_start);
    }
    if let Some(schedule_path) = &lsq.schedule_path {
        sqlb.and_where_eq("path", "?".bind(schedule_path));
    }
    if let Some(description) = &lsq.description {
        sqlb.and_where(&format!(
            "description ILIKE '%{}%'",
            description.replace("'", "''")
        ));
    }
    if let Some(summary) = &lsq.summary {
        sqlb.and_where(&format!("summary ILIKE '%{}%'", summary.replace("'", "''")));
    }
    let sql = sqlb.sql().map_err(|e| Error::internal_err(e.to_string()))?;
    let rows = sqlx::query_as::<_, ScheduleLight>(&sql)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(rows))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScheduleWJobs {
    pub path: String,
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
        // Query plan:
        // - use of the `ix_completed_job_workspace_id_started_at_new_2` index first, then;
        // - use of the `ix_v2_job_root_by_path` index; hence the `parent_job IS NULL` clause.
        // - both `workspace_id = $1` checks are required to hit both indexes.
        "SELECT
            schedule.path, t.jobs FROM schedule,
            LATERAL(SELECT ARRAY(
                SELECT json_build_object('id', id, 'success', status = 'success', 'duration_ms', duration_ms)
                FROM v2_job_completed c JOIN v2_job j USING (id)
                WHERE trigger_kind = 'schedule'
                    AND trigger = schedule.path
                    AND c.workspace_id = $1
                    AND j.workspace_id = $1
                    AND parent_job IS NULL AND runnable_path = schedule.script_path
                    AND status <> 'skipped'
                ORDER BY completed_at DESC
                LIMIT 20
            ) AS jobs) t
        WHERE workspace_id = $1
        ORDER BY edited_at DESC
        LIMIT $2 OFFSET $3",
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
    check_scopes(&authed, || format!("schedules:read:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    let schedule_o = windmill_queue::schedule::get_schedule_opt(&mut *tx, &w_id, path).await?;
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
    pub cron_version: Option<String>,
}

pub async fn preview_schedule(
    Json(payload): Json<PreviewPayload>,
) -> JsonResult<Vec<DateTime<Utc>>> {
    let schedule =
        ScheduleType::from_str(&payload.schedule, payload.cron_version.as_deref(), true)?;

    let tz =
        chrono_tz::Tz::from_str(&payload.timezone).map_err(|e| Error::BadRequest(e.to_string()))?;

    let upcoming: Vec<DateTime<Utc>> = schedule.upcoming(tz, 5)?;

    Ok(Json(upcoming))
}

pub async fn set_enabled(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
    Json(payload): Json<SetEnabled>,
) -> Result<String> {
    let mut tx = user_db.begin(&authed).await?;
    let path = path.to_path();
    check_scopes(&authed, || format!("schedules:write:{}", path))?;
    let schedule_o = sqlx::query_as!(
        Schedule,
        r#"
        UPDATE schedule SET
            enabled = $1,
            email = $2
        WHERE path = $3 AND workspace_id = $4
        RETURNING
            workspace_id,
            path,
            edited_by,
            edited_at,
            schedule,
            timezone,
            enabled,
            script_path,
            is_flow,
            args AS "args: _",
            extra_perms,
            email,
            error,
            on_failure,
            on_failure_times,
            on_failure_exact,
            on_failure_extra_args AS "on_failure_extra_args: _",
            on_recovery,
            on_recovery_times,
            on_recovery_extra_args AS "on_recovery_extra_args: _",
            on_success,
            on_success_extra_args AS "on_success_extra_args: _",
            ws_error_handler_muted,
            retry,
            no_flow_overlap,
            summary,
            description,
            tag,
            paused_until,
            cron_version,
            dynamic_skip
        "#,
        payload.enabled,
        authed.email,
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let schedule = not_found_if_none(schedule_o, "Schedule", path)?;

    clear_schedule(&mut tx, path, &w_id).await?;

    audit_log(
        &mut *tx,
        &authed,
        "schedule.setenabled",
        ActionKind::Update,
        &w_id,
        Some(path),
        Some([("enabled", payload.enabled.to_string().as_ref())].into()),
    )
    .await?;

    if payload.enabled {
        tx = push_scheduled_job(&db, tx, &schedule, None, None).await?;
    }
    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Schedule { path: path.to_string() },
        None,
        true,
        None,
    )
    .await?;

    Ok(format!(
        "succesfully updated schedule at path {} to status {}",
        path, payload.enabled
    ))
}

// pub async fn do_catchup(
//     authed: ApiAuthed,
//     Extension(db): Extension<DB>,
//     Extension(user_db): Extension<UserDB>,
// //     Path((w_id, path)): Path<(String, StripPath)>,
//     Json(payload): Json<SetEnabled>,
// ) -> Result<String> {
//     let mut tx: QueueTransaction<'_, rsmq_async::MultiplexedRsmq> =
//         (user_db.begin(&authed).await?).into();
//     let path = path.to_path();
//     let schedule_o = sqlx::query_as!(
//         Schedule,
//         "UPDATE schedule SET enabled = $1, email = $2 WHERE path = $3 AND workspace_id = $4 RETURNING *",
//         &payload.enabled,
//         authed.email,
//         path,
//         w_id
//     )
//     .fetch_optional(&mut *tx)
//     .await?;

//     let schedule = not_found_if_none(schedule_o, "Schedule", path)?;

//     clear_schedule(&mut tx, path, &w_id).await?;

//     audit_log(
//         &mut *tx,
//         &authed,
//         "schedule.setenabled",
//         ActionKind::Update,
//         &w_id,
//         Some(path),
//         Some([("enabled", payload.enabled.to_string().as_ref())].into()),
//     )
//     .await?;

//     if payload.enabled {
//         tx = push_scheduled_job(&db, tx, &schedule, None).await?;
//     }
//     tx.commit().await?;

//     Ok(format!(
//         "succesfully updated schedule at path {} to status {}",
//         path, payload.enabled
//     ))
// }

async fn delete_schedule(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, path)): Path<(String, StripPath)>,
) -> Result<String> {
    let path = path.to_path();
    check_scopes(&authed, || format!("schedules:write:{}", path))?;
    let mut tx = user_db.begin(&authed).await?;

    clear_schedule(&mut tx, path, &w_id).await?;
    let exists = sqlx::query_scalar!(
        "SELECT 1 FROM schedule WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    if exists.is_none() {
        return Err(windmill_common::error::Error::NotFound(format!(
            "Schedule {} not found",
            path
        )));
    }

    let del = sqlx::query_scalar!(
        "DELETE FROM schedule WHERE path = $1 AND workspace_id = $2 RETURNING 1",
        path,
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .flatten();

    if del.is_none() {
        return Err(windmill_common::error::Error::NotAuthorized(format!(
            "Not authorized to delete schedule {}",
            path
        )));
    }

    audit_log(
        &mut *tx,
        &authed,
        "schedule.delete",
        ActionKind::Delete,
        &w_id,
        Some(path),
        None,
    )
    .await?;

    tx.commit().await?;

    handle_deployment_metadata(
        &authed.email,
        &authed.username,
        &db,
        &w_id,
        DeployedObject::Schedule { path: path.to_string() },
        Some(format!("Schedule '{}' deleted", path)),
        true,
        None,
    )
    .await?;

    Ok(format!("schedule {} deleted", path))
}

async fn set_default_error_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(payload): Json<ErrorOrRecoveryHandler>,
) -> Result<()> {
    require_super_admin(&db, &authed.email).await?;
    let (key, value) = match payload.handler_type {
        HandlerType::Error => {
            let key = format!("default_error_handler_{}", w_id);
            if let Some(payload_path) = payload.path.as_ref() {
                let value = serde_json::json!({
                    "wsErrorHandlerMuted": payload.workspace_handler_muted,
                    "errorHandlerPath": payload_path,
                    "errorHandlerExtraArgs": payload.extra_args,
                    "failedTimes": payload.number_of_occurence,
                    "failedExact": payload.number_of_occurence_exact,
                });
                (key, Some(value))
            } else {
                (key, None)
            }
        }
        HandlerType::Recovery => {
            let key = format!("default_recovery_handler_{}", w_id);
            if let Some(payload_path) = payload.path.as_ref() {
                let value = serde_json::json!({
                    "recoveryHandlerPath": payload_path,
                    "recoveryHandlerExtraArgs": payload.extra_args,
                    "recoveredTimes": payload.number_of_occurence,
                });
                (key, Some(value))
            } else {
                (key, None)
            }
        }
        HandlerType::Success => {
            let key = format!("default_success_handler_{}", w_id);
            if let Some(payload_path) = payload.path.as_ref() {
                let value = serde_json::json!({
                    "successHandlerPath": payload_path,
                    "successHandlerExtraArgs": payload.extra_args,
                });
                (key, Some(value))
            } else {
                (key, None)
            }
        }
    };

    if let Some(value_content) = value {
        windmill_api_settings::set_global_setting_internal(&db, key, value_content).await?;
    } else {
        windmill_api_settings::delete_global_setting(&db, key.as_str()).await?;
    }

    if payload.override_existing {
        let updated_schedules: Vec<String>;
        match payload.handler_type {
            HandlerType::Error => {
                if payload.path.is_some() {
                    updated_schedules = sqlx::query_scalar!(
                        "UPDATE schedule SET ws_error_handler_muted = $1, on_failure = $2, on_failure_extra_args = $3, on_failure_times = $4, on_failure_exact = $5 WHERE workspace_id = $6 RETURNING path",
                        payload.workspace_handler_muted,
                        payload.path,
                        payload.extra_args,
                        payload.number_of_occurence,
                        payload.number_of_occurence_exact,
                        w_id,
                    )
                    .fetch_all(&db)
                    .await?;
                } else {
                    updated_schedules = sqlx::query_scalar!(
                        "UPDATE schedule SET ws_error_handler_muted = false, on_failure = NULL, on_failure_extra_args = NULL, on_failure_times = NULL, on_failure_exact = NULL WHERE workspace_id = $1 RETURNING path",
                        w_id,
                    )
                    .fetch_all(&db)
                    .await?;
                }
            }
            HandlerType::Recovery => {
                if payload.path.is_some() {
                    updated_schedules = sqlx::query_scalar!(
                        "UPDATE schedule SET on_recovery = $1, on_recovery_extra_args = $2, on_recovery_times = $3 WHERE workspace_id = $4 RETURNING path",
                        payload.path,
                        payload.extra_args,
                        payload.number_of_occurence,
                        w_id,
                    )
                    .fetch_all(&db)
                    .await?;
                } else {
                    updated_schedules = sqlx::query_scalar!(
                        "UPDATE schedule SET on_recovery = NULL, on_recovery_extra_args = NULL, on_recovery_times = NULL WHERE workspace_id = $1 RETURNING path",
                        w_id,
                    )
                    .fetch_all(&db)
                    .await?;
                }
            }
            HandlerType::Success => {
                if payload.path.is_some() {
                    updated_schedules = sqlx::query_scalar!(
                        "UPDATE schedule SET on_success = $1, on_success_extra_args = $2 WHERE workspace_id = $3 RETURNING path",
                        payload.path,
                        payload.extra_args,
                        w_id,
                    )
                    .fetch_all(&db)
                    .await?;
                } else {
                    updated_schedules = sqlx::query_scalar!(
                        "UPDATE schedule SET on_success = NULL, on_success_extra_args = NULL WHERE workspace_id = $1 RETURNING path",
                        w_id,
                    )
                    .fetch_all(&db)
                    .await?;
                }
            }
        }
        for updated_schedule_path in updated_schedules {
            handle_deployment_metadata(
                &authed.email,
                &authed.username,
                &db,
                &w_id,
                DeployedObject::Schedule { path: updated_schedule_path },
                None,
                true,
                None,
            )
            .await?;
        }
    }
    Ok(())
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
    pub summary: Option<String>,
    pub description: Option<String>,
    pub on_failure: Option<String>,
    pub on_failure_times: Option<i32>,
    pub on_failure_exact: Option<bool>,
    pub on_failure_extra_args: Option<serde_json::Value>,
    pub on_recovery: Option<String>,
    pub on_recovery_times: Option<i32>,
    pub on_recovery_extra_args: Option<serde_json::Value>,
    pub on_success: Option<String>,
    pub on_success_extra_args: Option<serde_json::Value>,
    pub ws_error_handler_muted: Option<bool>,
    pub retry: Option<serde_json::Value>,
    pub no_flow_overlap: Option<bool>,
    pub tag: Option<String>,
    pub paused_until: Option<DateTime<Utc>>,
    pub cron_version: Option<String>,
    pub dynamic_skip: Option<String>,
}

pub use windmill_queue::schedule::clear_schedule;

#[derive(Deserialize)]
pub struct SetEnabled {
    pub enabled: bool,
}

// #[derive(Deserialize)]
// pub struct Catchup {
//     pub from: DateTime<Utc>,
//     pub to: Option<DateTime<Utc>>,
// }
