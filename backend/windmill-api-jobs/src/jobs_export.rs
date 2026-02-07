/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use axum::{
    extract::{Path, Query},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_common::{
    db::UserDB,
    error,
    jobs::{JobKind, JobStatus, JobTriggerKind},
    scripts::ScriptLang,
    utils::{paginate, paginate_without_limits, require_admin, Pagination},
};

use windmill_api_auth::ApiAuthed;

#[derive(Serialize, Deserialize)]
pub struct ExportableCompletedJob {
    pub id: Uuid,
    pub raw_code: Option<String>,
    pub raw_lock: Option<String>,
    pub raw_flow: Option<sqlx::types::Json<Box<RawValue>>>,
    pub tag: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: String,
    pub permissioned_as: String,
    pub permissioned_as_email: String,
    pub kind: JobKind,
    pub runnable_id: Option<i64>,
    pub runnable_path: Option<String>,
    pub parent_job: Option<Uuid>,
    pub root_job: Option<Uuid>,
    pub script_lang: Option<ScriptLang>,
    pub script_entrypoint_override: Option<String>,
    pub flow_step: Option<i32>,
    pub flow_step_id: Option<String>,
    pub flow_innermost_root_job: Option<Uuid>,
    pub trigger: Option<String>,
    pub trigger_kind: Option<JobTriggerKind>,
    pub same_worker: bool,
    pub visible_to_owner: bool,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub cache_ttl: Option<i32>,
    pub timeout: Option<i32>,
    pub priority: Option<i16>,
    pub preprocessed: Option<bool>,
    pub args: Option<sqlx::types::Json<Box<RawValue>>>,
    pub labels: Option<Vec<String>>,
    pub pre_run_error: Option<String>,
    pub duration_ms: i64,
    pub result: Option<sqlx::types::Json<Box<RawValue>>>,
    pub deleted: bool,
    pub canceled_by: Option<String>,
    pub canceled_reason: Option<String>,
    pub flow_status: Option<sqlx::types::Json<Box<RawValue>>>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub memory_peak: Option<i32>,
    pub status: JobStatus,
    pub completed_at: chrono::DateTime<chrono::Utc>,
    pub worker: Option<String>,
    pub workflow_as_code_status: Option<sqlx::types::Json<Box<RawValue>>>,
    pub result_columns: Option<Vec<String>>,
    pub retries: Option<Vec<Uuid>>,
    pub extras: Option<sqlx::types::Json<Box<RawValue>>>,
    pub logs: Option<String>,
    pub log_offset: Option<i32>,
    pub log_file_index: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct ExportableQueuedJob {
    // v2_job columns (excluding workspace_id)
    pub id: Uuid,
    pub raw_code: Option<String>,
    pub raw_lock: Option<String>,
    pub raw_flow: Option<sqlx::types::Json<Box<RawValue>>>,
    pub tag: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: String,
    pub permissioned_as: String,
    pub permissioned_as_email: String,
    pub kind: JobKind,
    pub runnable_id: Option<i64>,
    pub runnable_path: Option<String>,
    pub parent_job: Option<Uuid>,
    pub root_job: Option<Uuid>,
    pub script_lang: Option<ScriptLang>,
    pub script_entrypoint_override: Option<String>,
    pub flow_step: Option<i32>,
    pub flow_step_id: Option<String>,
    pub flow_innermost_root_job: Option<Uuid>,
    pub trigger: Option<String>,
    pub trigger_kind: Option<JobTriggerKind>,
    pub same_worker: bool,
    pub visible_to_owner: bool,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub cache_ttl: Option<i32>,
    pub timeout: Option<i32>,
    pub priority: Option<i16>,
    pub preprocessed: Option<bool>,
    pub args: Option<sqlx::types::Json<Box<RawValue>>>,
    pub labels: Option<Vec<String>>,
    pub pre_run_error: Option<String>,

    // v2_job_queue columns (excluding workspace_id and id/created_at/tag/priority)
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    pub running: bool,
    pub canceled_by: Option<String>,
    pub canceled_reason: Option<String>,
    pub suspend: Option<i32>,
    pub suspend_until: Option<chrono::DateTime<chrono::Utc>>,
    pub worker: Option<String>,
    pub extras: Option<sqlx::types::Json<Box<RawValue>>>,

    // v2_job_runtime columns (excluding id)
    pub ping: Option<chrono::DateTime<chrono::Utc>>,
    pub memory_peak: Option<i32>,

    // v2_job_status columns (excluding id)
    pub flow_status: Option<sqlx::types::Json<Box<RawValue>>>,
    pub flow_leaf_jobs: Option<sqlx::types::Json<Box<RawValue>>>,
    pub workflow_as_code_status: Option<sqlx::types::Json<Box<RawValue>>>,

    // concurrency_key table
    pub concurrency_key: Option<String>,
}

pub async fn export_completed_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> error::JsonResult<Vec<ExportableCompletedJob>> {
    require_admin(authed.is_admin, &authed.username)?;

    let (per_page, offset) = paginate(pagination);
    let per_page = per_page as i64;
    let offset = offset as i64;

    let mut tx = user_db.begin(&authed).await?;

    let jobs = sqlx::query_as!(
        ExportableCompletedJob,
        r#"
        SELECT
            v2_job.id,
            v2_job.raw_code,
            v2_job.raw_lock,
            v2_job.raw_flow as "raw_flow: _",
            v2_job.tag,
            v2_job.created_at,
            v2_job.created_by,
            v2_job.permissioned_as,
            v2_job.permissioned_as_email,
            v2_job.kind as "kind: _",
            v2_job.runnable_id,
            v2_job.runnable_path,
            v2_job.parent_job,
            v2_job.root_job,
            v2_job.script_lang as "script_lang: _",
            v2_job.script_entrypoint_override,
            v2_job.flow_step,
            v2_job.flow_step_id,
            v2_job.flow_innermost_root_job,
            v2_job.trigger,
            v2_job.trigger_kind as "trigger_kind: _",
            v2_job.same_worker,
            v2_job.visible_to_owner,
            v2_job.concurrent_limit,
            v2_job.concurrency_time_window_s,
            v2_job.cache_ttl,
            v2_job.timeout,
            v2_job.priority,
            v2_job.preprocessed,
            v2_job.args as "args: _",
            v2_job.labels,
            v2_job.pre_run_error,
            v2_job_completed.duration_ms,
            v2_job_completed.result as "result: _",
            v2_job_completed.deleted,
            v2_job_completed.canceled_by,
            v2_job_completed.canceled_reason,
            v2_job_completed.flow_status as "flow_status: _",
            v2_job_completed.started_at,
            v2_job_completed.memory_peak,
            v2_job_completed.status as "status: _",
            v2_job_completed.completed_at,
            v2_job_completed.worker,
            v2_job_completed.workflow_as_code_status as "workflow_as_code_status: _",
            v2_job_completed.result_columns,
            v2_job_completed.retries,
            v2_job_completed.extras as "extras: _",
            job_logs.logs AS logs,
            job_logs.log_offset,
            job_logs.log_file_index
        FROM v2_job_completed
        INNER JOIN v2_job ON v2_job.id = v2_job_completed.id
        LEFT JOIN v2_job_status ON v2_job_completed.id = v2_job_status.id
        LEFT JOIN job_logs ON job_logs.job_id = v2_job.id
        WHERE v2_job_completed.workspace_id = $1
        ORDER BY v2_job.created_at DESC
        LIMIT $2
        OFFSET $3
        "#,
        w_id,
        per_page,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(jobs))
}

pub async fn export_queued_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(pagination): Query<Pagination>,
) -> error::JsonResult<Vec<ExportableQueuedJob>> {
    require_admin(authed.is_admin, &authed.username)?;

    let (per_page, offset) = paginate_without_limits(pagination);
    let per_page = per_page as i64;
    let offset = offset as i64;

    let mut tx = user_db.begin(&authed).await?;

    let jobs = sqlx::query_as!(
        ExportableQueuedJob,
        r#"
        SELECT
            v2_job.id,
            v2_job.raw_code,
            v2_job.raw_lock,
            v2_job.raw_flow as "raw_flow: _",
            v2_job.tag,
            v2_job.created_at,
            v2_job.created_by,
            v2_job.permissioned_as,
            v2_job.permissioned_as_email,
            v2_job.kind as "kind: _",
            v2_job.runnable_id,
            v2_job.runnable_path,
            v2_job.parent_job,
            v2_job.root_job,
            v2_job.script_lang as "script_lang: _",
            v2_job.script_entrypoint_override,
            v2_job.flow_step,
            v2_job.flow_step_id,
            v2_job.flow_innermost_root_job,
            v2_job."trigger",
            v2_job.trigger_kind as "trigger_kind: _",
            v2_job.same_worker,
            v2_job.visible_to_owner,
            v2_job.concurrent_limit,
            v2_job.concurrency_time_window_s,
            v2_job.cache_ttl,
            v2_job.timeout,
            v2_job.priority,
            v2_job.preprocessed,
            v2_job.args as "args: _",
            v2_job.labels,
            v2_job.pre_run_error,

            v2_job_queue.started_at,
            v2_job_queue.scheduled_for,
            v2_job_queue.running,
            v2_job_queue.canceled_by,
            v2_job_queue.canceled_reason,
            v2_job_queue.suspend,
            v2_job_queue.suspend_until,
            v2_job_queue.worker,
            v2_job_queue.extras as "extras: _",

            v2_job_runtime.ping,
            v2_job_runtime.memory_peak,

            v2_job_status.flow_status as "flow_status: _",
            v2_job_status.flow_leaf_jobs as "flow_leaf_jobs: _",
            v2_job_status.workflow_as_code_status as "workflow_as_code_status: _",

            concurrency_key.key as "concurrency_key?"
        FROM v2_job_queue
        INNER JOIN v2_job ON v2_job.id = v2_job_queue.id
        LEFT JOIN v2_job_runtime ON v2_job_runtime.id = v2_job_queue.id
        LEFT JOIN v2_job_status ON v2_job_status.id = v2_job_queue.id
        LEFT JOIN concurrency_key ON concurrency_key.job_id = v2_job_queue.id
        WHERE v2_job_queue.workspace_id = $1
            AND v2_job_queue.running = false
            AND v2_job.parent_job IS NULL
            AND v2_job.trigger_kind IS DISTINCT FROM 'schedule'
        ORDER BY v2_job.created_at DESC
        LIMIT $2
        OFFSET $3
        "#,
        w_id,
        per_page,
        offset
    )
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(jobs))
}

