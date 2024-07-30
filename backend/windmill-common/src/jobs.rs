use std::collections::HashMap;

use bytes::Bytes;
use futures_core::Stream;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json, Pool, Postgres, Transaction};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub const ENTRYPOINT_OVERRIDE: &str = "_ENTRYPOINT_OVERRIDE";

use crate::{
    error::{self, to_anyhow, Error},
    flow_status::{FlowStatus, RestartedFrom},
    flows::{FlowValue, Retry},
    get_latest_deployed_hash_for_path,
    scripts::{ScriptHash, ScriptLang},
    worker::{to_raw_value, TMP_DIR},
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
    SingleScriptFlow,
    Identity,
    FlowDependencies,
    AppDependencies,
    Noop,
    DeploymentCallback,
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
    pub flow_status: Option<Json<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_flow: Option<Json<Box<RawValue>>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(skip)]
    pub self_wait_time_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(skip)]
    pub aggregate_wait_time_ms: Option<i64>,
}

impl QueuedJob {
    pub fn script_path(&self) -> &str {
        self.script_path
            .as_ref()
            .map(String::as_str)
            .unwrap_or("tmp/main")
    }
    pub fn is_flow(&self) -> bool {
        matches!(
            self.job_kind,
            JobKind::Flow | JobKind::FlowPreview | JobKind::SingleScriptFlow
        )
    }

    pub fn full_path_with_workspace(&self) -> String {
        format!(
            "{}/{}/{}",
            self.workspace_id,
            if self.is_flow() { "flow" } else { "script" },
            self.script_path()
        )
    }

    pub fn parse_raw_flow(&self) -> Option<FlowValue> {
        self.raw_flow.as_ref().and_then(|v| {
            let str = (**v).get();
            // tracing::error!("raw_flow: {}", str);
            return serde_json::from_str::<FlowValue>(str).ok();
        })
    }

    pub fn parse_flow_status(&self) -> Option<FlowStatus> {
        self.flow_status
            .as_ref()
            .and_then(|v| serde_json::from_str::<FlowStatus>((**v).get()).ok())
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
            priority: None,
            self_wait_time_ms: None,
            aggregate_wait_time_ms: None,
        }
    }
}

#[derive(Debug, sqlx::FromRow, Serialize, Clone)]
pub struct CompletedJob {
    pub workspace_id: String,
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_job: Option<Uuid>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: i64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<ScriptHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    pub args: Option<sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<sqlx::types::Json<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
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
    pub flow_status: Option<sqlx::types::Json<Box<RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_flow: Option<sqlx::types::Json<Box<RawValue>>>,
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
    #[sqlx(skip)]
    pub self_wait_time_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sqlx(skip)]
    pub aggregate_wait_time_ms: Option<i64>,
}

impl CompletedJob {
    pub fn json_result(&self) -> Option<serde_json::Value> {
        self.result
            .as_ref()
            .map(|r| serde_json::from_str(r.get()).ok())
            .flatten()
    }

    pub fn parse_raw_flow(&self) -> Option<FlowValue> {
        self.raw_flow
            .as_ref()
            .and_then(|v| serde_json::from_str::<FlowValue>((**v).get()).ok())
    }

    pub fn parse_flow_status(&self) -> Option<FlowStatus> {
        self.flow_status
            .as_ref()
            .and_then(|v| serde_json::from_str::<FlowStatus>((**v).get()).ok())
    }
}

#[derive(sqlx::FromRow)]
pub struct BranchResults {
    pub result: sqlx::types::Json<Box<RawValue>>,
    pub id: Uuid,
}

