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
use std::collections::HashMap;
use std::str::FromStr;
use windmill_common::db::Authed;
use windmill_common::flows::Retry;
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
    schedule: &Schedule,
    authed: Option<&Authed>,
) -> Result<QueueTransaction<'c, R>> {
    let sched = cron::Schedule::from_str(schedule.schedule.as_ref())
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
        tracing::info!(
            "Job for schedule {} at {} already exists",
            &schedule.path,
            next
        );
        return Ok(tx);
    }

    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();

    if let Some(args_v) = &schedule.args {
        if let Ok(args_m) =
            serde_json::from_str::<HashMap<String, Box<serde_json::value::RawValue>>>(args_v.get())
        {
            args = args_m.clone()
        } else {
            return Err(error::Error::ExecutionErr(
                "args of scripts needs to be dict".to_string(),
            ));
        }
    }

    let (payload, tag, timeout) = if schedule.is_flow {
        let r = sqlx::query!(
            "SELECT tag, dedicated_worker from flow WHERE path = $1 and workspace_id = $2",
            &schedule.script_path,
            &schedule.workspace_id,
        )
        .fetch_optional(&mut tx)
        .await?;
        let (tag, dedicated_worker) = r
            .map(|x| (x.tag, x.dedicated_worker))
            .unwrap_or_else(|| (None, None));
        (
            JobPayload::Flow { path: schedule.script_path.clone(), dedicated_worker },
            tag,
            None,
        )
    } else {
        let (
            hash,
            tag,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
            timeout,
        ) = windmill_common::get_latest_hash_for_path(
            tx.transaction_mut(),
            &schedule.workspace_id,
            &schedule.script_path,
        )
        .await?;

        if schedule.retry.is_some() {
            let parsed_retry = serde_json::from_value::<Retry>(schedule.retry.clone().unwrap())
                .map_err(|err| {
                    error::Error::InternalErr(format!(
                        "Unable to parse retry information from schedule: {}",
                        err.to_string(),
                    ))
                })?;
            let mut static_args = HashMap::<String, Box<serde_json::value::RawValue>>::new();
            for (arg_name, arg_value) in args.clone() {
                static_args.insert(arg_name, arg_value);
            }
            // if retry is set, we wrap the script into a one step flow with a retry on the module
            (
                JobPayload::SingleScriptFlow {
                    path: schedule.script_path.clone(),
                    hash: hash,
                    retry: parsed_retry,
                    args: static_args,
                    custom_concurrency_key,
                    concurrent_limit: concurrent_limit,
                    concurrency_time_window_s: concurrency_time_window_s,
                    cache_ttl: cache_ttl,
                    priority: priority,
                    tag_override: schedule.tag.clone(),
                },
                Some("flow".to_string()),
                timeout,
            )
        } else {
            (
                JobPayload::ScriptHash {
                    hash,
                    path: schedule.script_path.clone(),
                    custom_concurrency_key,
                    concurrent_limit: concurrent_limit,
                    concurrency_time_window_s: concurrency_time_window_s,
                    cache_ttl: cache_ttl,
                    dedicated_worker,
                    language,
                    priority,
                },
                if schedule.tag.as_ref().is_some_and(|x| x != "") {
                    schedule.tag.clone()
                } else {
                    tag
                },
                timeout,
            )
        }
    };

    if let Err(e) = sqlx::query!(
        "UPDATE schedule SET error = NULL WHERE workspace_id = $1 AND path = $2",
        &schedule.workspace_id,
        &schedule.path
    )
    .execute(&mut tx)
    .await
    {
        tracing::error!(
            "Failed to clear error for schedule {}: {}",
            &schedule.path,
            e
        );
    };

    let tx = PushIsolationLevel::Transaction(tx);
    let (_, tx) = push(
        &db,
        tx,
        &schedule.workspace_id,
        payload,
        crate::PushArgs { args, extra: HashMap::new() },
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
        timeout,
        None,
        None,
        authed,
    )
    .await?;
    Ok(tx) // TODO: Bubble up pushed UUID from here
}

pub async fn get_schedule_opt<'c>(
    db: &mut Transaction<'c, Postgres>,
    w_id: &str,
    path: &str,
) -> Result<Option<Schedule>> {
    let schedule_opt = sqlx::query_as::<_, Schedule>(
        "SELECT * FROM schedule WHERE path = $1 AND workspace_id = $2",
    )
    .bind(path)
    .bind(w_id)
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
