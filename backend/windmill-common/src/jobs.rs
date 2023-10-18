use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    error::{self, Error},
    flow_status::FlowStatus,
    flows::FlowValue,
    get_latest_deployed_hash_for_path,
    scripts::{ScriptHash, ScriptLang},
};

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[sqlx(type_name = "JOB_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase"))]
pub enum JobKind {
    Script,
    #[allow(non_camel_case_types)]
    Script_Hub,
    Preview,
    Dependencies,
    Flow,
    FlowPreview,
    Identity,
    FlowDependencies,
    AppDependencies,
    Noop,
}

#[derive(sqlx::FromRow, Debug, Serialize, Clone)]
pub struct QueuedJob {
    pub workspace_id: String,
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<ScriptHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    pub args: Option<Json<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_lock: Option<String>,
    pub canceled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_ping: Option<chrono::DateTime<chrono::Utc>>,
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
    pub same_worker: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_run_error: Option<String>,
    pub email: String,
    pub visible_to_owner: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspend: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_peak: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_job: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leaf_jobs: Option<serde_json::Value>,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrent_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_time_window_s: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow_step_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<i32>,
}

impl QueuedJob {
    pub fn get_args(&self) -> HashMap<String, Box<RawValue>> {
        if let Some(args) = self.args.as_ref() {
            args.0.clone()
        } else {
            HashMap::new()
        }
    }

    pub fn script_path(&self) -> &str {
        self.script_path
            .as_ref()
            .map(String::as_str)
            .unwrap_or("tmp/main")
    }
    pub fn is_flow(&self) -> bool {
        matches!(self.job_kind, JobKind::Flow | JobKind::FlowPreview)
    }

    pub fn full_path(&self) -> String {
        format!(
            "{}/{}",
            if self.is_flow() { "flow" } else { "script" },
            self.script_path()
        )
    }

    pub fn parse_raw_flow(&self) -> Option<FlowValue> {
        self.raw_flow
            .as_ref()
            .and_then(|v| serde_json::from_value::<FlowValue>(v.clone()).ok())
    }

    pub fn parse_flow_status(&self) -> Option<FlowStatus> {
        self.flow_status
            .as_ref()
            .and_then(|v| serde_json::from_value::<FlowStatus>(v.clone()).ok())
    }
}

impl Default for QueuedJob {
    fn default() -> Self {
        Self {
            workspace_id: "".to_string(),
            id: Uuid::default(),
            parent_job: None,
            created_by: "".to_string(),
            created_at: chrono::Utc::now(),
            started_at: None,
            scheduled_for: chrono::Utc::now(),
            running: false,
            script_hash: None,
            script_path: None,
            args: None,
            logs: None,
            raw_code: None,
            raw_lock: None,
            canceled: false,
            canceled_by: None,
            canceled_reason: None,
            last_ping: None,
            job_kind: JobKind::Identity,
            schedule_path: None,
            permissioned_as: "".to_string(),
            flow_status: None,
            raw_flow: None,
            is_flow_step: false,
            language: None,
            same_worker: false,
            pre_run_error: None,
            email: "".to_string(),
            visible_to_owner: false,
            suspend: None,
            mem_peak: None,
            root_job: None,
            leaf_jobs: None,
            tag: "deno".to_string(),
            concurrent_limit: None,
            concurrency_time_window_s: None,
            timeout: None,
            flow_step_id: None,
            cache_ttl: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum JobPayload {
    ScriptHub {
        path: String,
    },
    ScriptHash {
        hash: ScriptHash,
        path: String,
        concurrent_limit: Option<i32>,
        concurrency_time_window_s: Option<i32>,
        cache_ttl: Option<i32>,
        dedicated_worker: Option<bool>,
        language: ScriptLang,
    },
    Code(RawCode),
    Dependencies {
        path: String,
        hash: ScriptHash,
        dependencies: String,
        language: ScriptLang,
    },
    FlowDependencies {
        path: String,
    },
    AppDependencies {
        path: String,
        version: i64,
    },
    Flow(String),
    RawFlow {
        value: FlowValue,
        path: Option<String>,
    },
    Identity,
    Noop,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RawCode {
    pub content: String,
    pub path: Option<String>,
    pub language: ScriptLang,
    pub lock: Option<String>,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub cache_ttl: Option<i32>,
}

type Tag = String;

pub type DB = Pool<Postgres>;

pub async fn script_path_to_payload(
    script_path: &str,
    db: &DB,
    w_id: &str,
) -> error::Result<(JobPayload, Option<Tag>)> {
    let (job_payload, tag) = if script_path.starts_with("hub/") {
        (JobPayload::ScriptHub { path: script_path.to_owned() }, None)
    } else {
        let (
            script_hash,
            tag,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
        ) = get_latest_deployed_hash_for_path(db, w_id, script_path).await?;
        (
            JobPayload::ScriptHash {
                hash: script_hash,
                path: script_path.to_owned(),
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl: cache_ttl,
                language,
                dedicated_worker,
            },
            tag,
        )
    };
    Ok((job_payload, tag))
}

pub async fn script_hash_to_tag_and_limits<'c>(
    script_hash: &ScriptHash,
    db: &mut Transaction<'c, Postgres>,
    w_id: &String,
) -> error::Result<(
    Option<Tag>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    ScriptLang,
    Option<bool>,
)> {
    let script = sqlx::query!(
        "select tag, concurrent_limit, concurrency_time_window_s, cache_ttl, language as \"language: ScriptLang\", dedicated_worker from script where hash = $1 AND workspace_id = $2",
        script_hash.0,
        w_id
    )
    .fetch_one(&mut **db)
    .await
    .map_err(|e| {
        Error::InternalErr(format!(
            "querying getting tag for hash {script_hash}: {e}"
        ))
    })?;
    Ok((
        script.tag,
        script.concurrent_limit,
        script.concurrency_time_window_s,
        script.cache_ttl,
        script.language,
        script.dedicated_worker,
    ))
}

pub async fn get_payload_tag_from_prefixed_path(
    path: &str,
    db: &DB,
    w_id: &str,
) -> Result<(JobPayload, Option<String>), Error> {
    let (payload, tag) = if path.starts_with("script/") {
        script_path_to_payload(path.strip_prefix("script/").unwrap(), &db, w_id).await?
    } else if path.starts_with("flow/") {
        (
            JobPayload::Flow(path.strip_prefix("flow/").unwrap().to_string()),
            None,
        )
    } else {
        return Err(Error::BadRequest(format!(
            "path must start with script/ or flow/ (got {})",
            path
        )));
    };
    Ok((payload, tag))
}

#[derive(Clone)]
pub struct Metrics {
    pub worker_execution_failed: prometheus::IntCounter,
}
