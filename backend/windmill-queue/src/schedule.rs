/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::str::FromStr;

use chrono::{DateTime, Duration, FixedOffset};
use serde::{Deserialize, Serialize};
use sqlx::{query_scalar, FromRow, Postgres, Transaction};
use windmill_audit::{audit_log, ActionKind};
use windmill_common::{
    error::{self, Error, Result},
    utils::{get_owner_from_path, not_found_if_none, now_from_db, paginate, Pagination, StripPath},
};

use crate::{push, JobPayload};

#[derive(FromRow, Serialize, Deserialize, Debug)]
pub struct Schedule {
    pub workspace_id: String,
    pub path: String,
    pub edited_by: String,
    pub edited_at: DateTime<chrono::Utc>,
    pub schedule: String,
    pub offset_: i32,
    pub enabled: bool,
    pub script_path: String,
    pub is_flow: bool,
    pub args: Option<serde_json::Value>,
    pub extra_perms: serde_json::Value,
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

pub async fn push_scheduled_job<'c>(
    mut tx: Transaction<'c, Postgres>,
    schedule: Schedule,
) -> Result<Transaction<'c, Postgres>> {
    let sched = cron::Schedule::from_str(&schedule.schedule)
        .map_err(|e| error::Error::BadRequest(e.to_string()))?;

    let offset = Duration::minutes(schedule.offset_.into());
    let now = now_from_db(&mut tx).await?;
    let next = sched
        .after(&(now - offset + Duration::seconds(1)))
        .next()
        .expect("a schedule should have a next event")
        + offset;

    let already_exists: bool = query_scalar!(
        "SELECT EXISTS (SELECT 1 FROM queue WHERE workspace_id = $1 AND schedule_path = $2 AND scheduled_for = $3)",
        &schedule.workspace_id,
        &schedule.path,
        next
    )
    .fetch_one(&mut tx)
    .await?
    .unwrap_or(false);

    if already_exists {
        return Ok(tx);
    }

    let mut args: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

    if let Some(args_v) = schedule.args {
        if let serde_json::Value::Object(args_m) = args_v {
            args = args_m
        } else {
            return Err(error::Error::ExecutionErr(
                "args of scripts needs to be dict".to_string(),
            ));
        }
    }

    let payload = if schedule.is_flow {
        JobPayload::Flow(schedule.script_path)
    } else {
        JobPayload::ScriptHash {
            hash: windmill_common::get_latest_hash_for_path(
                &mut tx,
                &schedule.workspace_id,
                &schedule.script_path,
            )
            .await?,
            path: schedule.script_path,
        }
    };

    let (_, tx) = push(
        tx,
        &schedule.workspace_id,
        payload,
        args,
        &schedule_to_user(&schedule.path),
        get_owner_from_path(&schedule.path),
        Some(next),
        Some(schedule.path),
        None,
        false,
        false,
    )
    .await?;
    Ok(tx)
}

