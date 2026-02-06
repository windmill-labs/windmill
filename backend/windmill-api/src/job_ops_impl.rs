/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Implementation of `windmill_triggers::jobs_ext::JobOps` trait.
//! This bridges windmill-triggers back to windmill-api internals.

use axum::response::Response;
use serde_json::value::RawValue;
use sqlx::Postgres;
#[cfg(feature = "parquet")]
use std::sync::Arc;
use uuid::Uuid;
use windmill_api_auth::{ApiAuthed, OptTokened};
use windmill_common::{
    db::UserDB,
    error,
    triggers::TriggerMetadata,
    utils::StripPath,
    DB,
};
use windmill_queue::PushArgsOwned;
use windmill_triggers::jobs_ext::{JobOps, JobUpdateSSEStream, RunJobQuery};

pub struct JobOpsImpl;

#[axum::async_trait]
impl JobOps for JobOpsImpl {
    async fn push_script_job_by_path_into_queue(
        &self,
        authed: ApiAuthed,
        db: DB,
        tx_o: Option<sqlx::Transaction<'static, Postgres>>,
        user_db: UserDB,
        w_id: String,
        script_path: StripPath,
        run_query: RunJobQuery,
        args: PushArgsOwned,
        trigger: Option<TriggerMetadata>,
    ) -> error::Result<(
        Uuid,
        Option<bool>,
        Option<sqlx::Transaction<'static, Postgres>>,
    )> {
        // Convert RunJobQuery from windmill-triggers to the windmill-api version
        let api_run_query = crate::jobs::RunJobQuery {
            scheduled_for: run_query.scheduled_for,
            scheduled_in_secs: run_query.scheduled_in_secs,
            parent_job: run_query.parent_job,
            root_job: run_query.root_job,
            invisible_to_owner: run_query.invisible_to_owner,
            queue_limit: run_query.queue_limit,
            payload: run_query.payload,
            job_id: run_query.job_id,
            tag: run_query.tag,
            timeout: run_query.timeout,
            cache_ttl: run_query.cache_ttl,
            cache_ignore_s3_path: run_query.cache_ignore_s3_path,
            skip_preprocessor: run_query.skip_preprocessor,
            poll_delay_ms: run_query.poll_delay_ms,
            memory_id: run_query.memory_id,
            trigger_external_id: run_query.trigger_external_id,
            service_name: run_query.service_name,
            suspended_mode: run_query.suspended_mode,
        };
        crate::jobs::push_script_job_by_path_into_queue(
            authed, db, tx_o, user_db, w_id, script_path, api_run_query, args, trigger,
        )
        .await
    }

    async fn push_flow_job_by_path_into_queue(
        &self,
        authed: ApiAuthed,
        db: DB,
        tx_o: Option<sqlx::Transaction<'static, Postgres>>,
        user_db: UserDB,
        w_id: String,
        flow_path: StripPath,
        run_query: RunJobQuery,
        args: PushArgsOwned,
        trigger: Option<TriggerMetadata>,
    ) -> error::Result<(
        Uuid,
        Option<String>,
        Option<sqlx::Transaction<'static, Postgres>>,
    )> {
        let api_run_query = crate::jobs::RunJobQuery {
            scheduled_for: run_query.scheduled_for,
            scheduled_in_secs: run_query.scheduled_in_secs,
            parent_job: run_query.parent_job,
            root_job: run_query.root_job,
            invisible_to_owner: run_query.invisible_to_owner,
            queue_limit: run_query.queue_limit,
            payload: run_query.payload,
            job_id: run_query.job_id,
            tag: run_query.tag,
            timeout: run_query.timeout,
            cache_ttl: run_query.cache_ttl,
            cache_ignore_s3_path: run_query.cache_ignore_s3_path,
            skip_preprocessor: run_query.skip_preprocessor,
            poll_delay_ms: run_query.poll_delay_ms,
            memory_id: run_query.memory_id,
            trigger_external_id: run_query.trigger_external_id,
            service_name: run_query.service_name,
            suspended_mode: run_query.suspended_mode,
        };
        crate::jobs::push_flow_job_by_path_into_queue(
            authed, db, tx_o, user_db, w_id, flow_path, api_run_query, args, trigger,
        )
        .await
    }

    async fn run_wait_result_internal(
        &self,
        db: &DB,
        uuid: Uuid,
        w_id: &str,
        node_id_for_empty_return: Option<String>,
        username: &str,
    ) -> error::Result<(Box<RawValue>, bool)> {
        crate::jobs::run_wait_result_internal(db, uuid, w_id, node_id_for_empty_return, username)
            .await
    }

    fn result_to_response(
        &self,
        result: Box<RawValue>,
        success: bool,
    ) -> error::Result<Response> {
        crate::jobs::result_to_response(result, success)
    }

    async fn cancel_jobs(
        &self,
        jobs: Vec<Uuid>,
        db: &DB,
        username: &str,
        w_id: &str,
        force_cancel: bool,
    ) -> error::JsonResult<Vec<Uuid>> {
        crate::jobs::cancel_jobs(jobs, db, username, w_id, force_cancel).await
    }

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
                        JobUpdateSSEStream::Update(
                            serde_json::to_value(&update).unwrap_or_default(),
                        )
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
    fn get_random_file_name(&self, ext: Option<String>) -> String {
        crate::job_helpers_oss::get_random_file_name(ext)
    }

    #[cfg(feature = "parquet")]
    async fn upload_file_internal(
        &self,
        s3_client: Arc<dyn object_store::ObjectStore>,
        file_key: &str,
        bytes_stream: std::pin::Pin<
            Box<dyn futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>,
        >,
        options: object_store::PutMultipartOpts,
    ) -> error::Result<()> {
        crate::job_helpers_oss::upload_file_internal(s3_client, file_key, bytes_stream, options)
            .await
            .map(|_| ())
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
