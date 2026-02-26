/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Shared types for the jobs subsystem.

use axum::{
    extract::{FromRequest, Json, Query, Request},
    response::{IntoResponse, Response},
};
use http::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::HashMap;
use uuid::Uuid;
use windmill_common::{
    error,
    jobs::{CompletedJob, JobKind, JobTriggerKind, QueuedJob},
    scripts::{ScriptHash, ScriptLang},
    utils::now_from_db,
    DB,
};

use windmill_api_sse::{Job, JobExtended};

use crate::negated_filter::NegatedListFilter;

// ------------ RunJobQuery ------------

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RunJobQuery {
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_in_secs: Option<i64>,
    pub parent_job: Option<Uuid>,
    pub root_job: Option<Uuid>,
    pub invisible_to_owner: Option<bool>,
    pub queue_limit: Option<i64>,
    pub payload: Option<String>,
    pub job_id: Option<Uuid>,
    pub tag: Option<String>,
    pub timeout: Option<i32>,
    pub cache_ttl: Option<i32>,
    pub cache_ignore_s3_path: Option<bool>,
    pub skip_preprocessor: Option<bool>,
    pub poll_delay_ms: Option<u64>,
    pub memory_id: Option<Uuid>,
    pub trigger_external_id: Option<String>,
    pub service_name: Option<String>,
    pub suspended_mode: Option<bool>,
}

impl RunJobQuery {
    pub async fn get_scheduled_for(
        &self,
        db: &DB,
    ) -> error::Result<Option<chrono::DateTime<chrono::Utc>>> {
        if let Some(scheduled_for) = self.scheduled_for {
            Ok(Some(scheduled_for))
        } else if let Some(scheduled_in_secs) = self.scheduled_in_secs {
            let now = now_from_db(db).await?;
            Ok(Some(
                now + chrono::Duration::try_seconds(scheduled_in_secs).unwrap_or_default(),
            ))
        } else {
            Ok(None)
        }
    }

    pub fn payload_as_args(&self) -> error::Result<HashMap<String, Box<RawValue>>> {
        let payload_r = self.payload.clone().map(decode_payload).map(|x| {
            x.map_err(|e| {
                error::Error::internal_err(format!("Impossible to decode query payload: {e:#?}"))
            })
        });

        let payload_as_args = if let Some(payload) = payload_r {
            payload?
        } else {
            HashMap::new()
        };

        Ok(payload_as_args)
    }
}

// ------------ List query types ------------

