use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sqlx::types::Json;
use uuid::Uuid;

use crate::{
    apps::AppScriptId,
    flow_status::{FlowStatus, RestartedFrom},
    flows::{FlowNodeId, FlowValue, Retry},
    runnable_settings::{ConcurrencySettings, ConcurrencySettingsWithCustom, DebouncingSettings},
    scripts::{ScriptHash, ScriptLang},
};

#[derive(Debug, Deserialize, Clone)]
pub struct DynamicInput {
    #[serde(rename = "x-windmill-dyn-select-code")]
    pub x_windmill_dyn_select_code: String,
    #[serde(rename = "x-windmill-dyn-select-lang")]
    pub x_windmill_dyn_select_lang: ScriptLang,
}

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
    Nextcloud,
    Google,
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
            JobTriggerKind::Nextcloud => "nextcloud",
            JobTriggerKind::Google => "google",
        };
        write!(f, "{}", kind)
    }
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Default)]
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
    SingleStepFlow,
    Identity,
    FlowDependencies,
    AppDependencies,
    #[default]
    Noop,
    DeploymentCallback,
    FlowScript,
    FlowNode,
    AppScript,
    AIAgent,
    #[serde(rename = "unassigned_script")]
    #[sqlx(rename = "unassigned_script")]
    UnassignedScript,
    #[serde(rename = "unassigned_flow")]
    #[sqlx(rename = "unassigned_flow")]
    UnassignedFlow,
    #[serde(rename = "unassigned_singlestepflow")]
    #[sqlx(rename = "unassigned_singlestepflow")]
    UnassignedSinglestepFlow,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Copy, Clone)]
#[sqlx(type_name = "JOB_STATUS", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum JobStatus {
    Success,
    Failure,
    Canceled,
    Skipped,
}

impl JobKind {
    pub fn is_flow(&self) -> bool {
        matches!(
            self,
            JobKind::Flow | JobKind::FlowPreview | JobKind::SingleStepFlow | JobKind::FlowNode
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
    pub cache_ignore_s3_path: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preprocessed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runnable_settings_handle: Option<i64>,
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
            cache_ignore_s3_path: None,
            priority: None,
            preprocessed: None,
            runnable_settings_handle: None,
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
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
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
        cache_ttl: Option<i32>,
        cache_ignore_s3_path: Option<bool>,
        dedicated_worker: Option<bool>,
        language: ScriptLang,
        priority: Option<i16>,
        apply_preprocessor: bool,
        concurrency_settings: ConcurrencySettings,
        debouncing_settings: DebouncingSettings,
    },
    FlowNode {
        id: FlowNodeId,
        path: String,
    },
    FlowScript {
        id: FlowNodeId,
        path: String,
        language: ScriptLang,
        cache_ttl: Option<i32>,
        cache_ignore_s3_path: Option<bool>,
        dedicated_worker: Option<bool>,
        concurrency_settings: ConcurrencySettings,
    },
    AppScript {
        id: AppScriptId,
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
        debouncing_settings: DebouncingSettings,
    },
    FlowDependencies {
        path: String,
        dedicated_worker: Option<bool>,
        version: i64,
        debouncing_settings: DebouncingSettings,
    },
    AppDependencies {
        path: String,
        version: i64,
        debouncing_settings: DebouncingSettings,
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
        flow_version: Option<i64>,
    },
    RawFlow {
        value: FlowValue,
        path: Option<String>,
        restarted_from: Option<RestartedFrom>,
    },
    SingleStepFlow {
        path: String,
        hash: Option<ScriptHash>,
        flow_version: Option<i64>,
        args: HashMap<String, Box<serde_json::value::RawValue>>,
        retry: Option<Retry>,
        error_handler_path: Option<String>,
        error_handler_args: Option<HashMap<String, Box<RawValue>>>,
        skip_handler: Option<SkipHandler>,
        cache_ttl: Option<i32>,
        cache_ignore_s3_path: Option<bool>,
        priority: Option<i16>,
        tag_override: Option<String>,
        trigger_path: Option<String>,
        apply_preprocessor: bool,
        concurrency_settings: ConcurrencySettings,
        debouncing_settings: DebouncingSettings,
    },
    DeploymentCallback {
        path: String,
        debouncing_settings: DebouncingSettings,
    },
    Identity,
    Noop,
    AIAgent {
        path: String,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SkipHandler {
    pub path: String,
    pub args: HashMap<String, Box<RawValue>>,
    pub stop_condition: String,
    pub stop_message: String,
}

#[derive(Clone, Deserialize, Debug, Default)]
pub struct RawCode {
    pub content: String,
    pub path: Option<String>,
    pub hash: Option<i64>,
    pub language: ScriptLang,
    pub lock: Option<String>,
    pub cache_ttl: Option<i32>,
    pub cache_ignore_s3_path: Option<bool>,
    pub dedicated_worker: Option<bool>,
    #[serde(flatten)]
    pub concurrency_settings: ConcurrencySettingsWithCustom,
    #[serde(flatten)]
    pub debouncing_settings: DebouncingSettings,
}

impl JobPayload {
    pub fn job_kind(&self) -> JobKind {
        match self {
            JobPayload::Noop => JobKind::Noop,
            JobPayload::Identity => JobKind::Identity,
            JobPayload::Code { .. } => JobKind::Preview,
            JobPayload::AIAgent { .. } => JobKind::AIAgent,
            JobPayload::FlowNode { .. } => JobKind::FlowNode,
            JobPayload::ScriptHash { .. } => JobKind::Script,
            JobPayload::AppScript { .. } => JobKind::AppScript,
            JobPayload::RawFlow { .. } => JobKind::FlowPreview,
            JobPayload::ScriptHub { .. } => JobKind::Script_Hub,
            JobPayload::FlowScript { .. } => JobKind::FlowScript,
            JobPayload::Dependencies { .. } => JobKind::Dependencies,
            JobPayload::SingleStepFlow { .. } => JobKind::SingleStepFlow,
            JobPayload::AppDependencies { .. } => JobKind::AppDependencies,
            JobPayload::FlowDependencies { .. } => JobKind::FlowDependencies,
            JobPayload::RawScriptDependencies { .. } => JobKind::Dependencies,
            JobPayload::RawFlowDependencies { .. } => JobKind::FlowDependencies,
            JobPayload::DeploymentCallback { .. } => JobKind::DeploymentCallback,
            JobPayload::Flow { .. } | JobPayload::RestartedFlow { .. } => JobKind::Flow,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OnBehalfOf {
    pub email: String,
    pub permissioned_as: String,
}

pub const ENTRYPOINT_OVERRIDE: &str = "_ENTRYPOINT_OVERRIDE";
pub const LARGE_LOG_THRESHOLD_SIZE: usize = 9000;
pub const EMAIL_ERROR_HANDLER_USER_EMAIL: &str = "email_error_handler@windmill.dev";

#[inline(always)]
pub fn generate_dynamic_input_key(workspace_id: &str, path: &str) -> String {
    format!("{workspace_id}:{path}")
}