#[derive(Debug, Clone)]
pub enum JobPayload {
    ScriptHub {
        path: String,
    },
    ScriptHash {
        hash: ScriptHash,
        path: String,
        custom_concurrency_key: Option<String>,
        concurrent_limit: Option<i32>,
        concurrency_time_window_s: Option<i32>,
        cache_ttl: Option<i32>,
        dedicated_worker: Option<bool>,
        language: ScriptLang,
        priority: Option<i16>,
    },
    Code(RawCode),
    Dependencies {
        path: String,
        hash: ScriptHash,
        language: ScriptLang,
        dedicated_worker: Option<bool>,
    },
    FlowDependencies {
        path: String,
        dedicated_worker: Option<bool>,
        version: i64,
    },
    AppDependencies {
        path: String,
        version: i64,
    },
    RawFlowDependencies {
        path: String,
        flow_value: FlowValue,
    },
    RawScriptDependencies {
        script_path: String,
        content: String,
        language: ScriptLang,
    },
    Flow {
        path: String,
        dedicated_worker: Option<bool>,
    },
    RestartedFlow {
        completed_job_id: Uuid,
        step_id: String,
        branch_or_iteration_n: Option<usize>,
    },
    RawFlow {
        value: FlowValue,
        path: Option<String>,
        restarted_from: Option<RestartedFrom>,
    },
    SingleScriptFlow {
        path: String,
        hash: ScriptHash,
        args: HashMap<String, Box<serde_json::value::RawValue>>,
        retry: Retry, // for now only used to retry the script, so retry is necessarily present
        custom_concurrency_key: Option<String>,
        concurrent_limit: Option<i32>,
        concurrency_time_window_s: Option<i32>,
        cache_ttl: Option<i32>,
        priority: Option<i16>,
        tag_override: Option<String>,
    },
    DeploymentCallback {
        path: String,
    },
    Identity,
    Noop,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RawCode {
    pub content: String,
    pub path: Option<String>,
    pub hash: Option<i64>,
    pub language: ScriptLang,
    pub lock: Option<String>,
    pub custom_concurrency_key: Option<String>,
    pub concurrent_limit: Option<i32>,
    pub concurrency_time_window_s: Option<i32>,
    pub cache_ttl: Option<i32>,
    pub dedicated_worker: Option<bool>,
}

type Tag = String;

pub type DB = Pool<Postgres>;

pub async fn script_path_to_payload<'e, E: sqlx::Executor<'e, Database = Postgres>>(
    script_path: &str,
    db: E,
    w_id: &str,
) -> error::Result<(JobPayload, Option<Tag>, Option<bool>, Option<i32>)> {
    let (job_payload, tag, delete_after_use, script_timeout) = if script_path.starts_with("hub/") {
        (
            JobPayload::ScriptHub { path: script_path.to_owned() },
            None,
            None,
            None,
        )
    } else {
        let (
            script_hash,
            tag,
            custom_concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
            delete_after_use,
            script_timeout,
        ) = get_latest_deployed_hash_for_path(db, w_id, script_path).await?;
        (
            JobPayload::ScriptHash {
                hash: script_hash,
                path: script_path.to_owned(),
                custom_concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl: cache_ttl,
                language,
                dedicated_worker,
                priority,
            },
            tag,
            delete_after_use,
            script_timeout,
        )
    };
    Ok((job_payload, tag, delete_after_use, script_timeout))
}

pub async fn script_hash_to_tag_and_limits<'c>(
    script_hash: &ScriptHash,
    db: &mut Transaction<'c, Postgres>,
    w_id: &String,
) -> error::Result<(
    Option<Tag>,
    Option<String>,
    Option<i32>,
    Option<i32>,
    Option<i32>,
    ScriptLang,
    Option<bool>,
    Option<i16>,
    Option<bool>,
    Option<i32>,
)> {
    let script = sqlx::query!(
        "select tag, concurrency_key, concurrent_limit, concurrency_time_window_s, cache_ttl, language as \"language: ScriptLang\", dedicated_worker, priority, delete_after_use, timeout from script where hash = $1 AND workspace_id = $2",
        script_hash.0,
        w_id
    )
    .fetch_one(&mut **db)
    .await
    .map_err(|e| {
        Error::InternalErr(format!(
            "querying getting tag for hash {script_hash}: {e:#}"
        ))
    })?;
    Ok((
        script.tag,
        script.concurrency_key,
        script.concurrent_limit,
        script.concurrency_time_window_s,
        script.cache_ttl,
        script.language,
        script.dedicated_worker,
        script.priority,
        script.delete_after_use,
        script.timeout,
    ))
}