#[derive(Deserialize, Clone)]
pub struct ListQueueQuery {
    pub script_path_start: Option<NegatedListFilter<String>>,
    pub script_path_exact: Option<NegatedListFilter<String>>,
    pub script_hash: Option<String>,
    pub created_by: Option<NegatedListFilter<String>>,
    pub started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub running: Option<bool>,
    pub schedule_path: Option<String>,
    pub parent_job: Option<String>,
    pub order_desc: Option<bool>,
    pub job_kinds: Option<NegatedListFilter<String>>,
    pub suspended: Option<bool>,
    pub worker: Option<NegatedListFilter<String>>,
    // filter by matching a subset of the args using base64 encoded json subset
    pub args: Option<String>,
    pub tag: Option<NegatedListFilter<String>>,
    pub scheduled_for_before_now: Option<bool>,
    pub all_workspaces: Option<bool>,
    pub is_flow_step: Option<bool>,
    pub has_null_parent: Option<bool>,
    pub is_not_schedule: Option<bool>,
    pub concurrency_key: Option<String>,
    pub allow_wildcards: Option<bool>,
    pub trigger_kind: Option<NegatedListFilter<JobTriggerKind>>,
    pub trigger_path: Option<NegatedListFilter<String>>,
    pub include_args: Option<bool>,
    pub broad_filter: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct ListCompletedQuery {
    pub script_path_start: Option<NegatedListFilter<String>>,
    pub script_path_exact: Option<NegatedListFilter<String>>,
    pub script_hash: Option<String>,
    pub created_by: Option<NegatedListFilter<String>>,
    pub started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_before: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_or_started_after_completed_jobs: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before_queue: Option<chrono::DateTime<chrono::Utc>>,
    pub created_after_queue: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_after: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_before: Option<chrono::DateTime<chrono::Utc>>,
    pub success: Option<bool>,
    pub running: Option<bool>,
    pub parent_job: Option<String>,
    pub order_desc: Option<bool>,
    pub job_kinds: Option<NegatedListFilter<String>>,
    pub is_skipped: Option<bool>,
    pub is_flow_step: Option<bool>,
    pub suspended: Option<bool>,
    pub schedule_path: Option<String>,
    // filter by matching a subset of the args using base64 encoded json subset
    pub args: Option<String>,
    // filter by matching a subset of the result using base64 encoded json subset
    pub result: Option<String>,
    pub tag: Option<NegatedListFilter<String>>,
    pub scheduled_for_before_now: Option<bool>,
    pub all_workspaces: Option<bool>,
    pub has_null_parent: Option<bool>,
    pub label: Option<NegatedListFilter<String>>,
    pub is_not_schedule: Option<bool>,
    pub concurrency_key: Option<String>,
    pub worker: Option<NegatedListFilter<String>>,
    pub allow_wildcards: Option<bool>,
    pub trigger_kind: Option<NegatedListFilter<JobTriggerKind>>,
    pub trigger_path: Option<NegatedListFilter<String>>,
    pub include_args: Option<bool>,
    pub broad_filter: Option<String>,
}

impl From<ListCompletedQuery> for ListQueueQuery {
    fn from(lcq: ListCompletedQuery) -> Self {
        Self {
            script_path_start: lcq.script_path_start,
            script_path_exact: lcq.script_path_exact,
            script_hash: lcq.script_hash,
            created_by: lcq.created_by,
            started_before: lcq.started_before,
            started_after: lcq.started_after,
            created_before: lcq.created_before_queue.or(lcq.created_before),
            created_after: lcq.created_after_queue.or(lcq.created_after),
            created_or_started_before: lcq.created_or_started_before,
            created_or_started_after: lcq.created_or_started_after,
            worker: lcq.worker,
            running: lcq.running,
            parent_job: lcq.parent_job,
            order_desc: lcq.order_desc,
            job_kinds: lcq.job_kinds,
            suspended: lcq.suspended,
            args: lcq.args,
            tag: lcq.tag,
            schedule_path: lcq.schedule_path,
            scheduled_for_before_now: lcq.scheduled_for_before_now,
            all_workspaces: lcq.all_workspaces,
            is_flow_step: lcq.is_flow_step,
            has_null_parent: lcq.has_null_parent,
            is_not_schedule: lcq.is_not_schedule,
            concurrency_key: lcq.concurrency_key,
            allow_wildcards: lcq.allow_wildcards,
            trigger_kind: lcq.trigger_kind,
            trigger_path: lcq.trigger_path,
            include_args: lcq.include_args,
            broad_filter: lcq.broad_filter,
        }
    }
}

// ------------ ListableCompletedJob ------------

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ListableCompletedJob {
    pub r#type: String,
    pub workspace_id: String,
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: i64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<ScriptHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_code: Option<String>,
    pub canceled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_reason: Option<String>,
    pub job_kind: JobKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_path: Option<String>,
    pub permissioned_as: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_status: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_flow: Option<serde_json::Value>,
    pub is_flow_step: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<ScriptLang>,
    pub is_skipped: bool,
    pub email: String,
    pub visible_to_owner: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_peak: Option<i32>,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

// ------------ UnifiedJob ------------

#[derive(sqlx::FromRow)]
pub struct UnifiedJob {
    pub workspace_id: String,
    pub typ: String,
    pub id: Uuid,
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
    pub running: Option<bool>,
    pub script_hash: Option<ScriptHash>,
    pub script_path: Option<String>,
    pub args: Option<serde_json::Value>,
    pub duration_ms: Option<i64>,
    pub success: Option<bool>,
    pub deleted: bool,
    pub canceled: bool,
    pub canceled_by: Option<String>,
    pub job_kind: JobKind,
    pub schedule_path: Option<String>,
    pub permissioned_as: String,
    pub is_flow_step: bool,
    pub language: Option<ScriptLang>,
    pub is_skipped: bool,
    pub email: String,
    pub visible_to_owner: bool,
    pub suspend: Option<i32>,
    pub mem_peak: Option<i32>,
    pub tag: String,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub priority: Option<i16>,
    pub labels: Option<serde_json::Value>,
    pub self_wait_time_ms: Option<i64>,
    pub aggregate_wait_time_ms: Option<i64>,
    pub preprocessed: Option<bool>,
    pub worker: Option<String>,
    pub runnable_settings_handle: Option<i64>,
}

const CJ_FIELDS: &[&str] = &[
    "'CompletedJob' as typ",
    "v2_job_completed.id",
    "v2_job_completed.workspace_id",
    "v2_job.parent_job",
    "v2_job.created_by",
    "v2_job.created_at",
    "v2_job_completed.started_at",
    "null as scheduled_for",
    "v2_job_completed.completed_at",
    "null as running",
    "v2_job.runnable_id as script_hash",
    "v2_job.runnable_path as script_path",
    "null as args",
    "v2_job_completed.duration_ms",
    "v2_job_completed.status = 'success' OR v2_job_completed.status = 'skipped' as success",
    "false as deleted",
    "v2_job_completed.status = 'canceled' as canceled",
    "v2_job_completed.canceled_by",
    "v2_job.kind as job_kind",
    "CASE WHEN v2_job.trigger_kind = 'schedule' THEN v2_job.trigger END as schedule_path",
    "v2_job.permissioned_as",
    "v2_job.flow_step_id IS NOT NULL as is_flow_step",
    "v2_job.script_lang as language",
    "v2_job_completed.status = 'skipped' as is_skipped",
    "v2_job.permissioned_as_email as email",
    "v2_job.visible_to_owner",
    "null as suspend",
    "v2_job_completed.memory_peak as mem_peak",
    "v2_job.tag",
    "null as concurrent_limit",
    "null as concurrency_time_window_s",
    "v2_job.priority",
    "v2_job_completed.result->'wm_labels' as labels",
    "self_wait_time_ms",
    "aggregate_wait_time_ms",
    "v2_job.preprocessed",
    "v2_job_completed.worker",
    "null as runnable_settings_handle",
];

const QJ_FIELDS: &[&str] = &[
    "'QueuedJob' as typ",
    "v2_job_queue.id",
    "v2_job_queue.workspace_id",
    "v2_job.parent_job",
    "v2_job.created_by",
    "v2_job_queue.created_at",
    "v2_job_queue.started_at",
    "v2_job_queue.scheduled_for",
    "null as completed_at",
    "v2_job_queue.running",
    "v2_job.runnable_id as script_hash",
    "v2_job.runnable_path as script_path",
    "null as args",
    "null as duration_ms",
    "null as success",
    "false as deleted",
    "v2_job_queue.canceled_by IS NOT NULL as canceled",
    "v2_job_queue.canceled_by",
    "v2_job.kind as job_kind",
    "CASE WHEN v2_job.trigger_kind = 'schedule' THEN v2_job.trigger END as schedule_path",
    "v2_job.permissioned_as",
    "v2_job.flow_step_id IS NOT NULL as is_flow_step",
    "v2_job.script_lang as language",
    "false as is_skipped",
    "v2_job.permissioned_as_email as email",
    "v2_job.visible_to_owner",
    "v2_job_queue.suspend",
    "null as mem_peak",
    "v2_job.tag",
    "v2_job.concurrent_limit",
    "v2_job.concurrency_time_window_s",
    "v2_job.priority",
    "null as labels",
    "self_wait_time_ms",
    "aggregate_wait_time_ms",
    "v2_job.preprocessed",
    "v2_job_queue.worker",
    "v2_job_queue.runnable_settings_handle",
];

impl UnifiedJob {
    pub fn completed_job_fields() -> &'static [&'static str] {
        CJ_FIELDS
    }
    pub fn queued_job_fields() -> &'static [&'static str] {
        QJ_FIELDS
    }
}

