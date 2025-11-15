use crate::{db::ApiAuthed, triggers::INACTIVE_TRIGGER_SCHEDULED_FOR_DATE};
use axum::{
    extract::{Extension, Path},
    response::Json,
};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{db::UserDB, error, jobs::JobTriggerKind, utils::require_admin};

pub async fn resume_suspended_trigger_jobs(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, trigger_kind, trigger_path)): Path<(String, JobTriggerKind, String)>,
) -> error::Result<Json<String>> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;

    // Use the date constant to identify suspended jobs
    // This date (9999-12-31 23:59:59) is used as a marker for suspended jobs
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
    .execute(&mut *tx)
    .await?;

    let count = result.rows_affected();

    let trigger_kind_str = format!("{:?}", trigger_kind);
    let count_str = count.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "triggers.bulk_resume",
        ActionKind::Execute,
        &w_id,
        Some(&trigger_path),
        Some(
            [
                ("trigger_kind", trigger_kind_str.as_str()),
                ("jobs_count", count_str.as_str()),
            ]
            .iter()
            .cloned()
            .collect(),
        ),
    )
    .await?;

    tx.commit().await?;

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
    Extension(user_db): Extension<UserDB>,
    Path((w_id, trigger_kind, trigger_path)): Path<(String, JobTriggerKind, String)>,
) -> error::Result<Json<String>> {
    require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;

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
    .execute(&mut *tx)
    .await?;

    let count = result.rows_affected();

    let trigger_kind_str = format!("{:?}", trigger_kind);
    let count_str = count.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "triggers.bulk_cancel",
        ActionKind::Delete,
        &w_id,
        Some(&trigger_path),
        Some(
            [
                ("trigger_kind", trigger_kind_str.as_str()),
                ("jobs_count", count_str.as_str()),
            ]
            .iter()
            .cloned()
            .collect(),
        ),
    )
    .await?;

    tx.commit().await?;

    let message = format!(
        "Successfully cancelled {} suspended job{} for trigger at path: {}",
        count,
        if count == 1 { "" } else { "s" },
        &trigger_path
    );
    Ok(Json(message))
}