pub async fn create_schedule(
    mut tx: Transaction<'_, Postgres>,
    w_id: String,
    ns: NewSchedule,
    username: &str,
) -> Result<String> {
    cron::Schedule::from_str(&ns.schedule).map_err(|e| error::Error::BadRequest(e.to_string()))?;
    check_flow_conflict(&mut tx, &w_id, &ns.path, ns.is_flow, &ns.script_path).await?;

    let schedule = sqlx::query_as!(
        Schedule,
        "INSERT INTO schedule (workspace_id, path, schedule, offset_, edited_by, script_path, \
         is_flow, args, enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *",
        w_id,
        ns.path,
        ns.schedule,
        ns.offset,
        username,
        ns.script_path,
        ns.is_flow,
        ns.args,
        ns.enabled.unwrap_or(false),
    )
    .fetch_one(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("inserting schedule in {w_id}: {e}")))?;

    audit_log(
        &mut tx,
        username,
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

async fn check_flow_conflict<'c>(
    tx: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
    is_flow: bool,
    script_path: &str,
) -> error::Result<()> {
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
            return Err(error::Error::BadConfig(format!(
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
    pub script_path: String,
    pub is_flow: bool,
    pub args: Option<serde_json::Value>,
}

async fn clear_schedule<'c>(db: &mut Transaction<'c, Postgres>, path: &str) -> Result<()> {
    sqlx::query!(
        "DELETE FROM queue WHERE schedule_path = $1 AND running = false",
        path
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn edit_schedule(
    mut tx: Transaction<'_, Postgres>,
    w_id: String,
    path: StripPath,
    es: EditSchedule,
    username: &String,
) -> Result<String> {
    let path = path.to_path();

    cron::Schedule::from_str(&es.schedule).map_err(|e| error::Error::BadRequest(e.to_string()))?;

    check_flow_conflict(&mut tx, &w_id, &path, es.is_flow, &es.script_path).await?;

    clear_schedule(&mut tx, path).await?;
    let schedule = sqlx::query_as!(
        Schedule,
        "UPDATE schedule SET schedule = $1, script_path = $2, is_flow = $3, args = $4 WHERE path \
         = $5 AND workspace_id = $6 RETURNING *",
        es.schedule,
        es.script_path,
        es.is_flow,
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
        username,
        "schedule.edit",
        ActionKind::Update,
        &w_id,
        Some(&path.to_string()),
        Some(
            [
                Some(("schedule", es.schedule.as_str())),
                Some(("script_path", es.script_path.as_str())),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
    )
    .await?;
    tx.commit().await?;

    Ok(path.to_string())
}

pub async fn list_schedule(
    mut tx: Transaction<'_, Postgres>,
    w_id: String,
    pagination: Pagination,
) -> Result<Vec<Schedule>> {
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
    Ok(rows)
}

pub async fn get_schedule_opt<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
) -> Result<Option<Schedule>> {
    let schedule_opt = sqlx::query_as!(
        Schedule,
        "SELECT * FROM schedule WHERE path = $1 AND workspace_id = $2",
        path,
        w_id
    )
    .fetch_optional(db)
    .await?;
    Ok(schedule_opt)
}

pub async fn exists_schedule(
    tx: &mut Transaction<'_, Postgres>,
    w_id: String,
    path: StripPath,
) -> Result<bool> {
    let path = path.to_path();

    let exists = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM schedule WHERE path = $1 AND workspace_id = $2)",
        path,
        w_id
    )
    .fetch_one(tx)
    .await?
    .unwrap_or(false);

    Ok(exists)
}

#[derive(Deserialize)]
pub struct PreviewPayload {
    pub schedule: String,
    pub offset: Option<i32>,
}

pub fn preview_schedule(
    PreviewPayload { schedule, offset }: PreviewPayload,
) -> Result<Vec<DateTime<chrono::Utc>>> {
    let schedule =
        cron::Schedule::from_str(&schedule).map_err(|e| error::Error::BadRequest(e.to_string()))?;
    let upcoming: Vec<DateTime<chrono::Utc>> = schedule
        .upcoming(get_offset(offset))
        .take(10)
        .map(|x| x.into())
        .collect();
    Ok(upcoming)
}

fn get_offset(offset: Option<i32>) -> FixedOffset {
    FixedOffset::west(offset.unwrap_or(0) * 60)
}

#[derive(Deserialize)]
pub struct SetEnabled {
    pub enabled: bool,
}

pub async fn set_enabled(
    mut tx: Transaction<'_, Postgres>,
    w_id: String,
    path: StripPath,
    SetEnabled { enabled }: SetEnabled,
    username: &str,
) -> Result<String> {
    let path = path.to_path();
    let schedule_o = sqlx::query_as!(
        Schedule,
        "UPDATE schedule SET enabled = $1 WHERE path = $2 AND workspace_id = $3 RETURNING *",
        enabled,
        path,
        w_id
    )
    .fetch_optional(&mut tx)
    .await?;

    let schedule = not_found_if_none(schedule_o, "Schedule", path)?;

    clear_schedule(&mut tx, path).await?;

    if enabled {
        tx = push_scheduled_job(tx, schedule).await?;
    }
    audit_log(
        &mut tx,
        username,
        "schedule.setenabled",
        ActionKind::Update,
        &w_id,
        Some(path),
        Some([("enabled", enabled.to_string().as_ref())].into()),
    )
    .await?;
    tx.commit().await?;
    Ok(format!(
        "succesfully updated schedule at path {} to status {}",
        path, enabled
    ))
}

pub async fn delete_schedule(
    mut tx: Transaction<'_, Postgres>,
    w_id: String,
    path: StripPath,
    username: &str,
) -> Result<String> {
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
        username,
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

fn schedule_to_user(path: &str) -> String {
    format!("schedule-{}", path.replace('/', "-"))
}
