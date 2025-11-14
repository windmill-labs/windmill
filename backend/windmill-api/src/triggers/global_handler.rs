use crate::{db::{ApiAuthed, DB}, triggers::INACTIVE_TRIGGER_SCHEDULED_FOR_DATE};
use axum::{
    extract::{Extension, Path},
    response::Json,
};
use windmill_common::{error, jobs::JobTriggerKind, utils::require_admin};

pub async fn resume_suspended_trigger_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, trigger_kind, trigger_path)): Path<(String, JobTriggerKind, String)>,
) -> error::Result<Json<String>> {
    require_admin(authed.is_admin, &authed.username)?;
    let scheduled_for = INACTIVE_TRIGGER_SCHEDULED_FOR_DATE.clone();
    let result = sqlx::query!(
        r#"
            UPDATE 
                v2_job_queue 
            SET 
                scheduled_for = now() 
            FROM 
                v2_job 
            WHERE 
                v2_job_queue.id = v2_job.id AND 
                v2_job_queue.running is FALSE AND
                v2_job_queue.scheduled_for = $1 AND
                v2_job_queue.workspace_id = $2 AND
                v2_job.trigger_kind = $3 AND
                v2_job.trigger = $4
        "#,
        scheduled_for,
        w_id,
        trigger_kind as _,
        trigger_path
    )
    .execute(&db)
    .await?;

    let count = result.rows_affected();

    let message = format!(
        "Successfully resumed {} suspended job{} for trigger at path: {}",
        count,
        if count == 1 { "" } else { "s" },
        &trigger_path
    );
    Ok(Json(message))
}

pub async fn cancel_suspended_trigger_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, trigger_kind, trigger_path)): Path<(String, JobTriggerKind, String)>,
) -> error::Result<Json<String>> {
    require_admin(authed.is_admin, &authed.username)?;
    let scheduled_for = INACTIVE_TRIGGER_SCHEDULED_FOR_DATE.clone();

    let result = sqlx::query!(
        r#"
            UPDATE 
                v2_job_queue 
            SET 
                canceled_by = $1,
                canceled_reason = 'cancelled by trigger bulk operation',
                scheduled_for = now()
            FROM 
                v2_job 
            WHERE 
                v2_job_queue.id = v2_job.id AND 
                v2_job_queue.workspace_id = $2 AND 
                v2_job_queue.running is FALSE AND
                v2_job_queue.scheduled_for = $3 AND
                v2_job.trigger_kind = $4 AND
                v2_job.trigger = $5
        "#,
        authed.username,
        w_id,
        scheduled_for,
        trigger_kind as _,
        trigger_path
    )
    .execute(&db)
    .await?;

    let count = result.rows_affected();

    let message = format!(
        "Successfully cancelled {} suspended job{} for trigger at path: {}",
        count,
        if count == 1 { "" } else { "s" },
        &trigger_path
    );
    Ok(Json(message))
}
