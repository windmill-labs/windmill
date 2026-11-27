//! A failed-only restart of a parallel ForLoop / BranchAll: which slots rerun, and where each
//! failed iteration (branch) resumes.

use std::collections::HashMap;

use serde_json::value::RawValue;
use sqlx::types::Json;
use uuid::Uuid;
use windmill_common::{
    error,
    flow_status::{FlowJobsDuration, FlowStatus, FlowStatusModule, RestartedFrom},
    jobs::JobPayload,
    DB,
};

/// A parallel ForLoop / BranchAll restarted with `failed_only`: the original run's slots, a
/// kept job in each slot that succeeded and `Uuid::nil()` in each slot to rerun. The pushed
/// jobs fill the nil slots in order.
#[derive(Debug)]
pub(crate) struct FailedOnlySlots {
    pub(crate) flow_jobs: Vec<Uuid>,
    pub(crate) flow_jobs_success: Option<Vec<Option<bool>>>,
    pub(crate) flow_jobs_duration: Option<FlowJobsDuration>,
    /// For a loop, the item each slot ran on in the restarted run.
    pub(crate) itered: Option<Vec<Box<RawValue>>>,
    /// Per slot to rerun, where its original job can resume rather than start over.
    resumes: HashMap<usize, IterationResume>,
}

/// A failed iteration (branch) resumed at its first step that did not succeed, keeping the
/// results of the steps before it.
#[derive(Debug)]
struct IterationResume {
    job: Uuid,
    /// The flow node the original job ran, when it ran one.
    node: Option<i64>,
    /// The original job's steps, and the index of the one it resumes at.
    steps: Vec<String>,
    at: usize,
}

impl FailedOnlySlots {
    /// Only a restart writes nil slots, and the worker resets the step as soon as it reads
    /// them, so a parallel step entered for any other reason is never mistaken for one.
    pub(crate) fn from_status(status_module: &FlowStatusModule) -> Option<Self> {
        match status_module {
            FlowStatusModule::InProgress {
                parallel: true,
                flow_jobs: Some(flow_jobs),
                flow_jobs_success,
                flow_jobs_duration,
                iterator,
                ..
            } if flow_jobs.iter().any(Uuid::is_nil) => Some(Self {
                flow_jobs: flow_jobs.clone(),
                flow_jobs_success: flow_jobs_success.clone(),
                flow_jobs_duration: flow_jobs_duration.clone(),
                itered: iterator.as_ref().and_then(|i| i.itered.clone()),
                resumes: HashMap::new(),
            }),
            _ => None,
        }
    }

    pub(crate) fn reruns(&self, slot: usize) -> bool {
        self.flow_jobs.get(slot).is_some_and(Uuid::is_nil)
    }

    pub(crate) fn rerun_slots(&self) -> Vec<usize> {
        (0..self.flow_jobs.len())
            .filter(|&i| self.reruns(i))
            .collect()
    }

    pub(crate) fn kept(&self) -> usize {
        self.flow_jobs.iter().filter(|j| !j.is_nil()).count()
    }

    /// The step's job, success and duration lists with the pushed `reruns` in the nil slots.
    pub(crate) fn with_reruns(
        self,
        reruns: &[Uuid],
    ) -> (Vec<Uuid>, Vec<Option<bool>>, FlowJobsDuration) {
        let mut reruns = reruns.iter();
        let flow_jobs: Vec<Uuid> = self
            .flow_jobs
            .into_iter()
            .map(|j| {
                if j.is_nil() {
                    reruns.next().copied().unwrap_or(j)
                } else {
                    j
                }
            })
            .collect();
        let n = flow_jobs.len();
        let success = self
            .flow_jobs_success
            .filter(|s| s.len() == n)
            .unwrap_or(vec![None; n]);
        let duration = self
            .flow_jobs_duration
            .unwrap_or_else(|| FlowJobsDuration::new(n));
        (flow_jobs, success, duration)
    }