impl From<UnifiedJob> for Job {
    fn from(uj: UnifiedJob) -> Self {
        let args = uj.args.and_then(|v| serde_json::from_value(v).ok());
        match uj.typ.as_ref() {
            "CompletedJob" => Job::CompletedJob(JobExtended::new(
                uj.self_wait_time_ms,
                uj.aggregate_wait_time_ms,
                CompletedJob {
                    workspace_id: uj.workspace_id,
                    id: uj.id,
                    parent_job: uj.parent_job,
                    created_by: uj.created_by,
                    created_at: uj.created_at,
                    started_at: uj.started_at,
                    completed_at: uj.completed_at,
                    duration_ms: uj.duration_ms.unwrap(),
                    success: uj.success.unwrap(),
                    script_hash: uj.script_hash,
                    script_path: uj.script_path,
                    args: args.clone(),
                    result: None,
                    result_columns: None,
                    logs: None,
                    flow_status: None,
                    workflow_as_code_status: None,
                    deleted: uj.deleted,
                    canceled: uj.canceled,
                    canceled_by: uj.canceled_by,
                    canceled_reason: None,
                    job_kind: uj.job_kind,
                    schedule_path: uj.schedule_path,
                    permissioned_as: uj.permissioned_as,
                    is_flow_step: uj.is_flow_step,
                    language: uj.language,
                    is_skipped: uj.is_skipped,
                    email: uj.email,
                    visible_to_owner: uj.visible_to_owner,
                    mem_peak: uj.mem_peak,
                    tag: uj.tag,
                    priority: uj.priority,
                    labels: uj.labels,
                    preprocessed: uj.preprocessed,
                },
            )),
            "QueuedJob" => Job::QueuedJob(JobExtended::new(
                uj.self_wait_time_ms,
                uj.aggregate_wait_time_ms,
                QueuedJob {
                    workspace_id: uj.workspace_id,
                    id: uj.id,
                    parent_job: uj.parent_job,
                    created_by: uj.created_by,
                    created_at: uj.created_at,
                    started_at: uj.started_at,
                    scheduled_for: uj.scheduled_for.unwrap(),
                    running: uj.running.unwrap(),
                    script_hash: uj.script_hash,
                    script_path: uj.script_path,
                    script_entrypoint_override: None,
                    args,
                    logs: None,
                    canceled: uj.canceled,
                    canceled_by: uj.canceled_by,
                    canceled_reason: None,
                    last_ping: None,
                    job_kind: uj.job_kind,
                    schedule_path: uj.schedule_path,
                    permissioned_as: uj.permissioned_as,
                    flow_status: None,
                    workflow_as_code_status: None,
                    is_flow_step: uj.is_flow_step,
                    language: uj.language,
                    same_worker: false,
                    pre_run_error: None,
                    email: uj.email,
                    visible_to_owner: uj.visible_to_owner,
                    suspend: uj.suspend,
                    mem_peak: uj.mem_peak,
                    root_job: None,
                    leaf_jobs: None,
                    tag: uj.tag,
                    concurrent_limit: uj.concurrent_limit,
                    concurrency_time_window_s: uj.concurrency_time_window_s,
                    timeout: None,
                    flow_step_id: None,
                    cache_ttl: None,
                    cache_ignore_s3_path: None,
                    priority: uj.priority,
                    preprocessed: uj.preprocessed,
                    runnable_settings_handle: uj.runnable_settings_handle,
                },
            )),
            t => panic!("job type {} not valid", t),
        }
    }
}

