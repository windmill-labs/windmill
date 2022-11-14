/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde_json::{Map, Value};
use sqlx::{Pool, Postgres, Transaction};
use tracing::instrument;
use uuid::Uuid;
use windmill_common::{error::Error, flow_status::FlowStatusModule};
use windmill_queue::{delete_job, JobKind, QueuedJob};

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job_error<E: ToString + std::fmt::Debug>(
    db: &Pool<Postgres>,
    client: &windmill_api_client::Client,
    queued_job: &QueuedJob,
    logs: String,
    e: E,
    metrics: Option<crate::worker::Metrics>,
) -> Result<(Uuid, serde_json::Map<String, serde_json::Value>), Error> {
    metrics.map(|m| m.worker_execution_failed.inc());
    let mut output_map = Map::new();
    error_to_result(&mut output_map, &e);
    let a = add_completed_job(
        db,
        client,
        &queued_job,
        false,
        false,
        serde_json::Value::Object(output_map.clone()),
        logs,
    )
    .await?;
    Ok((a, output_map))
}

pub fn error_to_result<E: ToString + std::fmt::Debug>(
    output_map: &mut Map<String, Value>,
    err: &E,
) {
    output_map.insert(
        "error".to_string(),
        serde_json::Value::String(err.to_string()),
    );
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job(
    db: &Pool<Postgres>,
    client: &windmill_api_client::Client,
    queued_job: &QueuedJob,
    success: bool,
    skipped: bool,
    result: serde_json::Value,
    logs: String,
) -> Result<Uuid, Error> {
    let duration =
        if queued_job.job_kind == JobKind::Flow || queued_job.job_kind == JobKind::FlowPreview {
            let jobs = queued_job.parse_flow_status().map(|s| {
                let mut modules = s.modules;
                modules.extend([s.failure_module]);
                modules
                    .into_iter()
                    .filter_map(|m| match m {
                        FlowStatusModule::Success { job, .. }
                        | FlowStatusModule::Failure { job, .. } => Some(job),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            });
            if let Some(jobs) = jobs {
                sqlx::query_scalar!(
                    "SELECT SUM(duration_ms) as duration FROM completed_job WHERE id = ANY($1)",
                    jobs.as_slice()
                )
                .fetch_one(db)
                .await
                .ok()
                .flatten()
            } else {
                tracing::warn!("Could not parse flow status");
                None
            }
        } else {
            None
        };
    let mut tx = db.begin().await?;
    let job_id = queued_job.id.clone();
    sqlx::query!(
        "INSERT INTO completed_job AS cj
                   ( workspace_id
                   , id
                   , parent_job
                   , created_by
                   , created_at
                   , started_at
                   , duration_ms
                   , success
                   , script_hash
                   , script_path
                   , args
                   , result
                   , logs
                   , raw_code
                   , raw_lock
                   , canceled
                   , canceled_by
                   , canceled_reason
                   , job_kind
                   , schedule_path
                   , permissioned_as
                   , flow_status
                   , raw_flow
                   , is_flow_step
                   , is_skipped
                   , language )
            VALUES ($1, $2, $3, $4, $5, $6, COALESCE($26, EXTRACT(milliseconds FROM (now() - $6))), $7, $8, $9,\
                    $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25)
         ON CONFLICT (id) DO UPDATE SET success = $7, result = $11, logs = concat(cj.logs, $12)",
        queued_job.workspace_id,
        queued_job.id,
        queued_job.parent_job,
        queued_job.created_by,
        queued_job.created_at,
        queued_job.started_at,
        success,
        queued_job.script_hash.map(|x| x.0),
        queued_job.script_path,
        queued_job.args,
        result,
        logs,
        queued_job.raw_code,
        queued_job.raw_lock,
        queued_job.canceled,
        queued_job.canceled_by,
        queued_job.canceled_reason,
        queued_job.job_kind: JobKind,
        queued_job.schedule_path,
        queued_job.permissioned_as,
        queued_job.flow_status,
        queued_job.raw_flow,
        queued_job.is_flow_step,
        skipped,
        queued_job.language: ScriptLang,
        duration: Option<i64>
    )
    .execute(&mut tx)
    .await
    .map_err(|e| Error::InternalErr(format!("Could not add completed job {job_id}: {e}")))?;
    let _ = delete_job(db, &queued_job.workspace_id, job_id).await?;
    if !queued_job.is_flow_step
        && queued_job.job_kind != JobKind::Flow
        && queued_job.job_kind != JobKind::FlowPreview
        && queued_job.schedule_path.is_some()
        && queued_job.script_path.is_some()
    {
        tx = schedule_again_if_scheduled(
            tx,
            client,
            queued_job.schedule_path.as_ref().unwrap(),
            queued_job.script_path.as_ref().unwrap(),
            &queued_job.workspace_id,
        )
        .await?;
    }
    tx.commit().await?;
    tracing::debug!("Added completed job {}", queued_job.id);
    Ok(queued_job.id)
}

#[instrument(level = "trace", skip_all)]
pub async fn schedule_again_if_scheduled<'c>(
    mut tx: Transaction<'c, Postgres>,
    client: &windmill_api_client::Client,
    schedule_path: &str,
    script_path: &str,
    w_id: &str,
) -> windmill_common::error::Result<Transaction<'c, Postgres>> {
    let schedule = client
        .get_schedule(w_id, schedule_path)
        .await
        .map_err(|_| {
            Error::InternalErr(format!(
                "Could not find schedule {:?} for workspace {}",
                schedule_path, w_id
            ))
        })?
        .into_inner();
    if schedule.enabled && script_path == schedule.script_path {
        tx = windmill_queue::schedule::push_scheduled_job(
            tx,
            windmill_queue::schedule::Schedule {
                workspace_id: w_id.to_owned(),
                path: schedule.path,
                edited_by: schedule.edited_by,
                edited_at: schedule.edited_at,
                schedule: schedule.schedule,
                offset_: schedule.offset as _,
                enabled: schedule.enabled,
                script_path: schedule.script_path,
                is_flow: schedule.is_flow,
                args: schedule
                    .args
                    .and_then(|e| serde_json::to_value(e).map_or(None, |v| Some(v))),
                extra_perms: serde_json::to_value(schedule.extra_perms).expect("hashmap -> json"),
            },
        )
        .await?;
    }

    Ok(tx)
}
