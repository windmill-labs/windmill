use crate::{
    db::{ApiAuthed, DB},
    triggers::{trigger_helpers::trigger_runnable_inner, TriggerCrud, TriggerForReassignment},
};

#[cfg(feature = "http_trigger")]
use crate::triggers::http::handler::HttpTrigger;

#[cfg(feature = "mqtt_trigger")]
use crate::triggers::mqtt::{MqttConfig, MqttTrigger};

#[cfg(feature = "postgres_trigger")]
use crate::triggers::postgres::{PostgresConfig, PostgresTrigger};

#[cfg(feature = "websocket")]
use crate::triggers::websocket::WebsocketTrigger;

#[cfg(all(feature = "smtp", feature = "enterprise", feature = "private"))]
use crate::triggers::email::{EmailConfig, EmailTrigger};

#[cfg(all(feature = "gcp_trigger", feature = "enterprise", feature = "private"))]
use crate::triggers::gcp::{GcpConfig, GcpTrigger};

#[cfg(all(feature = "kafka", feature = "enterprise", feature = "private"))]
use crate::triggers::kafka::{KafkaConfig, KafkaTrigger};

#[cfg(all(feature = "nats", feature = "enterprise", feature = "private"))]
use crate::triggers::nats::{NatsConfig, NatsTrigger};

#[cfg(all(feature = "sqs_trigger", feature = "enterprise", feature = "private"))]
use crate::triggers::sqs::{SqsConfig, SqsTrigger};
use axum::{
    extract::{Extension, Path},
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use uuid::Uuid;
use windmill_common::{db::UserDB, error, jobs::JobTriggerKind, triggers::TriggerMetadata};

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

    let trigger: TriggerForReassignment = match trigger_kind {
        JobTriggerKind::Websocket => {
            #[cfg(feature = "websocket")]
            {
                WebsocketTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(feature = "websocket"))]
            {
                return Err(error::Error::BadRequest(
                    "Websocket triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Http => {
            #[cfg(feature = "http_trigger")]
            {
                HttpTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(feature = "http_trigger"))]
            {
                return Err(error::Error::BadRequest(
                    "HTTP triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Mqtt => {
            #[cfg(feature = "mqtt_trigger")]
            {
                MqttTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(feature = "mqtt_trigger"))]
            {
                return Err(error::Error::BadRequest(
                    "MQTT triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Postgres => {
            #[cfg(feature = "postgres_trigger")]
            {
                PostgresTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(feature = "postgres_trigger"))]
            {
                return Err(error::Error::BadRequest(
                    "Postgres triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Kafka => {
            #[cfg(all(feature = "kafka", feature = "enterprise", feature = "private"))]
            {
                KafkaTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(all(feature = "kafka", feature = "enterprise", feature = "private")))]
            {
                return Err(error::Error::BadRequest(
                    "Kafka triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Email => {
            #[cfg(all(feature = "smtp", feature = "enterprise", feature = "private"))]
            {
                EmailTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(all(feature = "smtp", feature = "enterprise", feature = "private")))]
            {
                return Err(error::Error::BadRequest(
                    "Email triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Nats => {
            #[cfg(all(feature = "nats", feature = "enterprise", feature = "private"))]
            {
                NatsTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(all(feature = "nats", feature = "enterprise", feature = "private")))]
            {
                return Err(error::Error::BadRequest(
                    "NATS triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Sqs => {
            #[cfg(all(feature = "sqs_trigger", feature = "enterprise", feature = "private"))]
            {
                SqsTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(all(feature = "sqs_trigger", feature = "enterprise", feature = "private")))]
            {
                return Err(error::Error::BadRequest(
                    "SQS triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Gcp => {
            #[cfg(all(feature = "gcp_trigger", feature = "enterprise", feature = "private"))]
            {
                GcpTrigger
                    .get_trigger_for_reassignment(&mut *tx, &w_id, &trigger_path)
                    .await?
            }
            #[cfg(not(all(feature = "gcp_trigger", feature = "enterprise", feature = "private")))]
            {
                return Err(error::Error::BadRequest(
                    "GCP triggers are not enabled in this build".to_string(),
                ));
            }
        }
        JobTriggerKind::Webhook | JobTriggerKind::Schedule => {
            return Err(error::Error::BadRequest(
                "Webhook and Schedule triggers do not support job reassignment".to_string(),
            ));
        }
    };

    let jobs = if let Some(job_ids) = body.job_ids.as_ref() {
        if job_ids.is_empty() {
            vec![]
        } else {
            sqlx::query_as!(
                JobWithArgs,
                "SELECT id, args as \"args: _\", created_at FROM v2_job
                 WHERE workspace_id = $1
                   AND kind = 'unassigned'::JOB_KIND
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
        sqlx::query_as!(
            JobWithArgs,
            "SELECT id, args as \"args: _\", created_at FROM v2_job
             WHERE workspace_id = $1
               AND kind = 'unassigned'::JOB_KIND
               AND trigger_kind = $2
               AND trigger = $3",
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
                   AND kind = 'unassigned'::JOB_KIND
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
               AND kind = 'unassigned'::JOB_KIND
               AND trigger_kind = $2
               AND trigger = $3",
            w_id,
            trigger_kind as _,
            trigger_path,
        )
        .fetch_all(&mut *tx)
        .await?
    };

    let count = jobs_to_cancel.len();

    if count > 0 {
        // Use the cancel_jobs helper from windmill_queue
        for job_id in &jobs_to_cancel {
            let (returned_tx, _) = windmill_queue::cancel_job(
                &authed.username,
                Some("canceled by trigger management".to_string()),
                *job_id,
                &w_id,
                tx,
                &db,
                false,
                false,
            )
            .await?;
            tx = returned_tx;
        }
    }

    tx.commit().await?;

    Ok(Json(format!("Canceled {} jobs", count)))
}
