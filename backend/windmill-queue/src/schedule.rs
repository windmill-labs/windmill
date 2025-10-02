/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::push;
use crate::PushIsolationLevel;
use anyhow::Context;
use chrono::DateTime;
use chrono::Utc;
use sqlx::{PgExecutor, Postgres, Transaction};
use std::collections::HashMap;
use std::str::FromStr;
use windmill_common::db::Authed;
use windmill_common::ee_oss::LICENSE_KEY_VALID;
use windmill_common::flows::Retry;
use windmill_common::get_latest_flow_version_id_for_path;
use windmill_common::get_latest_flow_version_info_for_path_from_version;
use windmill_common::jobs::check_tag_available_for_workspace_internal;
use windmill_common::jobs::JobPayload;
use windmill_common::schedule::schedule_to_user;
use windmill_common::scripts::ScriptHash;
use windmill_common::worker::to_raw_value;
use windmill_common::FlowVersionInfo;
use windmill_common::DB;
use windmill_common::{
    error::{self, Result},
    schedule::Schedule,
    users::username_to_permissioned_as,
    utils::{now_from_db, ScheduleType, StripPath},
};

/// Helper to fetch metadata for a schedule's script or flow
async fn get_schedule_metadata<'c>(
    tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    schedule: &Schedule,
) -> Result<(
    Option<String>,                    // tag
    Option<i32>,                       // timeout
    Option<String>,                    // on_behalf_of_email
    String,                            // created_by
    Option<ScriptHash>,                // hash (for scripts)
    Option<i64>,                       // flow_version (for flows)
    Option<Retry>,                     // retry
)> {
    let parsed_retry = schedule
        .retry
        .clone()
        .and_then(|r| serde_json::from_value::<Retry>(r).ok());

    if schedule.is_flow {
        let version = get_latest_flow_version_id_for_path(
            None,
            &mut **tx,
            &schedule.workspace_id,
            &schedule.script_path,
            false,
        )
        .await?;

        let FlowVersionInfo {
            tag,
            on_behalf_of_email,
            edited_by,
            ..
        } = get_latest_flow_version_info_for_path_from_version(
            &mut **tx,
            version,
            &schedule.workspace_id,
            &schedule.script_path,
        )
        .await?;

        Ok((tag, None, on_behalf_of_email, edited_by, None, Some(version), parsed_retry))
    } else {
        let (
            hash,
            tag,
            _custom_concurrency_key,
            _concurrent_limit,
            _concurrency_time_window_s,
            _cache_ttl,
            _language,
            _dedicated_worker,
            _priority,
            timeout,
            on_behalf_of_email,
            created_by,
        ) = windmill_common::get_latest_hash_for_path(
            &mut **tx,
            &schedule.workspace_id,
            &schedule.script_path,
            false,
        )
        .await?;

        Ok((tag, timeout, on_behalf_of_email, created_by, Some(hash), None, parsed_retry))
    }
}

