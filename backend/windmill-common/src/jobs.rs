use std::collections::HashMap;

use bytes::Bytes;
use futures_core::Stream;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::{types::Json, Pool, Postgres};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

pub const ENTRYPOINT_OVERRIDE: &str = "_ENTRYPOINT_OVERRIDE";
pub const LARGE_LOG_THRESHOLD_SIZE: usize = 9000;

pub const EMAIL_ERROR_HANDLER_USER_EMAIL: &str = "email_error_handler@windmill.dev";

use crate::{
    apps::AppScriptId,
    auth::is_super_admin_email,
    error::{self, to_anyhow, Error},
    flow_status::{FlowStatus, RestartedFrom},
    flows::{FlowNodeId, FlowValue, Retry},
    get_latest_deployed_hash_for_path, get_latest_flow_version_info_for_path,
    scripts::{get_full_hub_script_by_path, ScriptHash, ScriptLang},
    users::username_to_permissioned_as,
    utils::{StripPath, HTTP_CLIENT},
    worker::{to_raw_value, CUSTOM_TAGS_PER_WORKSPACE, TMP_DIR},
    FlowVersionInfo, ScriptHashInfo,
};

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone)]
#[sqlx(type_name = "JOB_TRIGGER_KIND", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum JobTriggerKind {
    Webhook,
    Http,
    Websocket,
    Kafka,
    Email,
    Nats,
    Mqtt,
    Sqs,
    Postgres,
    Schedule,
    Gcp,
}

impl std::fmt::Display for JobTriggerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            JobTriggerKind::Webhook => "webhook",
            JobTriggerKind::Http => "http",
            JobTriggerKind::Websocket => "websocket",
            JobTriggerKind::Kafka => "kafka",
            JobTriggerKind::Email => "email",
            JobTriggerKind::Nats => "nats",
            JobTriggerKind::Mqtt => "mqtt",
            JobTriggerKind::Sqs => "sqs",
            JobTriggerKind::Postgres => "postgres",
            JobTriggerKind::Schedule => "schedule",
            JobTriggerKind::Gcp => "gcp",
        };
        write!(f, "{}", kind)
    }
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
#[sqlx(type_name = "JOB_KIND", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
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
    FlowScript,
    FlowNode,
    AppScript,
}

impl JobKind {
    pub fn is_flow(&self) -> bool {
        matches!(
            self,
            JobKind::Flow | JobKind::FlowPreview | JobKind::SingleScriptFlow | JobKind::FlowNode
        )
    }

    pub fn is_dependency(&self) -> bool {
        matches!(
            self,
            JobKind::FlowDependencies | JobKind::AppDependencies | JobKind::Dependencies
        )
    }
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
    pub script_entrypoint_override: Option<String>,
    pub args: Option<Json<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
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
    pub workflow_as_code_status: Option<Json<Box<RawValue>>>,
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
    pub preprocessed: Option<bool>,
}

impl QueuedJob {
    pub fn script_path(&self) -> &str {
        self.script_path
            .as_ref()
            .map(String::as_str)
            .unwrap_or("tmp/main")
    }
    pub fn is_flow(&self) -> bool {
        self.job_kind.is_flow()
    }

    pub fn full_path_with_workspace(&self) -> String {
        format!(
            "{}/{}/{}",
            self.workspace_id,
            if self.is_flow() { "flow" } else { "script" },
            self.script_path()
        )
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
            canceled: false,
            canceled_by: None,
            canceled_reason: None,
            last_ping: None,
            job_kind: JobKind::Identity,
            schedule_path: None,
            permissioned_as: "".to_string(),
            workflow_as_code_status: None,
            flow_status: None,
            is_flow_step: false,
            language: None,
            script_entrypoint_override: None,
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
            preprocessed: None,
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
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: i64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<ScriptHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    pub args: Option<sqlx::types::Json<HashMap<String, Box<RawValue>>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<sqlx::types::Json<Box<RawValue>>>,
    pub result_columns: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs: Option<String>,
    pub deleted: bool,
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
    pub workflow_as_code_status: Option<sqlx::types::Json<Box<RawValue>>>,
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
    pub preprocessed: Option<bool>,
}

impl CompletedJob {
    pub fn json_result(&self) -> Option<serde_json::Value> {
        self.result
            .as_ref()
            .map(|r| serde_json::from_str(r.get()).ok())
            .flatten()
    }