// ------------ Approval types ------------

#[derive(Deserialize, Debug)]
pub struct QueryApprover {
    pub approver: Option<String>,
    /// If true, generate/verify resume URLs for the parent flow instead of the specific step.
    /// This allows pre-approvals that can be consumed by any later suspend step in the same flow.
    pub flow_level: Option<bool>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Debug)]
pub struct ResumeUrls {
    pub approvalPage: String,
    pub cancel: String,
    pub resume: String,
}

// ------------ QueryOrBody extractor ------------

pub struct QueryOrBody<D>(pub Option<D>);

#[axum::async_trait]
impl<S, D> FromRequest<S, axum::body::Body> for QueryOrBody<D>
where
    D: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(
        req: Request<axum::body::Body>,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        return if req.method() == axum::http::Method::GET {
            let Query(InPayload { payload }) = Query::from_request(req, state)
                .await
                .map_err(IntoResponse::into_response)?;
            payload
                .map(|p| {
                    decode_payload(p)
                        .map(QueryOrBody)
                        .map_err(|err| (StatusCode::BAD_REQUEST, format!("{err:#?}")))
                        .map_err(IntoResponse::into_response)
                })
                .unwrap_or(Ok(QueryOrBody(None)))
        } else {
            Json::from_request(req, state)
                .await
                .map(|Json(v)| QueryOrBody(Some(v)))
                .map_err(IntoResponse::into_response)
        };

        #[derive(Deserialize)]
        struct InPayload {
            payload: Option<String>,
        }
    }
}

// ------------ Utility functions ------------

pub fn decode_payload<D: DeserializeOwned>(t: String) -> anyhow::Result<D> {
    let vec = base64::engine::general_purpose::STANDARD
        .decode(t)
        .context("invalid base64")?;
    serde_json::from_slice(vec.as_slice()).context("invalid json")
}

