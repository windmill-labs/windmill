/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Shared SSE types for job update streaming.
//!
//! This crate contains the types used by the SSE job update system,
//! separated out so they can be depended on without pulling in the
//! full windmill-api-jobs crate.

use serde::Serialize;
use serde_json::value::RawValue;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Deref, DerefMut};
use uuid::Uuid;
use windmill_common::flow_status::FlowStatus;
use windmill_common::jobs::{CompletedJob, JobKind, QueuedJob};

// ------------ Job types (from jobs.rs) ------------

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct JobExtended<T: JobCommon> {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub inner: T,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_lock: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_flow: Option<sqlx::types::Json<Box<RawValue>>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker: Option<String>,

    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_wait_time_ms: Option<i64>,
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate_wait_time_ms: Option<i64>,
}

pub trait JobCommon {
    fn job_kind(&self) -> &JobKind;
}

impl JobCommon for QueuedJob {
    fn job_kind(&self) -> &JobKind {
        &self.job_kind
    }
}

impl JobCommon for CompletedJob {
    fn job_kind(&self) -> &JobKind {
        &self.job_kind
    }
}

impl<T: JobCommon> JobExtended<T> {
    pub fn new(
        self_wait_time_ms: Option<i64>,
        aggregate_wait_time_ms: Option<i64>,
        inner: T,
    ) -> Self {
        Self {
            inner,
            raw_code: None,
            raw_lock: None,
            raw_flow: None,
            worker: None,
            self_wait_time_ms,
            aggregate_wait_time_ms,
        }
    }
}

impl<T: JobCommon> Deref for JobExtended<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: JobCommon> DerefMut for JobExtended<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum Job {
    QueuedJob(JobExtended<QueuedJob>),
    CompletedJob(JobExtended<CompletedJob>),
}

impl Job {
    pub fn created_by(&self) -> &str {
        match self {
            Job::QueuedJob(job) => &job.created_by,
            Job::CompletedJob(job) => &job.created_by,
        }
    }

    pub fn append_to_logs(&mut self, logs: &str) {
        match self {
            Job::QueuedJob(job) => {
                if let Some(ref mut l) = job.logs {
                    l.push_str(logs);
                } else {
                    job.logs = Some(logs.to_string());
                }
            }
            Job::CompletedJob(job) => {
                if let Some(ref mut l) = job.logs {
                    l.push_str(logs);
                } else {
                    job.logs = Some(logs.to_string());
                }
            }
        }
    }

    pub fn log_len(&self) -> Option<usize> {
        match self {
            Job::QueuedJob(job) => job.logs.as_ref().map(|l| l.len()),
            Job::CompletedJob(job) => job.logs.as_ref().map(|l| l.len()),
        }
    }

    pub fn logs(&self) -> Option<String> {
        match self {
            Job::QueuedJob(job) => job.logs.clone(),
            Job::CompletedJob(job) => job.logs.clone(),
        }
    }

    pub fn flow_status(&self) -> Option<FlowStatus> {
        match self {
            Job::QueuedJob(job) => job
                .flow_status
                .as_ref()
                .and_then(|rf| serde_json::from_str(rf.0.get()).ok()),
            Job::CompletedJob(job) => job
                .flow_status
                .as_ref()
                .and_then(|rf| serde_json::from_str(rf.0.get()).ok()),
        }
    }

    pub fn is_flow_step(&self) -> bool {
        match self {
            Job::QueuedJob(job) => job.is_flow_step,
            Job::CompletedJob(job) => job.is_flow_step,
        }
    }

    pub fn is_flow(&self) -> bool {
        self.job_kind().is_flow()
    }

    pub fn job_kind(&self) -> &JobKind {
        match self {
            Job::QueuedJob(job) => &job.job_kind,
            Job::CompletedJob(job) => &job.job_kind,
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            Job::QueuedJob(job) => job.id,
            Job::CompletedJob(job) => job.id,
        }
    }

    pub fn workspace_id(&self) -> &String {
        match self {
            Job::QueuedJob(job) => &job.workspace_id,
            Job::CompletedJob(job) => &job.workspace_id,
        }
    }

    pub fn script_path(&self) -> &str {
        match self {
            Job::QueuedJob(job) => job.script_path.as_ref(),
            Job::CompletedJob(job) => job.script_path.as_ref(),
        }
        .map(String::as_str)
        .unwrap_or("tmp/main")
    }

    pub fn args(&self) -> Option<&sqlx::types::Json<HashMap<String, Box<RawValue>>>> {
        match self {
            Job::QueuedJob(job) => job.args.as_ref(),
            Job::CompletedJob(job) => job.args.as_ref(),
        }
    }

    pub fn full_path_with_workspace(&self) -> String {
        format!(
            "{}/{}/{}",
            self.workspace_id(),
            if self.is_flow() { "flow" } else { "script" },
            self.script_path(),
        )
    }

    pub async fn concurrency_key(
        &self,
        db: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar!(
            "SELECT key FROM concurrency_key WHERE job_id = $1",
            self.id()
        )
        .fetch_optional(db)
        .await
    }

    pub async fn fetch_outstanding_wait_time(
        &mut self,
        db: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<(), sqlx::Error> {
        let r = sqlx::query!(
            "SELECT self_wait_time_ms, aggregate_wait_time_ms FROM outstanding_wait_time WHERE job_id = $1",
            self.id()
        )
        .fetch_optional(db)
        .await?;

        let (self_wait_time, aggregate_wait_time) = r
            .map(|x| (x.self_wait_time_ms, x.aggregate_wait_time_ms))
            .unwrap_or((None, None));

        match self {
            Job::QueuedJob(job) => {
                job.self_wait_time_ms = self_wait_time;
                job.aggregate_wait_time_ms = aggregate_wait_time;
            }
            Job::CompletedJob(job) => {
                job.self_wait_time_ms = self_wait_time;
                job.aggregate_wait_time_ms = aggregate_wait_time;
            }
        }
        Ok(())
    }
}

// ------------ SSE types ------------

#[derive(Serialize, Debug)]
pub struct JobUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_logs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_result_stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_offset: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_offset: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_peak: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_status: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_as_code_status: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<Job>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_result: Option<Box<RawValue>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_stream_job_id: Option<Uuid>,
}

impl JobUpdate {
    pub fn hash_str(&self) -> String {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl Hash for JobUpdate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.running.hash(state);
        self.completed.hash(state);
        self.log_offset.hash(state);
        self.mem_peak.hash(state);
        self.progress.hash(state);
        self.stream_offset.hash(state);
        self.flow_stream_job_id.hash(state);
        if !self.completed.unwrap_or(false) {
            self.flow_status.as_ref().map(|x| x.get().hash(state));
            self.workflow_as_code_status
                .as_ref()
                .map(|x| x.get().hash(state));
        }
    }
}

#[derive(serde::Deserialize)]
pub struct JobUpdateQuery {
    pub running: Option<bool>,
    pub log_offset: Option<i32>,
    pub stream_offset: Option<i32>,
    pub get_progress: Option<bool>,
    pub no_logs: Option<bool>,
    pub only_result: Option<bool>,
    pub fast: Option<bool>,
    pub is_flow: Option<bool>,
    pub poll_delay_ms: Option<u64>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum JobUpdateSSEStream {
    Update(JobUpdate),
    Error { error: String },
    NotFound,
    Timeout,
    Ping,
}

lazy_static::lazy_static! {
    pub static ref TIMEOUT_SSE_STREAM: u64 =
        std::env::var("TIMEOUT_SSE_STREAM").unwrap_or("60".to_string()).parse::<u64>().unwrap_or(60);
}