    /// Reads, from the restarted run, each rerun slot's original job and its first step that
    /// did not succeed. A slot that failed before its first step finished starts over.
    pub(crate) async fn load_resumes(
        &mut self,
        db: &DB,
        w_id: &str,
        status: &FlowStatus,
        step_id: &str,
    ) -> error::Result<()> {
        let Some(original_run) = status
            .restarted_from
            .as_ref()
            .filter(|r| r.step_id == step_id && r.from_failed_step)
            .map(|r| r.flow_job_id)
        else {
            return Ok(());
        };
        let original_status = sqlx::query_scalar!(
            "SELECT flow_status AS \"flow_status: Json<FlowStatus>\"
             FROM v2_job_completed WHERE id = $1 AND workspace_id = $2",
            original_run,
            w_id,
        )
        .fetch_optional(db)
        .await?
        .flatten();
        let Some(original_jobs) = original_status.and_then(|s| {
            s.0.modules
                .iter()
                .find(|m| m.id() == step_id)
                .and_then(|m| m.flow_jobs())
        }) else {
            return Ok(());
        };
        let reruns: Vec<(usize, Uuid)> = self
            .rerun_slots()
            .into_iter()
            .filter_map(|slot| original_jobs.get(slot).map(|job| (slot, *job)))
            .collect();
        let rows = sqlx::query!(
            "SELECT c.id, j.runnable_id, c.flow_status AS \"flow_status: Json<FlowStatus>\"
             FROM v2_job_completed c JOIN v2_job j USING (id)
             WHERE c.id = ANY($1) AND c.workspace_id = $2",
            &reruns.iter().map(|(_, job)| *job).collect::<Vec<_>>(),
            w_id,
        )
        .fetch_all(db)
        .await?;
        for (slot, job) in reruns {
            let Some(row) = rows.iter().find(|r| r.id == job) else {
                continue;
            };
            // A restart needs every step before its restart point to have succeeded. Only a
            // step that ran and failed is resumed at: one held back by an approval that was
            // rejected or timed out is recorded as failed with no job, and restarting past that
            // approval is refused, so its iteration starts over and asks for it again.
            let resume = row.flow_status.as_ref().and_then(|s| {
                let modules = &s.0.modules;
                let at = modules
                    .iter()
                    .position(|m| !matches!(m, FlowStatusModule::Success { .. }))?;
                let ran =
                    matches!(modules[at], FlowStatusModule::Failure { job, .. } if !job.is_nil());
                (at > 0 && ran).then(|| (modules.iter().map(|m| m.id()).collect(), at))
            });
            if let Some((steps, at)) = resume {
                self.resumes.insert(
                    slot,
                    IterationResume { job, node: row.runnable_id, steps, at },
                );
            }
        }
        Ok(())
    }

    /// The payload for `slot`: its fresh iteration, or a restart of its original job from the
    /// failed step when the iteration's definition can still run it. A deployed iteration runs
    /// a content-addressed flow node, so an unchanged node means an unchanged definition. An
    /// inline one is resumed only if its steps are the same, in the same order: a restart reuses
    /// the earlier steps' results by position and runs only the steps the original run had.
    pub(crate) fn resume_payload(&self, slot: usize, payload: JobPayload) -> JobPayload {
        let Some(resume) = self.resumes.get(&slot) else {
            return payload;
        };
        let Some(step_id) = resume.steps.get(resume.at).cloned() else {
            return payload;
        };
        match payload {
            JobPayload::FlowNode { id, .. } if Some(id.0) == resume.node => {
                JobPayload::RestartedFlow {
                    completed_job_id: resume.job,
                    step_id,
                    branch_or_iteration_n: None,
                    flow_version: None,
                    branch_chosen: None,
                    nested: None,
                    failed_only: false,
                    from_failed_step: false,
                }
            }
            JobPayload::RawFlow { value, path, restarted_from: None }
                if value.modules.iter().map(|m| &m.id).eq(resume.steps.iter()) =>
            {
                JobPayload::RawFlow {
                    value,
                    path,
                    restarted_from: Some(RestartedFrom {
                        flow_job_id: resume.job,
                        step_id,
                        ..Default::default()
                    }),
                }
            }
            payload => payload,
        }
    }
}
