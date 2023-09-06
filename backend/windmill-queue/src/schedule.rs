/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::push;
use crate::PushIsolationLevel;
use crate::QueueTransaction;
use sqlx::{query_scalar, Postgres, Transaction};
use std::str::FromStr;
use windmill_common::jobs::JobPayload;
use windmill_common::schedule::schedule_to_user;
use windmill_common::DB;
use windmill_common::{
    error::{self, Result},
    schedule::Schedule,
    users::username_to_permissioned_as,
    utils::{now_from_db, StripPath},
};

pub async fn push_scheduled_job<'c, R: rsmq_async::RsmqConnection + Send + 'c>(
    db: &DB,
    mut tx: QueueTransaction<'c, R>,
    schedule: Schedule,
) -> Result<QueueTransaction<'c, R>> {
    let sched = cron::Schedule::from_str(&schedule.schedule)
        .map_err(|e| error::Error::BadRequest(e.to_string()))?;

    let tz = chrono_tz::Tz::from_str(&schedule.timezone)
        .map_err(|e| error::Error::BadRequest(e.to_string()))?;

    let now = now_from_db(&mut tx).await?.with_timezone(&tz);

    let next = sched
        .after(&now)
        .next()
        .expect("a schedule should have a next event");

    // println!("next event ({:?}): {}", tz, next);
    // println!("next event(UTC): {}", next.with_timezone(&chrono::Utc));

    // Scheduled events must be stored in the database in UTC
    let next = next.with_timezone(&chrono::Utc);

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

    let (payload, tag) = if schedule.is_flow {
        (JobPayload::Flow(schedule.script_path), None)
    } else {
        let (
            hash,
            tag,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
        ) = windmill_common::get_latest_hash_for_path(
            tx.transaction_mut(),
            &schedule.workspace_id,
            &schedule.script_path,
        )
        .await?;
        (
            JobPayload::ScriptHash {
                hash,
                path: schedule.script_path,
                concurrent_limit: concurrent_limit,
                concurrency_time_window_s: concurrency_time_window_s,
                cache_ttl: cache_ttl,
                dedicated_worker,
                language,
            },
            tag,
        )
    };

    sqlx::query!(
        "UPDATE schedule SET error = NULL WHERE workspace_id = $1 AND path = $2",
        &schedule.workspace_id,
        &schedule.path
    )
    .execute(&mut tx)
    .await?;
    let tx = PushIsolationLevel::Transaction(tx);
    let (_, tx) = push(
        &db,
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
        None,
        None,
        false,
        false,
        None,
        true,
        tag,
        None,
        None,
    )
    .await?;
    Ok(tx) // TODO: Bubble up pushed UUID from here
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
    .fetch_optional(&mut **db)
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
    .fetch_one(&mut **tx)
    .await?
    .unwrap_or(false);

    Ok(exists)
}
