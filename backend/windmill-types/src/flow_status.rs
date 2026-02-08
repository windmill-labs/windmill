use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::flows::FlowValue;

const MINUTES: Duration = Duration::from_secs(60);
const HOURS: Duration = MINUTES.saturating_mul(60);

pub const MAX_RETRY_ATTEMPTS: u32 = u32::MAX;
pub const MAX_RETRY_INTERVAL: Duration = HOURS.saturating_mul(6);

pub fn is_retry_default(v: &RetryStatus) -> bool {
    v.fail_count == 0 && v.failed_jobs.is_empty()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowStatus {
    pub step: i32,
    pub modules: Vec<FlowStatusModule>,
    pub failure_module: Box<FlowStatusModuleWParent>,
    pub preprocessor_module: Option<FlowStatusModule>,

    #[serde(skip_serializing_if = "HashMap::is_empty")]
    #[serde(default)]
    pub user_states: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub cleanup_module: FlowCleanupModule,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_retry_default")]
    pub retry: RetryStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_conditions: Option<ApprovalConditions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restarted_from: Option<RestartedFrom>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_job: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_input_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct RetryStatus {
    pub fail_count: u32,
    pub failed_jobs: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct ApprovalConditions {
    pub user_auth_required: bool,
    pub user_groups_required: Vec<String>,
    pub self_approval_disabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct RestartedFrom {
    pub flow_job_id: Uuid,
    pub step_id: String,
    pub branch_or_iteration_n: Option<usize>,
    pub flow_version: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Iterator {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itered: Option<Vec<Box<serde_json::value::RawValue>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub itered_len: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BranchAllStatus {
    pub branch: usize,
    pub len: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all(serialize = "lowercase", deserialize = "lowercase")
)]
pub enum BranchChosen {
    Default,
    Branch { branch: usize },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Approval {
    pub resume_id: u16,
    pub approver: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FlowStatusModuleWParent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_module: Option<String>,
    #[serde(flatten)]
    pub module_status: FlowStatusModule,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FlowCleanupModule {
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flow_jobs_to_clean: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowJobsDuration {
    pub started_at: Vec<Option<chrono::DateTime<chrono::Utc>>>,
    pub duration_ms: Vec<Option<i64>>,
}

impl FlowJobsDuration {
    pub fn set(&mut self, position: Option<usize>, value: &Option<FlowJobDuration>) {
        if let Some(position) = position {
            if position >= self.started_at.len()
                || position >= self.duration_ms.len()
                || value.is_none()
            {
                return;
            }
            let value = value.clone().unwrap();
            self.started_at[position] = Some(value.started_at);
            self.duration_ms[position] = Some(value.duration_ms);
        }
    }

    pub fn push(&mut self, value: &Option<FlowJobDuration>) {
        self.started_at.push(value.as_ref().map(|x| x.started_at));
        self.duration_ms.push(value.as_ref().map(|x| x.duration_ms));
    }

    pub fn new(n: usize) -> Self {
        Self { started_at: vec![None; n], duration_ms: vec![None; n] }
    }

    pub fn truncate(&mut self, n: usize) {
        self.started_at.truncate(n);
        self.duration_ms.truncate(n);
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FlowJobDuration {
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: i64,
}

#[derive(Deserialize)]
struct UntaggedFlowStatusModule {
    #[serde(rename = "type")]
    type_: String,
    id: Option<String>,
    count: Option<u16>,
    progress: Option<u8>,
    job: Option<Uuid>,
    iterator: Option<Iterator>,
    flow_jobs: Option<Vec<Uuid>>,
    flow_jobs_success: Option<Vec<Option<bool>>>,
    flow_jobs_duration: Option<FlowJobsDuration>,
    branch_chosen: Option<BranchChosen>,
    branchall: Option<BranchAllStatus>,
    parallel: Option<bool>,
    while_loop: Option<bool>,
    approvers: Option<Vec<Approval>>,
    failed_retries: Option<Vec<Uuid>>,
    skipped: Option<bool>,
    agent_actions: Option<Vec<AgentAction>>,
    agent_actions_success: Option<Vec<bool>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentAction {
    ToolCall {
        job_id: uuid::Uuid,
        function_name: String,
        module_id: String,
    },
    McpToolCall {
        call_id: uuid::Uuid,
        function_name: String,
        resource_path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        arguments: Option<serde_json::Value>,
    },
    Message {},
    WebSearch {},
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum FlowStatusModule {
    WaitingForPriorSteps {
        id: String,
    },
    WaitingForEvents {
        id: String,
        count: u16,
        job: Uuid,
    },
    WaitingForExecutor {
        id: String,
        job: Uuid,
    },
    InProgress {
        id: String,
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        progress: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        iterator: Option<Iterator>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs_success: Option<Vec<Option<bool>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs_duration: Option<FlowJobsDuration>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branchall: Option<BranchAllStatus>,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        parallel: bool,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        while_loop: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_actions: Option<Vec<AgentAction>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_actions_success: Option<Vec<bool>>,
    },
    Success {
        id: String,
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs_success: Option<Vec<Option<bool>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs_duration: Option<FlowJobsDuration>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        approvers: Vec<Approval>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        failed_retries: Vec<Uuid>,
        skipped: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_actions: Option<Vec<AgentAction>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_actions_success: Option<Vec<bool>>,
    },
    Failure {
        id: String,
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs_success: Option<Vec<Option<bool>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs_duration: Option<FlowJobsDuration>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        failed_retries: Vec<Uuid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_actions: Option<Vec<AgentAction>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_actions_success: Option<Vec<bool>>,
    },
}

impl<'de> Deserialize<'de> for FlowStatusModule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let untagged: UntaggedFlowStatusModule =
            UntaggedFlowStatusModule::deserialize(deserializer)?;

        match untagged.type_.as_str() {
            "WaitingForPriorSteps" => Ok(FlowStatusModule::WaitingForPriorSteps {
                id: untagged
                    .id
                    .ok_or_else(|| serde::de::Error::missing_field("id"))?,
            }),
            "WaitingForEvents" => Ok(FlowStatusModule::WaitingForEvents {
                id: untagged
                    .id
                    .ok_or_else(|| serde::de::Error::missing_field("id"))?,
                count: untagged
                    .count
                    .ok_or_else(|| serde::de::Error::missing_field("count"))?,
                job: untagged
                    .job
                    .ok_or_else(|| serde::de::Error::missing_field("job"))?,
            }),
            "WaitingForExecutor" => Ok(FlowStatusModule::WaitingForExecutor {
                id: untagged
                    .id
                    .ok_or_else(|| serde::de::Error::missing_field("id"))?,
                job: untagged
                    .job
                    .ok_or_else(|| serde::de::Error::missing_field("job"))?,
            }),
            "InProgress" => Ok(FlowStatusModule::InProgress {
                id: untagged
                    .id
                    .ok_or_else(|| serde::de::Error::missing_field("id"))?,
                job: untagged
                    .job
                    .ok_or_else(|| serde::de::Error::missing_field("job"))?,
                iterator: untagged.iterator,
                flow_jobs: untagged.flow_jobs,
                flow_jobs_success: untagged.flow_jobs_success,
                flow_jobs_duration: untagged.flow_jobs_duration,
                branch_chosen: untagged.branch_chosen,
                branchall: untagged.branchall,
                parallel: untagged.parallel.unwrap_or(false),
                while_loop: untagged.while_loop.unwrap_or(false),
                progress: untagged.progress,
                agent_actions: untagged.agent_actions,
                agent_actions_success: untagged.agent_actions_success,
            }),
            "Success" => Ok(FlowStatusModule::Success {
                id: untagged
                    .id
                    .ok_or_else(|| serde::de::Error::missing_field("id"))?,
                job: untagged
                    .job
                    .ok_or_else(|| serde::de::Error::missing_field("job"))?,
                flow_jobs: untagged.flow_jobs,
                flow_jobs_success: untagged.flow_jobs_success,
                flow_jobs_duration: untagged.flow_jobs_duration,
                branch_chosen: untagged.branch_chosen,
                approvers: untagged.approvers.unwrap_or_default(),
                failed_retries: untagged.failed_retries.unwrap_or_default(),
                skipped: untagged.skipped.unwrap_or(false),
                agent_actions: untagged.agent_actions,
                agent_actions_success: untagged.agent_actions_success,
            }),
            "Failure" => Ok(FlowStatusModule::Failure {
                id: untagged
                    .id
                    .ok_or_else(|| serde::de::Error::missing_field("id"))?,
                job: untagged
                    .job
                    .ok_or_else(|| serde::de::Error::missing_field("job"))?,
                flow_jobs: untagged.flow_jobs,
                flow_jobs_success: untagged.flow_jobs_success,
                flow_jobs_duration: untagged.flow_jobs_duration,
                branch_chosen: untagged.branch_chosen,
                failed_retries: untagged.failed_retries.unwrap_or_default(),
                agent_actions: untagged.agent_actions,
                agent_actions_success: untagged.agent_actions_success,
            }),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &[
                    "WaitingForPriorSteps",
                    "WaitingForEvents",
                    "WaitingForExecutor",
                    "InProgress",
                    "Success",
                    "Failure",
                ],
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobResult {
    SingleJob(Uuid),
    ListJob(Vec<Uuid>),
}

impl FlowStatusModule {
    pub fn job(&self) -> Option<Uuid> {
        match self {
            FlowStatusModule::WaitingForPriorSteps { .. } => None,
            FlowStatusModule::WaitingForEvents { job, .. } => Some(*job),
            FlowStatusModule::WaitingForExecutor { job, .. } => Some(*job),
            FlowStatusModule::InProgress { job, .. } => Some(*job),
            FlowStatusModule::Success { job, .. } => Some(*job),
            FlowStatusModule::Failure { job, .. } => Some(*job),
        }
    }

    pub fn flow_jobs(&self) -> Option<Vec<Uuid>> {
        match self {
            FlowStatusModule::InProgress { flow_jobs, .. } => flow_jobs.clone(),
            FlowStatusModule::Success { flow_jobs, .. } => flow_jobs.clone(),
            FlowStatusModule::Failure { flow_jobs, .. } => flow_jobs.clone(),
            _ => None,
        }
    }

    pub fn branch_chosen(&self) -> Option<BranchChosen> {
        match self {
            FlowStatusModule::InProgress { branch_chosen, .. } => branch_chosen.clone(),
            FlowStatusModule::Success { branch_chosen, .. } => branch_chosen.clone(),
            FlowStatusModule::Failure { branch_chosen, .. } => branch_chosen.clone(),
            _ => None,
        }
    }

    pub fn flow_jobs_success(&self) -> Option<Vec<Option<bool>>> {
        match self {
            FlowStatusModule::InProgress { flow_jobs_success, .. } => flow_jobs_success.clone(),
            FlowStatusModule::Success { flow_jobs_success, .. } => flow_jobs_success.clone(),
            FlowStatusModule::Failure { flow_jobs_success, .. } => flow_jobs_success.clone(),
            _ => None,
        }
    }

    pub fn flow_jobs_duration(&self) -> Option<FlowJobsDuration> {
        match self {
            FlowStatusModule::InProgress { flow_jobs_duration, .. } => flow_jobs_duration.clone(),
            FlowStatusModule::Success { flow_jobs_duration, .. } => flow_jobs_duration.clone(),
            FlowStatusModule::Failure { flow_jobs_duration, .. } => flow_jobs_duration.clone(),
            _ => None,
        }
    }

    pub fn job_result(&self) -> Option<JobResult> {
        self.flow_jobs()
            .map(JobResult::ListJob)
            .or_else(|| self.job().map(JobResult::SingleJob))
    }

    pub fn id(&self) -> String {
        match self {
            FlowStatusModule::WaitingForPriorSteps { id, .. } => id.clone(),
            FlowStatusModule::WaitingForEvents { id, .. } => id.clone(),
            FlowStatusModule::WaitingForExecutor { id, .. } => id.clone(),
            FlowStatusModule::InProgress { id, .. } => id.clone(),
            FlowStatusModule::Success { id, .. } => id.clone(),
            FlowStatusModule::Failure { id, .. } => id.clone(),
        }
    }

    pub fn is_failure(&self) -> bool {
        match self {
            FlowStatusModule::Failure { .. } => true,
            _ => false,
        }
    }

    pub fn agent_actions(&self) -> Option<Vec<AgentAction>> {
        match self {
            FlowStatusModule::InProgress { agent_actions, .. } => agent_actions.clone(),
            FlowStatusModule::Success { agent_actions, .. } => agent_actions.clone(),
            FlowStatusModule::Failure { agent_actions, .. } => agent_actions.clone(),
            _ => None,
        }
    }

    pub fn agent_actions_success(&self) -> Option<Vec<bool>> {
        match self {
            FlowStatusModule::InProgress { agent_actions_success, .. } => {
                agent_actions_success.clone()
            }
            FlowStatusModule::Success { agent_actions_success, .. } => {
                agent_actions_success.clone()
            }
            FlowStatusModule::Failure { agent_actions_success, .. } => {
                agent_actions_success.clone()
            }
            _ => None,
        }
    }
}

impl FlowStatus {
    pub fn new(f: &FlowValue) -> Self {
        Self {
            step: if f.preprocessor_module.is_some() {
                -1
            } else {
                0
            },
            approval_conditions: None,
            modules: f
                .modules
                .iter()
                .map(|m| FlowStatusModule::WaitingForPriorSteps { id: m.id.clone() })
                .collect(),
            failure_module: Box::new(FlowStatusModuleWParent {
                parent_module: None,
                module_status: FlowStatusModule::WaitingForPriorSteps {
                    id: f
                        .failure_module
                        .as_ref()
                        .map(|x| x.id.clone())
                        .unwrap_or_else(|| "failure".to_string()),
                },
            }),
            preprocessor_module: if f.preprocessor_module.is_some() {
                Some(FlowStatusModule::WaitingForPriorSteps {
                    id: f.preprocessor_module.as_ref().unwrap().id.clone(),
                })
            } else {
                None
            },
            cleanup_module: FlowCleanupModule { flow_jobs_to_clean: vec![] },
            retry: RetryStatus { fail_count: 0, failed_jobs: vec![] },
            restarted_from: None,
            user_states: HashMap::new(),
            stream_job: None,
            chat_input_enabled: f.chat_input_enabled,
            memory_id: None,
        }
    }

    /// current module status ... excluding failure_module
    pub fn current_step(&self) -> Option<&FlowStatusModule> {
        let i = usize::try_from(self.step).ok()?;
        self.modules.get(i)
    }
}
