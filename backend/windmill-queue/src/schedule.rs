/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::str::FromStr;

use chrono::Duration;
use sqlx::{query_scalar, Postgres, Transaction};
use windmill_common::{
    error::{self, Result},
    schedule::Schedule,
    users::username_to_permissioned_as,
    utils::{now_from_db, StripPath},
};

use crate::{push, JobPayload};

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

    let (_, mut tx) = push(
        tx,
        &schedule.workspace_id,
        payload,
        args,
        &schedule_to_user(&schedule.path),
        &schedule.email,
        username_to_permissioned_as(&schedule.edited_by),
        Some(next),
        Some(schedule.path.clone()),
        None,
        false,
        false,
        None,
        true,
    )
    .await?;
    sqlx::query!(
        "UPDATE schedule SET error = NULL WHERE workspace_id = $1 AND path = $2",
        &schedule.workspace_id,
        &schedule.path
    )
    .execute(&mut tx)
    .await?;
    Ok(tx)
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

fn schedule_to_user(path: &str) -> String {
    format!("schedule-{}", path.replace('/', "-"))
}
