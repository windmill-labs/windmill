/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Implementation of `windmill_triggers::jobs_ext::JobOps` trait.
//! This bridges windmill-triggers back to windmill-api internals.

use uuid::Uuid;
use windmill_api_auth::{ApiAuthed, OptTokened};
use windmill_common::{
    db::UserDB,
    error,
    DB,
};
use windmill_triggers::jobs_ext::{JobOps, JobUpdateSSEStream};

pub struct JobOpsImpl;

#[axum::async_trait]
impl JobOps for JobOpsImpl {
    fn start_job_update_sse_stream(
        &self,
        opt_authed: Option<ApiAuthed>,
        opt_tokened: OptTokened,
        db: DB,
        w_id: String,
        job_id: Uuid,
        initial_log_offset: Option<i32>,
        initial_stream_offset: Option<i32>,
        get_progress: Option<bool>,
        running: Option<bool>,
        only_result: Option<bool>,
        fast: Option<bool>,
        no_logs: Option<bool>,
        is_flow: Option<bool>,
        tx: tokio::sync::mpsc::Sender<JobUpdateSSEStream>,
        poll_delay_ms: Option<u64>,
    ) {
        // Convert between windmill-api's SSE type and windmill-triggers' SSE type
        let (bridge_tx, mut bridge_rx) =
            tokio::sync::mpsc::channel::<crate::jobs::JobUpdateSSEStream>(32);

        crate::jobs::start_job_update_sse_stream(
            opt_authed,
            opt_tokened,
            db,
            w_id,
            job_id,
            initial_log_offset,
            initial_stream_offset,
            get_progress,
            running,
            only_result,
            fast,
            no_logs,
            is_flow,
            bridge_tx,
            poll_delay_ms,
        );

        // Spawn a task to convert between the two SSE stream types
        tokio::spawn(async move {
            while let Some(msg) = bridge_rx.recv().await {
                let converted = match msg {
                    crate::jobs::JobUpdateSSEStream::Update(update) => {
                        match serde_json::to_value(&update) {
                            Ok(v) => JobUpdateSSEStream::Update(v),
                            Err(e) => {
                                tracing::error!("Failed to serialize SSE job update: {e}");
                                continue;
                            }
                        }
                    }
                    crate::jobs::JobUpdateSSEStream::Error { error } => {
                        JobUpdateSSEStream::Error { error }
                    }
                    crate::jobs::JobUpdateSSEStream::NotFound => JobUpdateSSEStream::NotFound,
                    crate::jobs::JobUpdateSSEStream::Timeout => JobUpdateSSEStream::Timeout,
                    crate::jobs::JobUpdateSSEStream::Ping => JobUpdateSSEStream::Ping,
                };
                if tx.send(converted).await.is_err() {
                    break;
                }
            }
        });
    }

    async fn try_get_resource_from_db(
        &self,
        authed: &ApiAuthed,
        user_db: Option<UserDB>,
        db: &DB,
        resource_path: &str,
        w_id: &str,
    ) -> error::Result<serde_json::Value> {
        use windmill_common::db::DbWithOptAuthed;

        let db_with_authed = DbWithOptAuthed::from_authed(authed, db.clone(), user_db);
        let resource = crate::resources::get_resource_value_interpolated_internal(
            &db_with_authed,
            w_id,
            resource_path,
            None,
            None,
            false,
        )
        .await?;

        match resource {
            Some(v) => Ok(v),
            None => Err(error::Error::NotFound(format!(
                "resource at path :{} does not exist",
                resource_path
            ))),
        }
    }

    async fn interpolate(
        &self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
        s: String,
    ) -> Result<String, anyhow::Error> {
        #[cfg(all(
            feature = "enterprise",
            any(feature = "nats", feature = "kafka", feature = "sqs_trigger"),
        ))]
        {
            crate::ee_oss::interpolate(authed, db, w_id, s).await
        }
        #[cfg(not(all(
            feature = "enterprise",
            any(feature = "nats", feature = "kafka", feature = "sqs_trigger"),
        )))]
        {
            let _ = (authed, db, w_id);
            Ok(s)
        }
    }

    #[cfg(feature = "parquet")]
    async fn get_workspace_s3_resource(
        &self,
        authed: &ApiAuthed,
        db: &DB,
        user_db: Option<UserDB>,
        w_id: &str,
        storage: Option<String>,
    ) -> error::Result<(
        Option<bool>,
        Option<windmill_common::s3_helpers::ObjectStoreResource>,
    )> {
        crate::job_helpers_oss::get_workspace_s3_resource(authed, db, user_db, w_id, storage).await
    }
}
