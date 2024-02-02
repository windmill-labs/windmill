/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::flows::FlowValue;
use crate::more_serde::default_false;

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
    pub failure_module: FlowStatusModuleWParent,
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
    pub itered: Vec<serde_json::Value>,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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
        iterator: Option<Iterator>,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branchall: Option<BranchAllStatus>,
        #[serde(default = "default_false")]
        parallel: bool,
    },
    Success {
        id: String,
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
        #[serde(default)]
        #[serde(skip_serializing_if = "Vec::is_empty")]
        approvers: Vec<Approval>,
    },
    Failure {
        id: String,
        job: Uuid,
        #[serde(skip_serializing_if = "Option::is_none")]
        flow_jobs: Option<Vec<Uuid>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch_chosen: Option<BranchChosen>,
    },
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
            step: 0,
            approval_conditions: None,
            modules: f
                .modules
                .iter()
                .map(|m| FlowStatusModule::WaitingForPriorSteps { id: m.id.clone() })
                .collect(),
            failure_module: FlowStatusModuleWParent {
                parent_module: None,
                module_status: FlowStatusModule::WaitingForPriorSteps {
                    id: f
                        .failure_module
                        .as_ref()
                        .map(|x| x.id.clone())
                        .unwrap_or_else(|| "failure".to_string()),
                },
            },
            cleanup_module: FlowCleanupModule { flow_jobs_to_clean: vec![] },
            retry: RetryStatus { fail_count: 0, failed_jobs: vec![] },
            restarted_from: None,
        }
    }

    /// current module status ... excluding failure_module
    pub fn current_step(&self) -> Option<&FlowStatusModule> {
        let i = usize::try_from(self.step).ok()?;
        self.modules.get(i)
    }
}