pub async fn import_completed_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(jobs): Json<Vec<ExportableCompletedJob>>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;

    for job in jobs {
        sqlx::query!(
            r#"
            INSERT INTO v2_job (
                id, raw_code, raw_lock, raw_flow, tag, workspace_id, created_at, created_by,
                permissioned_as, permissioned_as_email, kind, runnable_id, runnable_path,
                parent_job, root_job, script_lang, script_entrypoint_override, flow_step,
                flow_step_id, flow_innermost_root_job, trigger, trigger_kind, same_worker,
                visible_to_owner, concurrent_limit, concurrency_time_window_s, cache_ttl,
                timeout, priority, preprocessed, args, labels, pre_run_error
            ) VALUES (
                $1,  $2,  $3,  $4,  $5,  $6,  $7,  $8,  $9,  $10,  $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33
            )
            ON CONFLICT (id) DO NOTHING
            "#,
            job.id, job.raw_code, job.raw_lock, job.raw_flow as _, &job.tag, &w_id,
            job.created_at, &job.created_by, &job.permissioned_as, job.permissioned_as_email,
            job.kind as _, job.runnable_id, job.runnable_path, job.parent_job, job.root_job,
            job.script_lang as _, job.script_entrypoint_override, job.flow_step,
            job.flow_step_id, job.flow_innermost_root_job, job.trigger, job.trigger_kind as _,
            job.same_worker, job.visible_to_owner, job.concurrent_limit,
            job.concurrency_time_window_s, job.cache_ttl, job.timeout, job.priority,
            job.preprocessed, job.args as _, job.labels as _, job.pre_run_error
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO v2_job_completed (
                id, workspace_id, started_at, completed_at, duration_ms, result, deleted,
                canceled_by, canceled_reason, flow_status, memory_peak, status, worker,
                workflow_as_code_status, result_columns, retries, extras
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
            )
            ON CONFLICT (id) DO NOTHING
            "#,
            job.id,
            &w_id,
            job.started_at,
            job.completed_at,
            job.duration_ms,
            job.result as _,
            job.deleted,
            job.canceled_by,
            job.canceled_reason,
            job.flow_status as _,
            job.memory_peak,
            job.status as _,
            job.worker,
            job.workflow_as_code_status as _,
            job.result_columns as _,
            job.retries as _,
            job.extras as _
        )
        .execute(&mut *tx)
        .await?;

        if let Some(logs) = &job.logs {
            sqlx::query!(
                r#"
            INSERT INTO job_logs (
                job_id, workspace_id, logs, log_offset, log_file_index
            ) VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (job_id) DO NOTHING
            "#,
                job.id,
                &w_id,
                logs,
                job.log_offset,
                job.log_file_index as _
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(format!("Successfully imported jobs"))
}

pub async fn import_queued_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(jobs): Json<Vec<ExportableQueuedJob>>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;

    for job in jobs {
        sqlx::query!(
            r#"
            INSERT INTO v2_job (
                id, raw_code, raw_lock, raw_flow, tag, workspace_id, created_at, created_by,
                permissioned_as, permissioned_as_email, kind, runnable_id, runnable_path,
                parent_job, root_job, script_lang, script_entrypoint_override, flow_step,
                flow_step_id, flow_innermost_root_job, trigger, trigger_kind, same_worker,
                visible_to_owner, concurrent_limit, concurrency_time_window_s, cache_ttl,
                timeout, priority, preprocessed, args, labels, pre_run_error
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                $21, $22, $23, $24, $25, $26, $27, $28, $29, $30,
                $31, $32, $33
            )
            ON CONFLICT (id) DO NOTHING
            "#,
            job.id,
            job.raw_code,
            job.raw_lock,
            job.raw_flow as _,
            &job.tag,
            &w_id,
            job.created_at,
            &job.created_by,
            &job.permissioned_as,
            &job.permissioned_as_email,
            job.kind as _,
            job.runnable_id,
            job.runnable_path,
            job.parent_job,
            job.root_job,
            job.script_lang as _,
            job.script_entrypoint_override,
            job.flow_step,
            job.flow_step_id,
            job.flow_innermost_root_job,
            job.trigger,
            job.trigger_kind as _,
            job.same_worker,
            job.visible_to_owner,
            job.concurrent_limit,
            job.concurrency_time_window_s,
            job.cache_ttl,
            job.timeout,
            job.priority,
            job.preprocessed,
            job.args as _,
            job.labels as _,
            job.pre_run_error,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO v2_job_queue (
                id, workspace_id, started_at, scheduled_for, running, canceled_by,
                canceled_reason, suspend, suspend_until, worker, extras, tag, priority
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            )
            ON CONFLICT (id) DO NOTHING
            "#,
            job.id,
            &w_id,
            job.started_at,
            job.scheduled_for,
            job.running,
            job.canceled_by,
            job.canceled_reason,
            job.suspend,
            job.suspend_until,
            job.worker,
            job.extras as _,
            &job.tag,
            job.priority
        )
        .execute(&mut *tx)
        .await?;

        if job.ping.is_some() || job.memory_peak.is_some() {
            sqlx::query!(
                r#"
                INSERT INTO v2_job_runtime (id, ping, memory_peak)
                VALUES ($1, $2, $3)
                ON CONFLICT (id) DO NOTHING
                "#,
                job.id,
                job.ping,
                job.memory_peak
            )
            .execute(&mut *tx)
            .await?;
        }

        if job.flow_status.is_some()
            || job.flow_leaf_jobs.is_some()
            || job.workflow_as_code_status.is_some()
        {
            sqlx::query!(
                r#"
                INSERT INTO v2_job_status (id, flow_status, flow_leaf_jobs, workflow_as_code_status)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (id) DO NOTHING
                "#,
                job.id,
                job.flow_status as _,
                job.flow_leaf_jobs as _,
                job.workflow_as_code_status as _
            )
            .execute(&mut *tx)
            .await?;
        }

        if let Some(ref concurrency_key) = job.concurrency_key {
            sqlx::query!(
                r#"
                WITH inserted_concurrency_counter AS (
                    INSERT INTO concurrency_counter (concurrency_id, job_uuids)
                    VALUES ($1, '{}'::jsonb)
                    ON CONFLICT DO NOTHING
                )
                INSERT INTO concurrency_key(key, job_id)
                VALUES ($1, $2)
                ON CONFLICT (job_id) DO NOTHING
                "#,
                concurrency_key,
                job.id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(format!("Successfully imported jobs"))
}

pub async fn delete_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Json(job_ids): Json<Vec<Uuid>>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    if job_ids.is_empty() {
        return Ok("No jobs to delete".to_string());
    }

    let mut tx = user_db.begin(&authed).await?;

    let logs_deleted = sqlx::query!(
        "DELETE FROM job_logs WHERE workspace_id = $1 AND job_id = ANY($2)",
        &w_id,
        &job_ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let perms_deleted = sqlx::query!(
        "DELETE FROM job_perms WHERE workspace_id = $1 AND job_id = ANY($2)",
        &w_id,
        &job_ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let stats_deleted = sqlx::query!(
        "DELETE FROM job_stats WHERE workspace_id = $1 AND job_id = ANY($2)",
        &w_id,
        &job_ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let resume_deleted = sqlx::query!("DELETE FROM resume_job WHERE job = ANY($1)", &job_ids)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let runtime_deleted = sqlx::query!("DELETE FROM v2_job_runtime WHERE id = ANY($1)", &job_ids)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let status_deleted = sqlx::query!("DELETE FROM v2_job_status WHERE id = ANY($1)", &job_ids)
        .execute(&mut *tx)
        .await?
        .rows_affected();

    let concurrency_key_deleted = sqlx::query!(
        "DELETE FROM concurrency_key WHERE job_id = ANY($1)",
        &job_ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let queue_deleted = sqlx::query!(
        "DELETE FROM v2_job_queue WHERE workspace_id = $1 AND id = ANY($2)",
        &w_id,
        &job_ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let completed_deleted = sqlx::query!(
        "DELETE FROM v2_job_completed WHERE workspace_id = $1 AND id = ANY($2)",
        &w_id,
        &job_ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let zombie_deleted = sqlx::query!(
        "DELETE FROM zombie_job_counter WHERE job_id = ANY($1)",
        &job_ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let jobs_deleted = sqlx::query!(
        "DELETE FROM v2_job WHERE workspace_id = $1 AND id = ANY($2)",
        &w_id,
        &job_ids
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    tx.commit().await?;

    let total_rows_deleted = logs_deleted
        + perms_deleted
        + stats_deleted
        + resume_deleted
        + runtime_deleted
        + status_deleted
        + concurrency_key_deleted
        + queue_deleted
        + completed_deleted
        + zombie_deleted
        + jobs_deleted;

    tracing::info!(
        "Successfully deleted {} jobs ({} total rows across all tables) from workspace {}",
        job_ids.len(),
        total_rows_deleted,
        w_id
    );

    Ok(format!(
        "Successfully deleted {} jobs ({} total rows across all tables)",
        jobs_deleted, total_rows_deleted
    ))
}
