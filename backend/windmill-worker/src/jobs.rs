use sqlx::{Pool, Postgres};
use tracing::instrument;
use uuid::Uuid;
use windmill_api_client::apis::{configuration, schedule_api};
use windmill_common::error::Error;
pub use windmill_queue::*;

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job_error<E: ToString + std::fmt::Debug>(
    db: &Pool<Postgres>,
    api_config: &configuration::Configuration,
    queued_job: &QueuedJob,
    logs: String,
    e: E,
    metrics: Option<crate::worker::Metrics>,
) -> Result<(Uuid, serde_json::Map<String, serde_json::Value>), Error> {
    metrics.map(|m| m.worker_execution_failed.inc());
    let mut output_map = serde_json::Map::new();
    output_map.insert(
        "error".to_string(),
        serde_json::Value::String(e.to_string()),
    );
    let a = add_completed_job(
        db,
        api_config,
        &queued_job,
        false,
        false,
        serde_json::Value::Object(output_map.clone()),
        logs,
    )
    .await?;
    Ok((a, output_map))
}

#[instrument(level = "trace", skip_all)]
pub async fn add_completed_job(
    db: &Pool<Postgres>,
    api_config: &configuration::Configuration,
    queued_job: &QueuedJob,
    success: bool,
    skipped: bool,
    result: serde_json::Value,
    logs: String,
) -> Result<Uuid, Error> {
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
            VALUES ($1, $2, $3, $4, $5, $6, EXTRACT(milliseconds FROM (now() - $6)), $7, $8, $9,\
                    $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
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
        schedule_again_if_scheduled(
            api_config,
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
    api_config: &configuration::Configuration,
    schedule_path: &str,
    script_path: &str,
    w_id: &str,
) -> windmill_common::error::Result<()> {
    // MARKER: WINDMILL API CLIENT
    let schedule = schedule_api::get_schedule(api_config, w_id, schedule_path)
        .await
        .map_err(|_| {
            Error::InternalErr(format!(
                "Could not find schedule {:?} for workspace {}",
                schedule_path, w_id
            ))
        })?;
    if schedule.enabled && script_path == schedule.script_path {
        todo!("push scheduled job here");
        // tx = crate::schedule::push_scheduled_job(tx, schedule).await?;
    }

    Ok(())
}
