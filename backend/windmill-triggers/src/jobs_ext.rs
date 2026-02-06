/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Job operations trait for windmill-triggers.
//!
//! Complex job functions that have deep dependencies on windmill-api internals
//! are abstracted behind the `JobOps` trait. windmill-api provides the real
//! implementation at startup via `set_ops()`.

use axum::response::Response;
use serde_json::value::RawValue;
use sqlx::Postgres;
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

// Re-export shared types/functions from windmill-common
pub use windmill_common::jobs::{delete_job_metadata_after_use, RunJobQuery};

#[cfg(feature = "enterprise")]
pub use windmill_common::ee_oss::check_license_key_valid;

// Re-export scope helpers from windmill-api-auth
pub use windmill_api_auth::scopes::{check_tag_available_for_workspace, get_scope_tags};

/// Trait for complex job operations that stay in windmill-api.
/// windmill-api provides the implementation at startup via `set_ops()`.
#[axum::async_trait]
pub trait JobOps: Send + Sync + 'static {
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
    )>;

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
    )>;

    async fn run_wait_result_internal(
        &self,
        db: &DB,
        uuid: Uuid,
        w_id: &str,
        node_id_for_empty_return: Option<String>,
        username: &str,
    ) -> error::Result<(Box<RawValue>, bool)>;

    fn result_to_response(
        &self,
        result: Box<RawValue>,
        success: bool,
    ) -> error::Result<Response>;

    async fn cancel_jobs(
        &self,
        jobs: Vec<Uuid>,
        db: &DB,
        username: &str,
        w_id: &str,
        force_cancel: bool,
    ) -> error::JsonResult<Vec<Uuid>>;

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
    );

    async fn try_get_resource_from_db(
        &self,
        authed: &ApiAuthed,
        user_db: Option<UserDB>,
        db: &DB,
        resource_path: &str,
        w_id: &str,
    ) -> error::Result<serde_json::Value>;

    async fn interpolate(
        &self,
        authed: &ApiAuthed,
        db: &DB,
        w_id: &str,
        s: String,
    ) -> Result<String, anyhow::Error>;

    #[cfg(feature = "parquet")]
    fn get_random_file_name(&self, ext: Option<String>) -> String;

    #[cfg(feature = "parquet")]
    async fn upload_file_internal(
        &self,
        s3_client: Arc<dyn object_store::ObjectStore>,
        file_key: &str,
        bytes_stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>>,
        options: object_store::PutMultipartOpts,
    ) -> error::Result<()>;

    #[cfg(feature = "parquet")]
    async fn get_workspace_s3_resource(
        &self,
        authed: &ApiAuthed,
        db: &DB,
        user_db: Option<UserDB>,
        w_id: &str,
        storage: Option<String>,
    ) -> error::Result<(Option<bool>, Option<windmill_common::s3_helpers::ObjectStoreResource>)>;
}

#[derive(Debug, serde::Serialize)]
pub enum JobUpdateSSEStream {
    Update(serde_json::Value),
    Error { error: String },
    NotFound,
    Timeout,
    Ping,
}

static JOB_OPS: std::sync::OnceLock<Arc<dyn JobOps>> = std::sync::OnceLock::new();

/// Call this from windmill-api at startup to inject the real implementation.
pub fn set_ops(ops: Arc<dyn JobOps>) {
    let _ = JOB_OPS.set(ops);
}

/// Get the job ops implementation. Panics if not initialized.
pub fn get_ops() -> &'static Arc<dyn JobOps> {
    JOB_OPS
        .get()
        .expect("JobOps not initialized. Call jobs_ext::set_ops() at startup.")
}

// Convenience wrapper functions that delegate to the trait

pub async fn push_script_job_by_path_into_queue(
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
    get_ops()
        .push_script_job_by_path_into_queue(
            authed,
            db,
            tx_o,
            user_db,
            w_id,
            script_path,
            run_query,
            args,
            trigger,
        )
        .await
}

pub async fn push_flow_job_by_path_into_queue(
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
    get_ops()
        .push_flow_job_by_path_into_queue(
            authed,
            db,
            tx_o,
            user_db,
            w_id,
            flow_path,
            run_query,
            args,
            trigger,
        )
        .await
}

pub async fn run_wait_result_internal(
    db: &DB,
    uuid: Uuid,
    w_id: &str,
    node_id_for_empty_return: Option<String>,
    username: &str,
) -> error::Result<(Box<RawValue>, bool)> {
    get_ops()
        .run_wait_result_internal(db, uuid, w_id, node_id_for_empty_return, username)
        .await
}

pub fn result_to_response(result: Box<RawValue>, success: bool) -> error::Result<Response> {
    get_ops().result_to_response(result, success)
}

pub async fn cancel_jobs(
    jobs: Vec<Uuid>,
    db: &DB,
    username: &str,
    w_id: &str,
    force_cancel: bool,
) -> error::JsonResult<Vec<Uuid>> {
    get_ops()
        .cancel_jobs(jobs, db, username, w_id, force_cancel)
        .await
}

pub fn start_job_update_sse_stream(
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
    get_ops().start_job_update_sse_stream(
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
        tx,
        poll_delay_ms,
    )
}

pub async fn interpolate(
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
    s: String,
) -> Result<String, anyhow::Error> {
    get_ops().interpolate(authed, db, w_id, s).await
}

#[cfg(feature = "parquet")]
pub async fn get_workspace_s3_resource(
    authed: &ApiAuthed,
    db: &DB,
    user_db: Option<UserDB>,
    w_id: &str,
    storage: Option<String>,
) -> error::Result<(Option<bool>, Option<windmill_common::s3_helpers::ObjectStoreResource>)> {
    get_ops()
        .get_workspace_s3_resource(authed, db, user_db, w_id, storage)
        .await
}
