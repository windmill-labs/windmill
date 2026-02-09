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
use windmill_common::get_flow_version_info_from_version;
use windmill_common::get_latest_flow_version_id_for_path;
use windmill_common::jobs::check_tag_available_for_workspace_internal;
use windmill_common::jobs::JobPayload;
use windmill_common::jobs::JobTriggerKind;
use windmill_common::runnable_settings::ConcurrencySettings;
use windmill_common::runnable_settings::DebouncingSettings;
use windmill_common::schedule::schedule_to_user;
use windmill_common::scripts::ScriptHash;
use windmill_common::triggers::TriggerMetadata;
use windmill_common::utils::WarnAfterExt;
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
    Option<String>,     // tag
    Option<i32>,        // timeout
    Option<String>,     // on_behalf_of_email
    String,             // created_by
    Option<ScriptHash>, // hash (for scripts)
    Option<i64>,        // flow_version (for flows)
    Option<Retry>,      // retry
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

        let FlowVersionInfo { tag, on_behalf_of_email, edited_by, .. } =
            get_flow_version_info_from_version(
                &mut **tx,
                version,
                &schedule.workspace_id,
                &schedule.script_path,
            )
            .await?;

        Ok((
            tag,
            None,
            on_behalf_of_email,
            edited_by,
            None,
            Some(version),
            parsed_retry,
        ))
    } else {
        let (
            hash,
            tag,
            _custom_concurrency_key,
            _concurrent_limit,
            _concurrency_time_window_s,
            _debounce_key,
            _debounce_delay_s,
            _cache_ttl,
            _cache_ignore_s3_path,
            _language,
            _dedicated_worker,
            _priority,
            timeout,
            on_behalf_of_email,
            created_by,
            _runnable_settings_handle,
        ) = windmill_common::get_latest_hash_for_path(
            &mut **tx,
            &schedule.workspace_id,
            &schedule.script_path,
            false,
        )
        .await?;

        Ok((
            tag,
            timeout,
            on_behalf_of_email,
            created_by,
            Some(hash),
            None,
            parsed_retry,
        ))
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
                .warn_after_seconds_with_sql(1, "update_schedule_paused_until".to_string())
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
    .warn_after_seconds_with_sql(1, "already_exists_job".to_string())
    .await?
    .unwrap_or(false);

    if already_exists {
        tracing::warn!(
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
    let (payload, tag, timeout, on_behalf_of_email, created_by) = if let Some(handler_path) =
        &schedule.dynamic_skip
    {
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
                skip_handler: Some(windmill_common::jobs::SkipHandler {
                    path: handler_path.clone(),
                    args: skip_handler_args,
                    stop_condition,
                    stop_message,
                }),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                priority: None,
                tag_override: schedule.tag.clone(),
                trigger_path: None,
                apply_preprocessor: false,
                concurrency_settings: ConcurrencySettings::default(),
                debouncing_settings: DebouncingSettings::default(),
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
        .warn_after_seconds_with_sql(1, "get_latest_flow_version_id_for_path".to_string())
        .await?;

        let FlowVersionInfo {
            version, tag, dedicated_worker, on_behalf_of_email, edited_by, ..
        } = get_flow_version_info_from_version(
            &mut *tx,
            version,
            &schedule.workspace_id,
            &schedule.script_path,
        )
        .warn_after_seconds_with_sql(1, "get_flow_version_info_from_version".to_string())
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
            concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            debounce_key,
            debounce_delay_s,
            cache_ttl,
            cache_ignore_s3_path,
            language,
            dedicated_worker,
            priority,
            timeout,
            on_behalf_of_email,
            created_by,
            runnable_settings_handle,
        ) = windmill_common::get_latest_hash_for_path(
            &mut *tx,
            &schedule.workspace_id,
            &schedule.script_path,
            false,
        )
        .warn_after_seconds_with_sql(1, "get_latest_hash_for_path".to_string())
        .await?;

        let (debouncing_settings, concurrency_settings) =
            windmill_common::runnable_settings::prefetch_cached_from_handle(runnable_settings_handle, db)
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
                    skip_handler: None,
                    args: static_args,
                    cache_ttl,
                    cache_ignore_s3_path,
                    priority,
                    tag_override: schedule.tag.clone(),
                    trigger_path: None,
                    apply_preprocessor: false,
                    concurrency_settings: ConcurrencySettings::default(),
                    debouncing_settings: DebouncingSettings::default(),
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
                    cache_ttl,
                    cache_ignore_s3_path,
                    dedicated_worker,
                    language,
                    priority,
                    apply_preprocessor: false,
                    debouncing_settings: debouncing_settings
                        .maybe_fallback(debounce_key, debounce_delay_s),
                    concurrency_settings: concurrency_settings.maybe_fallback(
                        concurrency_key,
                        concurrent_limit,
                        concurrency_time_window_s,
                    ),
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
    .warn_after_seconds_with_sql(1, "clear_schedule_error".to_string())
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
                .warn_after_seconds_with_sql(1, "is_windmill_user".to_string())
                .await?;
        if is_windmill_user {
            sqlx::query!("SET LOCAL ROLE NONE")
                .execute(&mut *tx)
                .warn_after_seconds_with_sql(1, "set_local_role_none".to_string())
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

    let obo_authed;
    let push_authed = match push_authed {
        Some(a) => Some(a),
        None => {
            obo_authed = windmill_common::auth::fetch_authed_from_permissioned_as(
                &permissioned_as,
                email,
                &schedule.workspace_id,
                &mut *tx,
            )
            .await
            .ok();
            obo_authed.as_ref()
        }
    };

    if let Some(tag) = tag.as_deref().filter(|t| !t.is_empty()) {
        check_tag_available_for_workspace_internal(
            &db,
            &schedule.workspace_id,
            &tag,
            email,
            None, // no token for schedules so no scopes so no scope_tags
        )
        .warn_after_seconds_with_sql(1, "check_tag_available_for_workspace_internal".to_string())
        .await?;
    }

    tracing::info!(
        "Pushing next scheduled job for schedule {} at {} (schedule: {})",
        &schedule.path,
        next,
        &schedule.schedule
    );
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
        None,
        Some(TriggerMetadata::new(
            Some(schedule.path.clone()),
            JobTriggerKind::Schedule,
        )),
        None,
    )
    .warn_after_seconds_with_sql(1, "push in push_scheduled_job".to_string())
    .await?;

    if revert_to_windmill_user {
        sqlx::query!("SET LOCAL ROLE windmill_user")
            .execute(&mut *tx)
            .warn_after_seconds_with_sql(1, "set_local_role_windmill_user".to_string())
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

pub async fn clear_schedule<'c>(
    tx: &mut Transaction<'c, Postgres>,
    path: &str,
    w_id: &str,
) -> Result<()> {
    tracing::info!("Clearing schedule {}", path);
    sqlx::query!(
        "WITH to_delete AS (
            SELECT id FROM v2_job_queue
                JOIN v2_job j USING (id)
            WHERE trigger_kind = 'schedule'
                AND trigger = $1
                AND j.workspace_id = $2
                AND flow_step_id IS NULL
                AND running = false
            FOR UPDATE
        ), deleted AS (
            DELETE FROM v2_job_queue
            WHERE id IN (SELECT id FROM to_delete)
            RETURNING id
        ) DELETE FROM v2_job WHERE id IN (SELECT id FROM deleted)",
        path,
        w_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
