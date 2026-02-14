/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Job execution primitives: push, cancel, tag/license checks, wait-for-result.

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use http::{HeaderMap, HeaderName, HeaderValue};
use hyper::StatusCode;
use serde::Deserialize;
use serde_json::value::RawValue;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use windmill_api_auth::{check_scopes, get_scope_tags, ApiAuthed};
use windmill_common::{
    db::{UserDB, UserDbWithAuthed},
    error::{self, Error},
    flow_conversations::{add_message_to_conversation_tx, MessageType},
    get_latest_flow_version_info_for_path,
    jobs::{
        check_tag_available_for_workspace_internal, format_result, script_path_to_payload,
        JobPayload,
    },
    triggers::TriggerMetadata,
    users::username_to_permissioned_as,
    utils::StripPath,
    FlowVersionInfo, DB,
};
use windmill_queue::{
    cancel_job, get_result_and_success_by_id_from_flow, push, PushArgs, PushArgsOwned,
    PushIsolationLevel,
};

use crate::types::RunJobQuery;

// ---------------------------------------------------------------------------
// Tag / license checks
// ---------------------------------------------------------------------------

pub async fn check_tag_available_for_workspace(
    db: &DB,
    w_id: &str,
    tag: &Option<String>,
    authed: &ApiAuthed,
) -> error::Result<()> {
    if let Some(tag) = tag.as_deref().filter(|t| !t.is_empty()) {
        let tags = get_scope_tags(authed);
        check_tag_available_for_workspace_internal(&db, w_id, tag, &authed.email, tags).await
    } else {
        Ok(())
    }
}

