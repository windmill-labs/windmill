/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::flows::FlowValue;

const MINUTES: Duration = Duration::from_secs(60);
const HOURS: Duration = MINUTES.saturating_mul(60);

pub const MAX_RETRY_ATTEMPTS: u16 = 1000;
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
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(default)]
pub struct RetryStatus {
    pub fail_count: u16,
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Iterator {
    pub index: usize,
    pub itered: Vec<Box<serde_json::value::RawValue>>,
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
    branch_chosen: Option<BranchChosen>,
    branchall: Option<BranchAllStatus>,
    parallel: Option<bool>,
    while_loop: Option<bool>,
    approvers: Option<Vec<Approval>>,
    failed_retries: Option<Vec<Uuid>>,
    skipped: Option<bool>,
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
        branch_chosen: Option<BranchChosen>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branchall: Option<BranchAllStatus>,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        parallel: bool,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        while_loop: bool,
    },
    Success {
        id: String,
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs_success: Option<Vec<Option<bool>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        approvers: Vec<Approval>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        failed_retries: Vec<Uuid>,
        skipped: bool,
    },
    Failure {
        id: String,
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs_success: Option<Vec<Option<bool>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        failed_retries: Vec<Uuid>,
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
                branch_chosen: untagged.branch_chosen,
                branchall: untagged.branchall,
                parallel: untagged.parallel.unwrap_or(false),
                while_loop: untagged.while_loop.unwrap_or(false),
                progress: untagged.progress,
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
                branch_chosen: untagged.branch_chosen,
                approvers: untagged.approvers.unwrap_or_default(),
                failed_retries: untagged.failed_retries.unwrap_or_default(),
                skipped: untagged.skipped.unwrap_or(false),
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
                branch_chosen: untagged.branch_chosen,
                failed_retries: untagged.failed_retries.unwrap_or_default(),
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
        }
    }

    /// current module status ... excluding failure_module
    pub fn current_step(&self) -> Option<&FlowStatusModule> {
        let i = usize::try_from(self.step).ok()?;
        self.modules.get(i)
    }
}
