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
    scripts::{ScriptHash, ScriptLang, ScriptModule},
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
    Azure,
    Nextcloud,
    Google,
    Github,
    #[serde(rename = "ci_test")]
    #[sqlx(rename = "ci_test")]
    CiTest,
    // A run dispatched because an upstream pipeline script wrote an asset
    // this runnable subscribes to via `// on s3://...` annotations.
    Asset,
    // A run pushed by the pipeline freshness watchdog (EE) because the
    // script's `// freshness` window elapsed without a successful run.
    Freshness,
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
            JobTriggerKind::Azure => "azure",
            JobTriggerKind::Nextcloud => "nextcloud",
            JobTriggerKind::Google => "google",
            JobTriggerKind::Github => "github",
            JobTriggerKind::CiTest => "ci_test",
            JobTriggerKind::Asset => "asset",
            JobTriggerKind::Freshness => "freshness",
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
    pub fn as_str(&self) -> &'static str {
        match self {
            JobKind::Script => "script",
            JobKind::Script_Hub => "script_hub",
            JobKind::Preview => "preview",
            JobKind::Dependencies => "dependencies",
            JobKind::Flow => "flow",
            JobKind::FlowPreview => "flowpreview",
            JobKind::SingleStepFlow => "singlestepflow",
            JobKind::Identity => "identity",
            JobKind::FlowDependencies => "flowdependencies",
            JobKind::AppDependencies => "appdependencies",
            JobKind::Noop => "noop",
            JobKind::DeploymentCallback => "deploymentcallback",
            JobKind::FlowScript => "flowscript",
            JobKind::FlowNode => "flownode",
            JobKind::AppScript => "appscript",
            JobKind::AIAgent => "aiagent",
            JobKind::UnassignedScript => "unassigned_script",
            JobKind::UnassignedFlow => "unassigned_flow",
            JobKind::UnassignedSinglestepFlow => "unassigned_singlestepflow",
        }
    }

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

    pub fn is_preview(&self) -> bool {
        matches!(self, JobKind::Preview | JobKind::FlowPreview)
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    // True when this job is a native retry attempt (has a native_retry_attempt
    // marker). Lets the run-page chain distinguish real retries from other
    // same-script children (e.g. WAC inline children). The list and single-job
    // GET endpoints select it; `#[sqlx(default)]` lets any other query omit the
    // column and default to None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub is_retry: Option<bool>,
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
            labels: None,
            is_retry: None,
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
    pub labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preprocessed: Option<bool>,
    // True when this job is a native retry attempt (has a native_retry_attempt
    // marker). Lets the run-page chain distinguish real retries from other
    // same-script children (e.g. WAC inline children). The list and single-job
    // GET endpoints select it; `#[sqlx(default)]` lets any other query omit the
    // column and default to None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[sqlx(default)]
    pub is_retry: Option<bool>,
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
        labels: Option<Vec<String>>,
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
        labels: Option<Vec<String>>,
    },
    RestartedFlow {
        completed_job_id: Uuid,
        step_id: String,
        branch_or_iteration_n: Option<usize>,
        flow_version: Option<i64>,
        /// Optional locked branch for BranchOne nested restart at the top level of this run.
        branch_chosen: Option<crate::flow_status::BranchChosen>,
        /// Optional nested restart chain. When Some, the worker for the spawned flow,
        /// upon reaching `step_id`, should spawn the inner child as a `RestartedFlow`
        /// using this chain rather than fresh-launching it.
        nested: Option<Box<crate::flow_status::RestartedFrom>>,
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
        // Set when wrapping a script (not a flow). Lets `push` materialize a
        // bare-script-with-retry as a native retryable `Script` job instead of
        // spawning a one-step flow.
        language: Option<ScriptLang>,
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
        // Per-invocation suffix appended to the `{workspace_id}:git_sync` concurrency key.
        // Used in promotion mode so sync jobs targeting different branches run in parallel
        // instead of serializing through a single workspace-wide key.
        concurrency_key_append: Option<String>,
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
    pub tag: Option<String>,
    #[serde(flatten)]
    pub concurrency_settings: ConcurrencySettingsWithCustom,
    #[serde(flatten)]
    pub debouncing_settings: DebouncingSettings,
    pub modules: Option<HashMap<String, ScriptModule>>,
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

/// Reserved job-arg key holding the inbound W3C `traceparent` captured from the
/// request that enqueued the job (run endpoints). It rides the `args` jsonb like
/// [`ENTRYPOINT_OVERRIDE`]; normal scripts never see it because args are bound by
/// declared parameter name. Read back at root-job completion to link the job's
/// OTLP span to the originating distributed trace (EE/OTel only).
pub const WM_TRACEPARENT: &str = "_wm_traceparent";

/// The entrypoint override (`_ENTRYPOINT_OVERRIDE` job arg ->
/// `v2_job.script_entrypoint_override`) is interpolated verbatim into
/// generated worker wrappers in a code position (e.g. the NativeTS
/// `import(...).then(m => m.<name>(...))` glue, the bun `Main.<name>(...)`
/// call, the deno `import { <name> } from "./main.ts"` line, the PHP
/// `<name>(...)` call and the Python `inner_script.<name>(**args)` call).
/// A caller only needs `jobs:run` to set it, so it MUST be restricted to a
/// conventional identifier or an attacker who can merely run a deployed
/// script could break out of the call expression into arbitrary
/// worker-process code. This ASCII subset is a valid function name in every
/// language Windmill wraps this way (JS/TS, Python, PHP).
pub fn is_valid_entrypoint_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 255 {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub const LARGE_LOG_THRESHOLD_SIZE: usize = 9000;
pub const EMAIL_ERROR_HANDLER_USER_EMAIL: &str = "email_error_handler@windmill.dev";

#[inline(always)]
pub fn generate_dynamic_input_key(workspace_id: &str, path: &str) -> String {
    format!("{workspace_id}:{path}")
}

#[cfg(test)]
mod tests {
    use super::is_valid_entrypoint_name;

    #[test]
    fn valid_entrypoint_names_are_accepted() {
        for name in [
            "main",
            "preprocessor",
            "my_helper",
            "_private",
            "fn2",
            "a",
            "MixedCase",
        ] {
            assert!(is_valid_entrypoint_name(name), "expected {name:?} valid");
        }
    }

    #[test]
    fn malicious_entrypoint_names_are_rejected() {
        // Regression for GHSA-wxjq-w5pj-jqhx: the entrypoint override is
        // interpolated verbatim into a code position of generated worker
        // wrappers (e.g. bun `Main.<name>(...)`, nativets
        // `m.<name>(...)`, python `inner_script.<name>(**args)`). Any value
        // that is not a strict identifier could break out of the call
        // expression into attacker-controlled worker code.
        for name in [
            "main(); globalThis.x = 1; //", // breaks out of `Main.<name>(...)`
            "x); require('child_process').execSync('id'); (",
            "1main",     // starts with a digit
            "my-fn",     // hyphen
            "my fn",     // space
            "my.fn",     // member access
            "$fn",       // dollar
            "fn\nother", // newline
            "fn;other",
            "",
        ] {
            assert!(
                !is_valid_entrypoint_name(name),
                "expected {name:?} to be rejected"
            );
        }
        // Over-long names are rejected.
        assert!(!is_valid_entrypoint_name(&"a".repeat(256)));
    }
}