#[cfg(feature = "enterprise")]
pub async fn check_license_key_valid() -> error::Result<()> {
    use windmill_common::ee_oss::LICENSE_KEY_VALID;

    let valid = *LICENSE_KEY_VALID.read().await;
    if !valid {
        return Err(error::Error::BadRequest(
            "License key is not valid. Go to your superadmin settings to update your license key."
                .to_string(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Cancel jobs
// ---------------------------------------------------------------------------

pub async fn cancel_jobs(
    jobs: Vec<Uuid>,
    db: &DB,
    username: &str,
    w_id: &str,
    force_cancel: bool,
) -> error::JsonResult<Vec<Uuid>> {
    let mut uuids = vec![];
    tracing::info!("Cancelling jobs: {:?}", jobs);
    let mut tx = db.begin().await?;
    let trivial_jobs =  sqlx::query!("INSERT INTO v2_job_completed AS cj
                   ( workspace_id
                   , id
                   , duration_ms
                   , result
                   , canceled_by
                   , canceled_reason
                   , flow_status
                   , status
                   , worker
                )
                SELECT  q.workspace_id
                   , q.id
                   , 0
                   , $4
                   , $1
                   , 'cancel all'
                   , (SELECT flow_status FROM v2_job_status WHERE id = q.id)
                   , 'canceled'::job_status
                   , worker
        FROM v2_job_queue q
            JOIN v2_job USING (id)
        WHERE q.id = any($2) AND running = false AND parent_job IS NULL AND q.workspace_id = $3 AND trigger_kind IS DISTINCT FROM 'schedule'
            FOR UPDATE SKIP LOCKED
        ON CONFLICT (id) DO NOTHING RETURNING id AS \"id!\"", username, &jobs, w_id, serde_json::json!({"error": { "message": format!("Job canceled: cancel all by {username}"), "name": "Canceled", "reason": "cancel all", "canceler": username}}))
        .fetch_all(&mut *tx)
        .await?.into_iter().map(|x| x.id).collect::<Vec<Uuid>>();

    sqlx::query!(
        "DELETE FROM v2_job_queue WHERE id = any($1) AND workspace_id = $2",
        &trivial_jobs,
        w_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    for job_id in jobs.into_iter() {
        if trivial_jobs.contains(&job_id) {
            continue;
        }
        match tokio::time::timeout(tokio::time::Duration::from_secs(5), async move {
            let tx = db.begin().await?;
            let (tx, _) = cancel_job(
                username,
                None,
                job_id.clone(),
                w_id,
                tx,
                db,
                force_cancel,
                false,
            )
            .await?;
            tx.commit().await?;
            Ok::<_, anyhow::Error>(())
        })
        .await
        {
            Ok(result) => match result {
                Ok(_) => {
                    uuids.push(job_id);
                }
                Err(e) => {
                    tracing::error!("Failed to cancel job {:?}: {:?}", job_id, e);
                }
            },
            Err(_) => {
                tracing::error!(
                    "Timeout while trying to cancel job {:?} after 5 seconds",
                    job_id
                );
            }
        }
    }

    uuids.extend(trivial_jobs);

    Ok(Json(uuids))
}

// ---------------------------------------------------------------------------
// Wait-for-result infrastructure
// ---------------------------------------------------------------------------

struct Guard {
    done: bool,
    id: Uuid,
    w_id: String,
    db: DB,
    username: String,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if !&self.done {
            let id = self.id;
            let w_id = self.w_id.clone();
            let db = self.db.clone();
            let username = self.username.clone();

            tracing::info!("http connection broke, marking job {id} as canceled");
            tokio::spawn(async move {
                let cancel_f = async {
                    let tx = db.begin().await?;
                    let (tx, _) = cancel_job(
                        &username,
                        Some("http connection broke".to_string()),
                        id,
                        &w_id,
                        tx,
                        &db,
                        false,
                        false,
                    )
                    .await?;
                    tx.commit().await?;
                    Ok::<_, anyhow::Error>(())
                };

                if let Err(e) = cancel_f.await {
                    tracing::error!(
                        "Error marking job as canceled after http connection broke: {e}"
                    );
                }
            });
        }
    }
}

lazy_static::lazy_static! {
    pub static ref TIMEOUT_WAIT_RESULT: Arc<RwLock<Option<u64>>> = Arc::new(RwLock::new(
        std::env::var("TIMEOUT_WAIT_RESULT")
            .ok()
            .and_then(|x| x.parse::<u64>().ok())
    ));

    pub static ref QUEUE_LIMIT_WAIT_RESULT: Option<i64> = std::env::var("QUEUE_LIMIT_WAIT_RESULT")
        .ok()
        .and_then(|x| x.parse().ok());

    pub static ref WAIT_RESULT_FAST_POLL_INTERVAL_MS: u64 = std::env::var("WAIT_RESULT_FAST_POLL_INTERVAL_MS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(50);

    pub static ref WAIT_RESULT_FAST_POLL_DURATION_SECS: u16 = std::env::var("WAIT_RESULT_FAST_POLL_DURATION_SECS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(2);

    pub static ref WAIT_RESULT_SLOW_POLL_INTERVAL_MS: u64 = std::env::var("WAIT_RESULT_SLOW_POLL_INTERVAL_MS")
        .ok()
        .and_then(|x| x.parse().ok())
        .unwrap_or(200);
}

#[derive(Deserialize)]
pub struct WindmillCompositeResult {
    windmill_status_code: Option<u16>,
    windmill_content_type: Option<String>,
    windmill_headers: Option<HashMap<String, String>>,
    result: Option<Box<RawValue>>,
}

pub async fn run_wait_result_internal(
    db: &DB,
    uuid: Uuid,
    w_id: &str,
    node_id_for_empty_return: Option<String>,
    username: &str,
) -> error::Result<(Box<RawValue>, bool)> {
    let mut result = None;
    let mut success = false;
    let timeout = TIMEOUT_WAIT_RESULT.read().await.clone().unwrap_or(600);
    let timeout_ms = if timeout <= 0 {
        2000
    } else {
        (timeout * 1000) as u64
    };

    let mut g = Guard {
        done: false,
        id: uuid,
        w_id: w_id.to_string(),
        db: db.clone(),
        username: username.to_string(),
    };

    let fast_poll_duration = *WAIT_RESULT_FAST_POLL_DURATION_SECS as u64 * 1000;
    let mut accumulated_delay = 0 as u64;

    loop {
        if let Some(node_id_for_empty_return) = node_id_for_empty_return.as_ref() {
            let result_and_success = get_result_and_success_by_id_from_flow(
                &db,
                w_id,
                &uuid,
                node_id_for_empty_return,
                None,
            )
            .await
            .ok();
            if let Some((r, s)) = result_and_success {
                result = Some(r);
                success = s;
            }
        }

        if result.is_none() {
            let row = sqlx::query!(
                "
                    SELECT
                        result AS \"result: sqlx::types::Json<Box<RawValue>>\",
                        result_columns,
                        status = 'success' AS \"success!\"
                    FROM
                        v2_job_completed
                    WHERE
                        id = $1 AND
                        workspace_id = $2
                    ",
                uuid,
                &w_id
            )
            .fetch_optional(db)
            .await?;
            if let Some(mut raw_result) = row {
                format_result(
                    raw_result.result_columns.as_ref(),
                    raw_result.result.as_mut(),
                );
                result = raw_result.result.map(|x| x.0);
                success = raw_result.success;
            }
        }

        if result.is_some() {
            break;
        }

        let delay = if accumulated_delay <= fast_poll_duration {
            *WAIT_RESULT_FAST_POLL_INTERVAL_MS
        } else {
            *WAIT_RESULT_SLOW_POLL_INTERVAL_MS
        };
        accumulated_delay += delay;
        if accumulated_delay > timeout_ms {
            break;
        };
        tokio::time::sleep(core::time::Duration::from_millis(delay)).await;
    }

    if let Some(result) = result {
        g.done = true;
        Ok((result, success))
    } else {
        Err(Error::ExecutionErr(format!("timeout after {}s", timeout)))
    }
}

pub fn result_to_response(result: Box<RawValue>, success: bool) -> error::Result<Response> {
    let composite_result = serde_json::from_str::<WindmillCompositeResult>(result.get());
    match composite_result {
        Ok(WindmillCompositeResult {
            windmill_status_code,
            windmill_content_type,
            windmill_headers,
            result: result_value,
        }) => {
            if windmill_content_type.is_none()
                && windmill_status_code.is_none()
                && windmill_headers.is_none()
            {
                return Ok((
                    if success {
                        StatusCode::OK
                    } else {
                        StatusCode::UNPROCESSABLE_ENTITY
                    },
                    Json(result),
                )
                    .into_response());
            }

            let status_code_or_default = windmill_status_code
                .map(|val| match StatusCode::from_u16(val) {
                    Ok(sc) => Ok(sc),
                    Err(_) => Err(Error::ExecutionErr("Invalid status code".to_string())),
                })
                .unwrap_or_else(|| {
                    if !success {
                        Ok(StatusCode::UNPROCESSABLE_ENTITY)
                    } else if result_value.is_some() {
                        Ok(StatusCode::OK)
                    } else {
                        Ok(StatusCode::NO_CONTENT)
                    }
                })?;

            let mut headers = HeaderMap::new();

            if let Some(windmill_headers) = windmill_headers {
                for (k, v) in windmill_headers {
                    let k = HeaderName::from_str(k.as_str()).map_err(|err| {
                        Error::internal_err(format!("Invalid header name {k}: {err}"))
                    })?;
                    let v = HeaderValue::from_str(v.as_str()).map_err(|err| {
                        Error::internal_err(format!("Invalid header value {v}: {err}"))
                    })?;
                    headers.insert(k, v);
                }
            }

            if let Some(content_type) = windmill_content_type {
                let serialized_json_result = result_value
                    .map(|val| val.get().to_owned())
                    .unwrap_or_else(String::new);
                let serialized_result =
                    serde_json::from_str::<String>(serialized_json_result.as_str())
                        .ok()
                        .unwrap_or(serialized_json_result);
                headers.insert(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_str(content_type.as_str()).map_err(|err| {
                        Error::internal_err(format!("Invalid content type {content_type}: {err}"))
                    })?,
                );
                return Ok((status_code_or_default, headers, serialized_result).into_response());
            }
            if let Some(result_value) = result_value {
                return Ok((status_code_or_default, headers, Json(result_value)).into_response());
            } else {
                Ok((status_code_or_default, headers).into_response())
            }
        }
        _ => Ok((
            if success {
                StatusCode::OK
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            },
            Json(result),
        )
            .into_response()),
    }
}

pub async fn run_wait_result(
    db: &DB,
    uuid: Uuid,
    w_id: &str,
    node_id_for_empty_return: Option<String>,
    username: &str,
) -> error::Result<Response> {
    let (result, success) =
        run_wait_result_internal(db, uuid, w_id, node_id_for_empty_return, username).await?;

    result_to_response(result, success)
}

// ---------------------------------------------------------------------------
// Delete job metadata
// ---------------------------------------------------------------------------

pub async fn delete_job_metadata_after_use(db: &DB, job_uuid: Uuid) -> Result<(), Error> {
    sqlx::query!(
        "UPDATE v2_job SET args = '{}'::jsonb WHERE id = $1",
        job_uuid,
    )
    .execute(db)
    .await?;
    sqlx::query!(
        "UPDATE v2_job_completed SET result = '{}'::jsonb WHERE id = $1",
        job_uuid,
    )
    .execute(db)
    .await?;
    sqlx::query!(
        "UPDATE job_logs SET logs = '##DELETED##' WHERE job_id = $1",
        job_uuid,
    )
    .execute(db)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Queue limit check
// ---------------------------------------------------------------------------

pub async fn check_queue_too_long(db: &DB, queue_limit: Option<i64>) -> error::Result<()> {
    if let Some(limit) = queue_limit {
        let count = sqlx::query_scalar!(
                "SELECT COUNT(*) FROM v2_job_queue q WHERE q.canceled_by IS NULL AND (q.scheduled_for <= now()
            OR (q.suspend_until IS NOT NULL
                AND (q.suspend <= 0 OR q.suspend_until <= now())))",
            )
            .fetch_one(db)
            .await?
            .unwrap_or(0);

        if count > queue_limit.unwrap() {
            return Err(Error::internal_err(format!(
                "Number of queued job is too high: {count} > {limit}"
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Flow run helpers
// ---------------------------------------------------------------------------

pub async fn set_flow_memory_id(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    job_id: Uuid,
    memory_id: Uuid,
) -> error::Result<()> {
    sqlx::query!(
        "UPDATE v2_job_status
         SET flow_status = jsonb_set(
             flow_status,
             '{memory_id}',
             to_jsonb($2::uuid)
         )
         WHERE id = $1",
        job_id,
        memory_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Apply flow-specific query parameters after job creation
pub async fn process_flow_run_query_params(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    job_id: Uuid,
    run_query: &RunJobQuery,
) -> error::Result<()> {
    if let Some(memory_id) = run_query.memory_id {
        set_flow_memory_id(tx, job_id, memory_id).await?;
    }
    Ok(())
}

pub async fn handle_chat_conversation_messages(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authed: &ApiAuthed,
    w_id: &str,
    flow_path: &str,
    run_query: &RunJobQuery,
    user_message_raw: Option<&Box<serde_json::value::RawValue>>,
) -> error::Result<()> {
    let memory_id = run_query.memory_id.ok_or_else(|| {
        windmill_common::error::Error::BadRequest(
            "memory_id is required for chat-enabled flows".to_string(),
        )
    })?;

    let user_message_raw = user_message_raw.ok_or_else(|| {
        windmill_common::error::Error::BadRequest(
            "user_message argument is required for chat-enabled flows".to_string(),
        )
    })?;

    let user_message: String = serde_json::from_str(user_message_raw.get()).map_err(|e| {
        windmill_common::error::Error::BadRequest(format!(
            "Failed to deserialize user_message: {}",
            e
        ))
    })?;

    windmill_common::flow_conversations::get_or_create_conversation_with_id(
        tx,
        w_id,
        flow_path,
        &authed.username,
        &user_message,
        memory_id,
    )
    .await?;

    add_message_to_conversation_tx(
        tx,
        memory_id,
        None,
        &user_message,
        MessageType::User,
        None,
        true,
    )
    .await?;

    Ok(())
}

pub async fn run_flow<'c>(
    authed: &ApiAuthed,
    db: &DB,
    tx_o: Option<sqlx::Transaction<'c, sqlx::Postgres>>,
    user_db: UserDB,
    w_id: &str,
    flow_path: &str,
    flow_version_info: FlowVersionInfo,
    run_query: RunJobQuery,
    args: PushArgsOwned,
    trigger: Option<TriggerMetadata>,
) -> error::Result<(
    Uuid,
    Option<String>,
    Option<sqlx::Transaction<'c, sqlx::Postgres>>,
)> {
    let FlowVersionInfo {
        version,
        tag,
        dedicated_worker,
        has_preprocessor,
        chat_input_enabled,
        on_behalf_of_email,
        edited_by,
        early_return,
        ..
    } = flow_version_info;

    let tag = run_query.tag.clone().or(tag);

    check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;
    let scheduled_for = run_query.get_scheduled_for(&db).await?;

    let return_tx = tx_o.is_some();

    let (email, permissioned_as, push_authed, tx) = if let Some(tx) = tx_o {
        (
            &authed.email,
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Transaction(tx),
        )
    } else if let Some(on_behalf_of_email) = on_behalf_of_email.as_ref() {
        (
            on_behalf_of_email,
            username_to_permissioned_as(&edited_by),
            None,
            PushIsolationLevel::IsolatedRoot(db.clone()),
        )
    } else {
        (
            &authed.email,
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into()),
        )
    };

    let (uuid, mut tx) = push(
        &db,
        tx,
        &w_id,
        JobPayload::Flow {
            path: flow_path.to_string(),
            dedicated_worker,
            version,
            apply_preprocessor: !run_query.skip_preprocessor.unwrap_or(false)
                && has_preprocessor.unwrap_or(false),
        },
        PushArgs { args: &args.args, extra: args.extra },
        authed.display_username(),
        email,
        permissioned_as,
        authed.token_prefix.as_deref(),
        scheduled_for,
        None,
        run_query.parent_job,
        None,
        run_query.root_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        None,
        None,
        None,
        push_authed.as_ref(),
        false,
        None,
        trigger,
        run_query.suspended_mode,
    )
    .await?;

    // Set memory_id if provided (for agent memory)
    if let Some(memory_id) = run_query.memory_id {
        set_flow_memory_id(&mut tx, uuid, memory_id).await?;
    }

    // Handle conversation messages for chat-enabled flows
    if chat_input_enabled.unwrap_or(false) {
        handle_chat_conversation_messages(
            &mut tx,
            &authed,
            &w_id,
            &flow_path.to_string(),
            &run_query,
            args.args.get("user_message"),
        )
        .await?;
    }

    // If we were given a transaction, return it; otherwise commit it
    if return_tx {
        Ok((uuid, early_return, Some(tx)))
    } else {
        tx.commit().await?;
        Ok((uuid, early_return, None))
    }
}

pub async fn run_flow_and_wait_result(
    authed: &ApiAuthed,
    db: &DB,
    user_db: UserDB,
    w_id: &str,
    flow_path: &str,
    flow_version_info: FlowVersionInfo,
    run_query: RunJobQuery,
    args: PushArgsOwned,
    trigger: Option<TriggerMetadata>,
) -> error::Result<Response> {
    let (uuid, early_return, _) = run_flow(
        authed,
        db,
        None,
        user_db,
        w_id,
        flow_path,
        flow_version_info,
        run_query,
        args,
        trigger,
    )
    .await?;

    run_wait_result(&db, uuid, w_id, early_return, &authed.username).await
}

// ---------------------------------------------------------------------------
// Push job by path
// ---------------------------------------------------------------------------

pub async fn push_flow_job_by_path_into_queue<'c>(
    authed: ApiAuthed,
    db: DB,
    tx_o: Option<sqlx::Transaction<'c, sqlx::Postgres>>,
    user_db: UserDB,
    w_id: String,
    flow_path: StripPath,
    run_query: RunJobQuery,
    args: PushArgsOwned,
    trigger: Option<TriggerMetadata>,
) -> error::Result<(
    Uuid,
    Option<String>,
    Option<sqlx::Transaction<'c, sqlx::Postgres>>,
)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let flow_path = flow_path.to_path();
    check_scopes(&authed, || format!("jobs:run:flows:{flow_path}"))?;

    let userdb_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };

    let flow_version_info =
        get_latest_flow_version_info_for_path(Some(userdb_authed), &db, &w_id, &flow_path, true)
            .await?;

    run_flow(
        &authed,
        &db,
        tx_o,
        user_db,
        &w_id,
        flow_path,
        flow_version_info,
        run_query,
        args,
        trigger,
    )
    .await
}

pub async fn push_script_job_by_path_into_queue<'c>(
    authed: ApiAuthed,
    db: DB,
    tx_o: Option<sqlx::Transaction<'c, sqlx::Postgres>>,
    user_db: UserDB,
    w_id: String,
    script_path: StripPath,
    run_query: RunJobQuery,
    args: PushArgsOwned,
    trigger: Option<TriggerMetadata>,
) -> error::Result<(
    Uuid,
    Option<bool>,
    Option<sqlx::Transaction<'c, sqlx::Postgres>>,
)> {
    #[cfg(feature = "enterprise")]
    check_license_key_valid().await?;

    let script_path = script_path.to_path();
    check_scopes(&authed, || format!("jobs:run:scripts:{script_path}"))?;

    let userdb_authed = UserDbWithAuthed { db: user_db.clone(), authed: &authed.to_authed_ref() };
    let (job_payload, tag, delete_after_use, timeout, on_behalf_of) = script_path_to_payload(
        script_path,
        Some(userdb_authed),
        db.clone(),
        &w_id,
        run_query.skip_preprocessor,
    )
    .await?;
    let scheduled_for = run_query.get_scheduled_for(&db).await?;

    let tag = run_query.tag.clone().or(tag);
    check_tag_available_for_workspace(&db, &w_id, &tag, &authed).await?;

    let return_tx = tx_o.is_some();

    let (email, permissioned_as, push_authed, tx) = if let Some(tx) = tx_o {
        (
            authed.email.as_str(),
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Transaction(tx),
        )
    } else if let Some(on_behalf_of) = on_behalf_of.as_ref() {
        (
            on_behalf_of.email.as_str(),
            on_behalf_of.permissioned_as.clone(),
            None,
            PushIsolationLevel::IsolatedRoot(db.clone()),
        )
    } else {
        (
            authed.email.as_str(),
            username_to_permissioned_as(&authed.username),
            Some(authed.clone().into()),
            PushIsolationLevel::Isolated(user_db, authed.clone().into()),
        )
    };

    let (uuid, tx) = push(
        &db,
        tx,
        &w_id,
        job_payload,
        PushArgs { args: &args.args, extra: args.extra },
        authed.display_username(),
        email,
        permissioned_as,
        authed.token_prefix.as_deref(),
        scheduled_for,
        None,
        run_query.parent_job,
        None,
        run_query.root_job,
        run_query.job_id,
        false,
        false,
        None,
        !run_query.invisible_to_owner.unwrap_or(false),
        tag,
        timeout,
        None,
        // If the job has a parent job, set priority to 2 as it may be ran synchronously and block a current worker until being executed. Flow steps have a priority of 1 so this is higher.
        if run_query.parent_job.is_some() || run_query.root_job.is_some() {
            Some(2)
        } else {
            None
        },
        push_authed.as_ref(),
        false,
        None,
        trigger,
        run_query.suspended_mode,
    )
    .await?;

    // If we were given a transaction, return it; otherwise commit it
    if return_tx {
        Ok((uuid, delete_after_use, Some(tx)))
    } else {
        tx.commit().await?;
        Ok((uuid, delete_after_use, None))
    }
}