pub async fn push_scheduled_job<'c>(
    db: &DB,
    mut tx: Transaction<'c, Postgres>,
    schedule: &Schedule,
    authed: Option<&Authed>,
    now_cutoff: Option<DateTime<Utc>>,
) -> Result<Transaction<'c, Postgres>> {
    if !*LICENSE_KEY_VALID.read().await {
        return Err(error::Error::BadRequest(
            "License key is not valid. Go to your superadmin settings to update your license key."
                .to_string(),
        ));
    }

    let sched =
        ScheduleType::from_str(&schedule.schedule, schedule.cron_version.as_deref(), false)?;

    let tz = chrono_tz::Tz::from_str(&schedule.timezone)
        .map_err(|e| error::Error::BadRequest(e.to_string()))?;

    let now = now_from_db(&mut *tx).await?;

    let now = match now_cutoff {
        Some(now_cutoff) if now_cutoff >= now => {
            tracing::error!(
                "now_cutoff ({:?}) is after now ({:?}) for schedule {}. Using now_cutoff + 1s. This likely means the pg clock was shifted backwards.",
                now_cutoff,
                now,
                &schedule.path
            );
            now_cutoff + chrono::Duration::seconds(1)
        }
        _ => now,
    };

    let starting_from = match schedule.paused_until {
        Some(paused_until) if paused_until > now => paused_until.with_timezone(&tz),
        paused_until_o => {
            if paused_until_o.is_some() {
                sqlx::query!(
                    "UPDATE schedule SET paused_until = NULL WHERE workspace_id = $1 AND path = $2",
                    &schedule.workspace_id,
                    &schedule.path
                )
                .execute(&mut *tx)
                .await
                .context("Failed to clear paused_until for schedule")?;
            }
            now.with_timezone(&tz)
        }
    };

    let next = sched.find_next(&starting_from);
    // println!("next event ({:?}): {}", tz, next);
    // println!("next event(UTC): {}", next.with_timezone(&chrono::Utc));

    // Scheduled events must be stored in the database in UTC
    let next = next.with_timezone(&chrono::Utc);
    // panic!("next: {}", next);
    let already_exists: bool = sqlx::query_scalar!(
        // Query plan:
        // - use of the `ix_v2_job_root_by_path` index; hence the `parent_job IS NULL` clause.
        // - select from `v2_job` first, then join with `v2_job_queue` to avoid a full table scan
        //   on `scheduled_for = $3`.
        "SELECT EXISTS (
            SELECT 1 FROM v2_job j JOIN v2_job_queue USING (id)
            WHERE j.workspace_id = $1 AND trigger_kind = 'schedule' AND trigger = $2 AND runnable_path = $4
                AND parent_job IS NULL
                AND scheduled_for = $3
        )",
        &schedule.workspace_id,
        &schedule.path,
        next,
        &schedule.script_path
    )
    .fetch_one(&mut *tx)
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

    // If schedule handler is defined, wrap the scheduled job in a synthetic flow
    // with the handler as the first step (with stop_after_if to skip if handler returns false)
    let (payload, tag, timeout, on_behalf_of_email, created_by) = if let Some(handler_path) = &schedule.dynamic_skip {
        // Build skip handler args
        let mut skip_handler_args = HashMap::<String, Box<serde_json::value::RawValue>>::new();
        skip_handler_args.insert(
            "scheduled_for".to_string(),
            to_raw_value(&next.to_rfc3339()),
        );

        let stop_condition = "result !== true".to_string();
        let stop_message = format!(
            "Schedule handler {} did not return true for datetime {}. Handler must return boolean true to execute scheduled job.",
            handler_path,
            next.to_rfc3339()
        );

        // Get metadata from the scheduled script/flow for tag, timeout, etc.
        let (tag, timeout, on_behalf_of_email, created_by, hash, flow_version, retry) =
            get_schedule_metadata(&mut tx, schedule).await?;

        (
            JobPayload::SingleStepFlow {
                path: schedule.script_path.clone(),
                hash,
                flow_version,
                args: args.clone(),
                retry,
                error_handler_path: None,
                error_handler_args: None,
                skip_handler_path: Some(handler_path.clone()),
                skip_handler_args: Some(skip_handler_args),
                skip_handler_stop_condition: Some(stop_condition),
                skip_handler_stop_message: Some(stop_message),
                custom_concurrency_key: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                priority: None,
                tag_override: schedule.tag.clone(),
                trigger_path: None,
                apply_preprocessor: false,
            },
            if schedule.tag.as_ref().is_some_and(|x| x != "") {
                schedule.tag.clone()
            } else {
                tag
            },
            timeout,
            on_behalf_of_email,
            created_by,
        )
    } else if schedule.is_flow {
        let version = get_latest_flow_version_id_for_path(
            None,
            &mut *tx,
            &schedule.workspace_id,
            &schedule.script_path,
            false,
        )
        .await?;

        let FlowVersionInfo {
            version, tag, dedicated_worker, on_behalf_of_email, edited_by, ..
        } = get_latest_flow_version_info_for_path_from_version(
            &mut *tx,
            version,
            &schedule.workspace_id,
            &schedule.script_path,
        )
        .await?;

        (
            JobPayload::Flow {
                path: schedule.script_path.clone(),
                dedicated_worker,
                apply_preprocessor: false,
                version,
            },
            tag,
            None,
            on_behalf_of_email,
            edited_by,
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
            on_behalf_of_email,
            created_by,
        ) = windmill_common::get_latest_hash_for_path(
            &mut *tx,
            &schedule.workspace_id,
            &schedule.script_path,
            false,
        )
        .await?;

        if schedule.retry.is_some() {
            let parsed_retry = serde_json::from_value::<Retry>(schedule.retry.clone().unwrap())
                .map_err(|err| {
                    error::Error::internal_err(format!(
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
                JobPayload::SingleStepFlow {
                    path: schedule.script_path.clone(),
                    hash: Some(hash),
                    flow_version: None,
                    retry: Some(parsed_retry),
                    error_handler_path: None,
                    error_handler_args: None,
                    skip_handler_path: None,
                    skip_handler_args: None,
                    skip_handler_stop_condition: None,
                    skip_handler_stop_message: None,
                    args: static_args,
                    custom_concurrency_key: None,
                    concurrent_limit: None,
                    concurrency_time_window_s: None,
                    cache_ttl: cache_ttl,
                    priority: priority,
                    tag_override: schedule.tag.clone(),
                    trigger_path: None,
                    apply_preprocessor: false,
                },
                if schedule.tag.as_ref().is_some_and(|x| x != "") {
                    schedule.tag.clone()
                } else {
                    tag
                },
                timeout,
                on_behalf_of_email,
                created_by,
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
                    apply_preprocessor: false,
                },
                if schedule.tag.as_ref().is_some_and(|x| x != "") {
                    schedule.tag.clone()
                } else {
                    tag
                },
                timeout,
                on_behalf_of_email,
                created_by,
            )
        }
    };

    if let Err(e) = sqlx::query!(
        "UPDATE schedule SET error = NULL WHERE workspace_id = $1 AND path = $2",
        &schedule.workspace_id,
        &schedule.path
    )
    .execute(&mut *tx)
    .await
    {
        tracing::error!(
            "Failed to clear error for schedule {}: {}",
            &schedule.path,
            e
        );
    };

    let (email, permissioned_as, push_authed, revert_to_windmill_user) = if let Some(email) =
        on_behalf_of_email.as_ref()
    {
        let is_windmill_user =
            sqlx::query_scalar!("SELECT CURRENT_USER = 'windmill_user' as \"is_windmill_user!\"")
                .fetch_one(&mut *tx)
                .await?;
        if is_windmill_user {
            sqlx::query!("SET LOCAL ROLE NONE")
                .execute(&mut *tx)
                .await?;
        }
        (
            email,
            username_to_permissioned_as(&created_by),
            None,
            is_windmill_user,
        )
    } else {
        (
            &schedule.email,
            username_to_permissioned_as(&schedule.edited_by),
            authed,
            false,
        )
    };

    if let Some(tag) = tag.as_deref().filter(|t| !t.is_empty()) {
        check_tag_available_for_workspace_internal(
            &db,
            &schedule.workspace_id,
            &tag,
            email,
            None, // no token for schedules so no scopes so no scope_tags
        )
        .await?;
    }

    let tx = PushIsolationLevel::Transaction(tx);
    let (_, mut tx) = push(
        &db,
        tx,
        &schedule.workspace_id,
        payload,
        crate::PushArgs { args: &args, extra: None },
        &schedule_to_user(&schedule.path),
        email,
        permissioned_as,
        Some(&schedule.path),
        Some(next),
        Some(schedule.path.clone()),
        None,
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
        push_authed,
        false,
    )
    .await?;

    if revert_to_windmill_user {
        sqlx::query!("SET LOCAL ROLE windmill_user")
            .execute(&mut *tx)
            .await?;
    }

    Ok(tx) // TODO: Bubble up pushed UUID from here
}

pub async fn get_schedule_opt<'c>(
    e: impl PgExecutor<'c>,
    w_id: &str,
    path: &str,
) -> Result<Option<Schedule>> {
    let schedule_opt = sqlx::query_as::<_, Schedule>(
        "SELECT * FROM schedule WHERE path = $1 AND workspace_id = $2",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(e)
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
