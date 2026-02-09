use crate::trigger_helpers::trigger_runnable_inner;
use axum::{
    extract::{Extension, Path},
    response::Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;
use windmill_api_auth::ApiAuthed;
use windmill_api_jobs::execution::cancel_jobs;
use windmill_common::{
    db::{UserDB, DB},
    error::{self, Error, Result},
    jobs::JobTriggerKind,
    triggers::TriggerMetadata,
};

#[derive(sqlx::FromRow)]
pub struct SuspendedTrigger {
    pub script_path: String,
    pub is_flow: bool,
    pub edited_by: String,
    pub email: String,
    pub edited_at: DateTime<Utc>,
    pub error_handler_path: Option<String>,
    pub error_handler_args: Option<sqlx::types::Json<HashMap<String, serde_json::Value>>>,
    pub retry: Option<sqlx::types::Json<windmill_common::flows::Retry>>,
}

async fn get_suspended_trigger(
    tx: &mut PgConnection,
    workspace_id: &str,
    trigger_kind: &JobTriggerKind,
    path: &str,
) -> Result<SuspendedTrigger> {
    match trigger_kind {
        JobTriggerKind::Webhook | JobTriggerKind::Schedule => {
            return Err(Error::BadRequest(format!(
                "{} triggers do not support job reassignment",
                trigger_kind
            )));
        }
        _ => {}
    }

    let table_name = format!("{}_trigger", trigger_kind.to_string());

    let fields = vec![
        "script_path",
        "is_flow",
        "edited_by",
        "email",
        "edited_at",
        "error_handler_path",
        "error_handler_args",
        "retry",
    ];

    let sql = format!(
        r#"SELECT
        {}
    FROM
        {}
    WHERE
        workspace_id = $1 AND
        path = $2
    "#,
        fields.join(", "),
        table_name
    );

    sqlx::query_as(&sql)
        .bind(workspace_id)
        .bind(path)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Trigger not found at path: {}", path)))
}

struct JobWithArgs {
    id: Uuid,
    args: Option<sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct ReassignJobsBody {
    #[serde(default)]
    pub job_ids: Option<Vec<Uuid>>,
}

pub async fn resume_suspended_trigger_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, trigger_kind, trigger_path)): Path<(String, JobTriggerKind, String)>,
    Json(body): Json<ReassignJobsBody>,
) -> error::Result<Json<String>> {
    let mut tx = user_db.clone().begin(&authed).await?;

    let trigger = get_suspended_trigger(&mut *tx, &w_id, &trigger_kind, &trigger_path).await?;

    let jobs = if let Some(job_ids) = body.job_ids.as_ref() {
        if job_ids.is_empty() {
            vec![]
        } else {
            sqlx::query_as!(
                JobWithArgs,
                r#"
                    SELECT
                        id,
                        args as "args: _",
                        created_at
                    FROM v2_job
                    WHERE workspace_id = $1
                      AND (
                            kind = 'unassigned_script'::JOB_KIND OR
                            kind = 'unassigned_flow'::JOB_KIND OR
                            kind = 'unassigned_singlestepflow'::JOB_KIND
                      )
                      AND trigger_kind = $2
                      AND trigger = $3
                      AND id = ANY($4)
                "#,
                w_id,
                trigger_kind as _,
                trigger_path,
                job_ids as _,
            )
            .fetch_all(&mut *tx)
            .await?
        }
    } else {
        sqlx::query_as!(
            JobWithArgs,
            r#"
                SELECT
                    id,
                    args as "args: _",
                    created_at
                FROM v2_job
                WHERE workspace_id = $1
                  AND (
                        kind = 'unassigned_script'::JOB_KIND OR
                        kind = 'unassigned_flow'::JOB_KIND OR
                        kind = 'unassigned_singlestepflow'::JOB_KIND
                  )
                  AND trigger_kind = $2
                  AND trigger = $3
            "#,
            w_id,
            trigger_kind as _,
            trigger_path,
        )
        .fetch_all(&mut *tx)
        .await?
    };

    let trigger_metadata = TriggerMetadata::new(Some(trigger_path.clone()), trigger_kind);

    let l = jobs.len();

    for job in jobs {
        // If job was created before trigger was edited, simply update it to unsuspend
        // instead of deleting and repushing
        if job.created_at > trigger.edited_at {
            let job_kind = if trigger.is_flow {
                windmill_common::jobs::JobKind::Flow
            } else {
                windmill_common::jobs::JobKind::Script
            };

            sqlx::query!(
                "UPDATE v2_job SET kind = $1 WHERE id = $2",
                job_kind as _,
                job.id
            )
            .execute(&mut *tx)
            .await?;

            // Update the job to unsuspend it and set the correct kind
            sqlx::query!(
                "UPDATE v2_job_queue SET scheduled_for = now() WHERE id = $1",
                job.id
            )
            .execute(&mut *tx)
            .await?;
        } else {
            // Job was created after trigger edit - delete and repush with new configuration
            // Pass the transaction to trigger_runnable_inner so everything is in the same transaction
            let (_uuid, _delete_after_use, _early_return, tx_o) = trigger_runnable_inner(
                &db,
                Some(tx),
                Some(user_db.clone()),
                authed.clone(),
                &w_id,
                &trigger.script_path,
                trigger.is_flow,
                windmill_queue::PushArgsOwned {
                    extra: None,
                    args: job.args.map(|a| a.0).unwrap_or_default(),
                },
                trigger.retry.as_ref(),
                trigger.error_handler_path.as_deref(),
                trigger.error_handler_args.as_ref(),
                trigger_path.clone(),
                None,
                trigger_metadata.clone(),
                None,
            )
            .await?;

            tx = match tx_o {
                Some(tx) => tx,
                None => {
                    return Err(error::Error::internal_err(
                        "Transaction should be returned when passed in".to_string(),
                    ));
                }
            };

            // Delete the unassigned job from all related tables
            sqlx::query!("DELETE FROM v2_job_queue WHERE id = $1", job.id)
                .execute(&mut *tx)
                .await?;

            sqlx::query!("DELETE FROM v2_job_runtime WHERE id = $1", job.id)
                .execute(&mut *tx)
                .await?;

            sqlx::query!("DELETE FROM job_perms WHERE job_id = $1", job.id)
                .execute(&mut *tx)
                .await?;

            sqlx::query!("DELETE FROM concurrency_key WHERE job_id = $1", job.id)
                .execute(&mut *tx)
                .await?;

            sqlx::query!("DELETE FROM debounce_key WHERE job_id = $1", job.id)
                .execute(&mut *tx)
                .await?;

            sqlx::query!("DELETE FROM debounce_stale_data WHERE job_id = $1", job.id)
                .execute(&mut *tx)
                .await?;

            sqlx::query!("DELETE FROM v2_job WHERE id = $1", job.id)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;

    Ok(Json(format!("Reassigned {} jobs", l)))
}

pub async fn cancel_suspended_trigger_jobs(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, trigger_kind, trigger_path)): Path<(String, JobTriggerKind, String)>,
    Json(body): Json<ReassignJobsBody>,
) -> error::Result<Json<String>> {
    let mut tx = user_db.clone().begin(&authed).await?;

    // Get the list of job IDs to cancel
    let jobs_to_cancel = if let Some(job_ids) = body.job_ids.as_ref() {
        if job_ids.is_empty() {
            vec![]
        } else {
            sqlx::query_scalar!(
                "SELECT id FROM v2_job
                 WHERE workspace_id = $1
                   AND (kind = 'unassigned_script'::JOB_KIND OR kind = 'unassigned_flow'::JOB_KIND OR kind = 'unassigned_singlestepflow'::JOB_KIND)
                   AND trigger_kind = $2
                   AND trigger = $3
                   AND id = ANY($4)",
                w_id,
                trigger_kind as _,
                trigger_path,
                job_ids as _,
            )
            .fetch_all(&mut *tx)
            .await?
        }
    } else {
        sqlx::query_scalar!(
            "SELECT id FROM v2_job
             WHERE workspace_id = $1
               AND (kind = 'unassigned_script'::JOB_KIND OR kind = 'unassigned_flow'::JOB_KIND OR kind = 'unassigned_singlestepflow'::JOB_KIND)
               AND trigger_kind = $2
               AND trigger = $3",
            w_id,
            trigger_kind as _,
            trigger_path,
        )
        .fetch_all(&mut *tx)
        .await?
    };

    tx.commit().await?;

    let count = jobs_to_cancel.len();

    if count > 0 {
        let cancelled_jobs = cancel_jobs(
            jobs_to_cancel,
            &db,
            authed.username.as_str(),
            w_id.as_str(),
            true,
        )
        .await?;
        Ok(Json(format!("Canceled {} jobs", cancelled_jobs.0.len())))
    } else {
        Ok(Json(format!("No jobs to cancel")))
    }
}