    pub fn parse_flow_status(&self) -> Option<FlowStatus> {
        self.flow_status
            .as_ref()
            .and_then(|v| serde_json::from_str::<FlowStatus>((**v).get()).ok())
    }
}

#[derive(Debug, Clone)]
pub enum JobPayload {
    ScriptHub {
        path: String,
        apply_preprocessor: bool,
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
        apply_preprocessor: bool,
    },
    FlowScript {
        id: FlowNodeId, // flow_node(id).
        language: ScriptLang,
        custom_concurrency_key: Option<String>,
        concurrent_limit: Option<i32>,
        concurrency_time_window_s: Option<i32>,
        cache_ttl: Option<i32>,
        dedicated_worker: Option<bool>,
        path: String,
    },
    FlowNode {
        id: FlowNodeId, // flow_node(id).
        path: String,   // flow node inner path (e.g. `outer/branchall-42`).
    },
    AppScript {
        id: AppScriptId, // app_script(id).
        path: Option<String>,
        language: ScriptLang,
        cache_ttl: Option<i32>,
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
        apply_preprocessor: bool,
        version: i64,
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
        retry: Option<Retry>,
        error_handler_path: Option<String>,
        error_handler_args: Option<HashMap<String, Box<RawValue>>>,
        custom_concurrency_key: Option<String>,
        concurrent_limit: Option<i32>,
        concurrency_time_window_s: Option<i32>,
        cache_ttl: Option<i32>,
        priority: Option<i16>,
        tag_override: Option<String>,
        trigger_path: Option<String>,
        apply_preprocessor: bool,
    },
    DeploymentCallback {
        path: String,
    },
    Identity,
    Noop,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
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

#[derive(Clone, Debug)]
pub struct OnBehalfOf {
    pub email: String,
    pub permissioned_as: String,
}

pub fn get_has_preprocessor_from_content_and_lang(
    content: &str,
    language: &ScriptLang,
) -> error::Result<bool> {
    let has_preprocessor = match language {
        ScriptLang::Bun | ScriptLang::Bunnative | ScriptLang::Deno | ScriptLang::Nativets => {
            let args = windmill_parser_ts::parse_deno_signature(&content, true, true, None)?;
            args.has_preprocessor.unwrap_or(false)
        }
        ScriptLang::Python3 => {
            let args = windmill_parser_py::parse_python_signature(&content, None, true)?;
            args.has_preprocessor.unwrap_or(false)
        }
        _ => false,
    };

    Ok(has_preprocessor)
}

pub async fn script_path_to_payload<'e, A: sqlx::Acquire<'e, Database = Postgres> + Send>(
    script_path: &str,
    db: A,
    w_id: &str,
    skip_preprocessor: Option<bool>,
) -> error::Result<(
    JobPayload,
    Option<Tag>,
    Option<bool>,
    Option<i32>,
    Option<OnBehalfOf>,
)> {
    let (job_payload, tag, delete_after_use, script_timeout, on_behalf_of) = if script_path
        .starts_with("hub/")
    {
        let hub_script =
            get_full_hub_script_by_path(StripPath(script_path.to_string()), &HTTP_CLIENT, None)
                .await?;

        let has_preprocessor =
            get_has_preprocessor_from_content_and_lang(&hub_script.content, &hub_script.language)?;

        (
            JobPayload::ScriptHub {
                path: script_path.to_owned(),
                apply_preprocessor: has_preprocessor && !skip_preprocessor.unwrap_or(false),
            },
            None,
            None,
            None,
            None,
        )
    } else {
        let ScriptHashInfo {
            hash,
            tag,
            concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            cache_ttl,
            language,
            dedicated_worker,
            priority,
            delete_after_use,
            timeout,
            has_preprocessor,
            on_behalf_of_email,
            created_by,
            ..
        } = get_latest_deployed_hash_for_path(db, w_id, script_path).await?;

        let on_behalf_of = if let Some(email) = on_behalf_of_email {
            Some(OnBehalfOf {
                email,
                permissioned_as: username_to_permissioned_as(created_by.as_str()),
            })
        } else {
            None
        };

        (
            JobPayload::ScriptHash {
                hash: ScriptHash(hash),
                path: script_path.to_owned(),
                custom_concurrency_key: concurrency_key,
                concurrent_limit,
                concurrency_time_window_s,
                cache_ttl: cache_ttl,
                language,
                dedicated_worker,
                priority,
                apply_preprocessor: !skip_preprocessor.unwrap_or(false)
                    && has_preprocessor.unwrap_or(false),
            },
            tag,
            delete_after_use,
            timeout,
            on_behalf_of,
        )
    };
    Ok((
        job_payload,
        tag,
        delete_after_use,
        script_timeout,
        on_behalf_of,
    ))
}

pub async fn get_payload_tag_from_prefixed_path(
    path: &str,
    db: &DB,
    w_id: &str,
) -> Result<(JobPayload, Option<String>, Option<OnBehalfOf>), Error> {
    let (payload, tag, _, _, on_behalf_of) = if path.starts_with("script/") {
        script_path_to_payload(path.strip_prefix("script/").unwrap(), db, w_id, Some(true)).await?
    } else if path.starts_with("flow/") {
        let path = path.strip_prefix("flow/").unwrap().to_string();
        let FlowVersionInfo { dedicated_worker, tag, version, .. } =
            get_latest_flow_version_info_for_path(db, w_id, &path, true).await?;
        (
            JobPayload::Flow { path, dedicated_worker, apply_preprocessor: false, version },
            tag,
            None,
            None,
            None,
        )
    } else {
        return Err(Error::BadRequest(format!(
            "path must start with script/ or flow/ (got {})",
            path
        )));
    };
    Ok((payload, tag, on_behalf_of))
}

pub fn order_columns(
    rows: Option<Vec<Box<RawValue>>>,
    column_order: &Vec<String>,
) -> Option<Box<RawValue>> {
    if let Some(mut rows) = rows {
        if let Some(first_row) = rows.get(0) {
            let first_row = serde_json::from_str::<HashMap<String, Box<RawValue>>>(first_row.get());
            if let Ok(first_row) = first_row {
                let mut new_first_row = IndexMap::new();
                for col in column_order {
                    if let Some(val) = first_row.get(col) {
                        new_first_row.insert(col.clone(), val.clone());
                    }
                }
                let new_row_as_raw_value = to_raw_value(&new_first_row);

                rows[0] = new_row_as_raw_value;

                return Some(to_raw_value(&rows));
            }
        }
    }

    None
}

pub fn format_result(
    result_columns: Option<&Vec<String>>,
    result: Option<&mut sqlx::types::Json<Box<RawValue>>>,
) -> () {
    if let Some(result_columns) = result_columns {
        if let Some(result) = result {
            let rows = serde_json::from_str::<Vec<Box<RawValue>>>(result.get()).ok();
            if let Some(ordered_result) = order_columns(rows, result_columns) {
                *result = sqlx::types::Json(ordered_result);
            }
        }
    }
}

pub fn format_completed_job_result(mut cj: CompletedJob) -> CompletedJob {
    format_result(cj.result_columns.as_ref(), cj.result.as_mut());
    cj
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
    use crate::s3_helpers::get_object_store;

    if log_offset > 0 {
        if let Some(file_index) = log_file_index.clone() {
            if let Some(os) = get_object_store().await {
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

pub async fn check_tag_available_for_workspace_internal(
    db: &DB,
    w_id: &str,
    tag: &str,
    email: &str,
    scope_tags: Option<Vec<&str>>,
) -> error::Result<()> {
    let mut is_tag_in_scope_tags = None;
    let mut is_tag_in_workspace_custom_tags = false;

    if let Some(scope_tags) = scope_tags.as_ref() {
        is_tag_in_scope_tags = Some(scope_tags.contains(&tag));
    }

    let custom_tags_per_w = CUSTOM_TAGS_PER_WORKSPACE.read().await;
    if custom_tags_per_w.global.contains(&tag.to_string()) {
        is_tag_in_workspace_custom_tags = true;
    } else if let Some(specific_tag) = custom_tags_per_w.specific.get(tag) {
        is_tag_in_workspace_custom_tags = specific_tag.applies_to_workspace(w_id);
    }

    match is_tag_in_scope_tags {
        Some(true) | None => {
            if is_tag_in_workspace_custom_tags {
                return Ok(());
            }
        }
        _ => {}
    }

    if !is_super_admin_email(db, email).await? {
        if scope_tags.is_some() && is_tag_in_scope_tags.is_some() {
            return Err(Error::BadRequest(format!(
                "Tag {tag} is not available in your scope"
            )));
        }

        return Err(error::Error::BadRequest(format!(
            "Only super admins are allowed to use tags that are not included in the allowed CUSTOM_TAGS: {:?}",
            custom_tags_per_w
        )));
    }

    return Ok(());
}