pub fn add_raw_string(
    raw_string: Option<String>,
    mut args: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    if let Some(raw_string) = raw_string {
        args.insert(
            "raw_string".to_string(),
            serde_json::Value::String(raw_string),
        );
    }
    return args;
}

use anyhow::Context;
use base64::Engine;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- decode_payload ---

    #[test]
    fn test_decode_payload_valid() {
        let payload = base64::engine::general_purpose::STANDARD.encode(r#"{"key": "value"}"#);
        let result: HashMap<String, serde_json::Value> = decode_payload(payload).unwrap();
        assert_eq!(result["key"], json!("value"));
    }

    #[test]
    fn test_decode_payload_invalid_base64() {
        let result: anyhow::Result<HashMap<String, serde_json::Value>> =
            decode_payload("not-valid-base64!!!".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_payload_invalid_json() {
        let payload = base64::engine::general_purpose::STANDARD.encode("not json");
        let result: anyhow::Result<HashMap<String, serde_json::Value>> = decode_payload(payload);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_payload_empty_object() {
        let payload = base64::engine::general_purpose::STANDARD.encode("{}");
        let result: HashMap<String, serde_json::Value> = decode_payload(payload).unwrap();
        assert!(result.is_empty());
    }

    // --- add_raw_string ---

    #[test]
    fn test_add_raw_string_some() {
        let args = serde_json::Map::new();
        let result = add_raw_string(Some("body content".to_string()), args);
        assert_eq!(
            result["raw_string"],
            serde_json::Value::String("body content".to_string())
        );
    }

    #[test]
    fn test_add_raw_string_none() {
        let args = serde_json::Map::new();
        let result = add_raw_string(None, args);
        assert!(!result.contains_key("raw_string"));
    }

    #[test]
    fn test_add_raw_string_preserves_existing() {
        let mut args = serde_json::Map::new();
        args.insert("existing".to_string(), json!("value"));
        let result = add_raw_string(Some("body".to_string()), args);
        assert_eq!(result["existing"], json!("value"));
        assert_eq!(result["raw_string"], json!("body"));
    }

    // --- RunJobQuery ---

    #[test]
    fn test_run_job_query_payload_as_args_none() {
        let q = RunJobQuery::default();
        let result = q.payload_as_args().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_run_job_query_payload_as_args_valid() {
        let encoded = base64::engine::general_purpose::STANDARD.encode(r#"{"x": 42}"#);
        let q = RunJobQuery { payload: Some(encoded), ..Default::default() };
        let result = q.payload_as_args().unwrap();
        assert!(result.contains_key("x"));
    }

    #[test]
    fn test_run_job_query_payload_as_args_invalid() {
        let q = RunJobQuery { payload: Some("invalid!!!".to_string()), ..Default::default() };
        assert!(q.payload_as_args().is_err());
    }

    // --- ListCompletedQuery -> ListQueueQuery conversion ---

    #[test]
    fn test_list_completed_to_queue_query_conversion() {
        let lcq = ListCompletedQuery {
            script_path_start: Some(NegatedListFilter::positive(vec!["f/test".to_string()])),
            script_path_exact: None,
            script_hash: None,
            created_by: Some(NegatedListFilter::positive(vec!["admin".to_string()])),
            started_before: None,
            started_after: None,
            created_before: Some(chrono::Utc::now()),
            created_after: None,
            created_or_started_before: None,
            created_or_started_after: None,
            created_or_started_after_completed_jobs: None,
            created_before_queue: None,
            created_after_queue: None,
            completed_after: None,
            completed_before: None,
            success: None,
            running: Some(true),
            parent_job: None,
            order_desc: Some(true),
            job_kinds: Some(NegatedListFilter::positive(vec![
                "script".to_string(),
                "flow".to_string(),
            ])),
            is_skipped: None,
            is_flow_step: None,
            suspended: None,
            schedule_path: None,
            args: None,
            result: None,
            tag: Some(NegatedListFilter::positive(vec!["custom".to_string()])),
            scheduled_for_before_now: None,
            all_workspaces: None,
            has_null_parent: None,
            label: None,
            is_not_schedule: None,
            concurrency_key: None,
            worker: None,
            allow_wildcards: None,
            trigger_kind: None,
            trigger_path: None,
            include_args: None,
            broad_filter: None,
        };

        let lqq: ListQueueQuery = lcq.into();
        assert_eq!(
            lqq.script_path_start
                .as_ref()
                .and_then(|f| f.values.first().cloned()),
            Some("f/test".to_string())
        );
        assert_eq!(
            lqq.created_by
                .as_ref()
                .and_then(|f| f.values.first().cloned()),
            Some("admin".to_string())
        );
        assert_eq!(lqq.running, Some(true));
        assert_eq!(lqq.job_kinds.as_ref().map(|f| f.values.len()), Some(2));
        assert_eq!(
            lqq.tag.as_ref().and_then(|f| f.values.first().cloned()),
            Some("custom".to_string())
        );
    }

    #[test]
    fn test_list_completed_to_queue_prefers_queue_created_fields() {
        let specific_time = chrono::Utc::now();
        let other_time = specific_time - chrono::Duration::hours(1);

        let lcq = ListCompletedQuery {
            script_path_start: None,
            script_path_exact: None,
            script_hash: None,
            created_by: None,
            started_before: None,
            started_after: None,
            created_before: Some(other_time),
            created_after: Some(other_time),
            created_or_started_before: None,
            created_or_started_after: None,
            created_or_started_after_completed_jobs: None,
            created_before_queue: Some(specific_time),
            created_after_queue: Some(specific_time),
            completed_after: None,
            completed_before: None,
            success: None,
            running: None,
            parent_job: None,
            order_desc: None,
            job_kinds: None,
            is_skipped: None,
            is_flow_step: None,
            suspended: None,
            schedule_path: None,
            args: None,
            result: None,
            tag: None,
            scheduled_for_before_now: None,
            all_workspaces: None,
            has_null_parent: None,
            label: None,
            is_not_schedule: None,
            concurrency_key: None,
            worker: None,
            allow_wildcards: None,
            trigger_kind: None,
            trigger_path: None,
            include_args: None,
            broad_filter: None,
        };

        let lqq: ListQueueQuery = lcq.into();
        assert_eq!(lqq.created_before, Some(specific_time));
        assert_eq!(lqq.created_after, Some(specific_time));
    }

    // --- UnifiedJob field constants ---

    #[test]
    fn test_completed_job_fields_not_empty() {
        let fields = UnifiedJob::completed_job_fields();
        assert!(!fields.is_empty());
        assert!(fields.iter().any(|f| f.contains("typ")));
        assert!(fields.iter().any(|f| f.contains("workspace_id")));
    }

    #[test]
    fn test_queued_job_fields_not_empty() {
        let fields = UnifiedJob::queued_job_fields();
        assert!(!fields.is_empty());
        assert!(fields.iter().any(|f| f.contains("typ")));
        assert!(fields.iter().any(|f| f.contains("scheduled_for")));
    }

    #[test]
    fn test_completed_and_queued_fields_same_count() {
        assert_eq!(
            UnifiedJob::completed_job_fields().len(),
            UnifiedJob::queued_job_fields().len(),
            "CJ and QJ field lists must have the same number of columns for UNION queries"
        );
    }

    // --- ListableCompletedJob serialization ---

    #[test]
    fn test_listable_completed_job_skip_none() {
        let job = ListableCompletedJob {
            r#type: "CompletedJob".to_string(),
            workspace_id: "test".to_string(),
            id: Uuid::nil(),
            parent_job: None,
            created_by: "admin".to_string(),
            created_at: chrono::Utc::now(),
            started_at: None,
            duration_ms: 100,
            success: true,
            script_hash: None,
            script_path: None,
            deleted: false,
            raw_code: None,
            canceled: false,
            canceled_by: None,
            canceled_reason: None,
            job_kind: JobKind::Script,
            schedule_path: None,
            permissioned_as: "u/admin".to_string(),
            flow_status: None,
            raw_flow: None,
            is_flow_step: false,
            language: None,
            is_skipped: false,
            email: "admin@test.com".to_string(),
            visible_to_owner: true,
            mem_peak: None,
            tag: "default".to_string(),
            priority: None,
            labels: None,
            args: None,
        };

        let json = serde_json::to_value(&job).unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("parent_job"));
        assert!(!obj.contains_key("script_hash"));
        assert!(!obj.contains_key("raw_code"));
        assert!(!obj.contains_key("canceled_by"));
        assert!(!obj.contains_key("mem_peak"));
        assert!(!obj.contains_key("labels"));
        assert!(obj.contains_key("type"));
        assert!(obj.contains_key("workspace_id"));
    }
}