pub async fn get_payload_tag_from_prefixed_path<'e, E: sqlx::Executor<'e, Database = Postgres>>(
    path: &str,
    db: E,
    w_id: &str,
) -> Result<(JobPayload, Option<String>), Error> {
    let (payload, tag, _, _) = if path.starts_with("script/") {
        script_path_to_payload(path.strip_prefix("script/").unwrap(), db, w_id).await?
    } else if path.starts_with("flow/") {
        let path = path.strip_prefix("flow/").unwrap().to_string();
        let r = sqlx::query!(
            "SELECT tag, dedicated_worker from flow WHERE path = $1 and workspace_id = $2",
            &path,
            &w_id,
        )
        .fetch_optional(db)
        .await?;
        let (tag, dedicated_worker) = r
            .map(|x| (x.tag, x.dedicated_worker))
            .unwrap_or_else(|| (None, None));
        (JobPayload::Flow { path, dedicated_worker }, tag, None, None)
    } else {
        return Err(Error::BadRequest(format!(
            "path must start with script/ or flow/ (got {})",
            path
        )));
    };
    Ok((payload, tag))
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum FormattedResult {
    RawValue(Option<Box<RawValue>>),
    Vec(Vec<Box<RawValue>>),
}

#[derive(Serialize, Debug)]
pub struct CompletedJobWithFormattedResult {
    #[serde(flatten)]
    pub cj: CompletedJob,
    pub result: Option<FormattedResult>,
}

#[derive(Deserialize)]
struct FlowStatusMetadata {
    column_order: Vec<String>,
}

#[derive(Deserialize)]
struct FlowStatusWithMetadataOnly {
    _metadata: FlowStatusMetadata,
}

pub fn order_columns(
    rows: Option<Vec<Box<RawValue>>>,
    column_order: Vec<String>,
) -> Option<Vec<Box<RawValue>>> {
    if let Some(mut rows) = rows {
        if let Some(first_row) = rows.get(0) {
            let first_row = serde_json::from_str::<HashMap<String, Box<RawValue>>>(first_row.get());
            if let Ok(first_row) = first_row {
                let mut new_first_row = IndexMap::new();
                for col in column_order {
                    if let Some(val) = first_row.get(&col) {
                        new_first_row.insert(col.clone(), val.clone());
                    }
                }
                let new_row_as_raw_value = to_raw_value(&new_first_row);

                rows[0] = new_row_as_raw_value;

                return Some(rows);
            }
        }
    }

    None
}

pub fn format_result(
    language: Option<&ScriptLang>,
    flow_status: Option<Box<RawValue>>,
    result: Option<Box<RawValue>>,
) -> FormattedResult {
    match language {
        Some(&ScriptLang::Postgresql)
        | Some(&ScriptLang::Mysql)
        | Some(&ScriptLang::Snowflake)
        | Some(&ScriptLang::Bigquery) => {
            if let Some(Ok(flow_status)) =
                flow_status.map(|x| serde_json::from_str::<FlowStatusWithMetadataOnly>(x.get()))
            {
                if let Some(result) = result {
                    let rows = serde_json::from_str::<Vec<Box<RawValue>>>(result.get()).ok();
                    match order_columns(rows, flow_status._metadata.column_order) {
                        Some(rows) => return FormattedResult::Vec(rows),
                        None => return FormattedResult::RawValue(Some(result)),
                    }
                }
            }
        }
        _ => {}
    }

    FormattedResult::RawValue(result)
}

pub fn format_completed_job_result(mut cj: CompletedJob) -> CompletedJobWithFormattedResult {
    let sql_result = format_result(
        cj.language.as_ref(),
        cj.flow_status.clone().map(|x| x.0),
        cj.result.map(|x| x.0),
    );
    cj.result = None; // very important to avoid sending the result twice
    CompletedJobWithFormattedResult { cj, result: Some(sql_result) }
}

pub async fn get_logs_from_disk(
    log_offset: i32,
    logs: &str,
    log_file_index: &Option<Vec<String>>,
) -> Option<impl Stream<Item = Result<Bytes, anyhow::Error>>> {
    if log_offset > 0 {
        if let Some(file_index) = log_file_index.clone() {
            for file_p in &file_index {
                if !tokio::fs::metadata(format!("{TMP_DIR}/{file_p}"))
                    .await
                    .is_ok()
                {
                    return None;
                }
            }

            let logs = logs.to_string();
            let stream = async_stream::stream! {
                for file_p in file_index.clone() {
                    let mut file = tokio::fs::File::open(format!("{TMP_DIR}/{file_p}")).await.map_err(to_anyhow)?;
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).await.map_err(to_anyhow)?;
                    yield Ok(bytes::Bytes::from(buffer)) as anyhow::Result<bytes::Bytes>;
                }

                yield Ok(bytes::Bytes::from(logs))
            };
            return Some(stream);
        }
    }
    return None;
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
pub async fn get_logs_from_store(
    log_offset: i32,
    logs: &str,
    log_file_index: &Option<Vec<String>>,
) -> Option<impl Stream<Item = Result<Bytes, object_store::Error>>> {
    use crate::s3_helpers::OBJECT_STORE_CACHE_SETTINGS;

    if log_offset > 0 {
        if let Some(file_index) = log_file_index.clone() {
            tracing::debug!("Getting logs from store: {file_index:?}");
            if let Some(os) = OBJECT_STORE_CACHE_SETTINGS.read().await.clone() {
                tracing::debug!("object store client present, streaming from there");

                let logs = logs.to_string();
                let stream = async_stream::stream! {
                    for file_p in file_index.clone() {
                        let file_p_2 = file_p.clone();
                        let file = os.get(&object_store::path::Path::from(file_p)).await;
                        if let Ok(file) = file {
                            if let Ok(bytes) = file.bytes().await {
                                yield Ok(bytes::Bytes::from(bytes)) as object_store::Result<bytes::Bytes>;
                            }
                        } else {
                            tracing::debug!("error getting file from store: {file_p_2}: {}", file.err().unwrap());
                        }
                    }

                    yield Ok(bytes::Bytes::from(logs))
                };
                return Some(stream);
            } else {
                tracing::debug!("object store client not present, cannot stream logs from store");
            }
        }
    }
    return None;
}
