//! Restarting a parallel ForLoop / BranchAll with `failed_only`: the restarted step's status,
//! which keeps the slots that succeeded and marks the others to rerun.

use serde_json::value::RawValue;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::{
    error::Error,
    flow_status::{
        BranchAllStatus, FlowJobsDuration, FlowStatusModule, Iterator as FlowIterator,
        RestartedFrom,
    },
    flows::{FlowModule, FlowModuleValue},
};

/// A worker that predates `failed_only` drops it from a restart chain and reruns the whole
/// step, so a chain carrying it anywhere is refused until every worker reads it.
pub(crate) async fn refuse_failed_only_on_outdated_workers(
    failed_only: bool,
    nested: Option<&RestartedFrom>,
) -> Result<(), Error> {
    let mut any = failed_only;
    let mut level = nested;
    while let Some(l) = level {
        any |= l.failed_only;
        level = l.nested.as_deref();
    }
    if any {
        windmill_common::min_version::MIN_VERSION_SUPPORTS_FAILED_ONLY_RESTART
            .assert()
            .await?;
    }
    Ok(())
}

/// Status of a parallel ForLoop / BranchAll restarted with `failed_only`: a slot whose job
/// succeeded keeps it, every other slot holds `Uuid::nil()`. The worker reads a nil slot as
/// "rerun this iteration (branch) here" and counts the kept ones as already completed, so the
/// step's result stays ordered by slot.
pub(crate) async fn create_failed_only_module(
    db: &Pool<Postgres>,
    workspace_id: &str,
    module: &FlowStatusModule,
    module_definition: &FlowModule,
    restart_step_id: &str,
) -> Result<FlowStatusModule, Error> {
    let (is_loop, branch_count) = match module_definition.get_value()? {
        FlowModuleValue::ForloopFlow { parallel: true, .. } => (true, None),
        FlowModuleValue::BranchAll { parallel: true, branches, .. } => (false, Some(branches.len())),
        _ => {
            return Err(Error::BadRequest(format!(
                "Step {restart_step_id} is not a parallel for-loop or branch-all, only those can rerun their failed iterations"
            )))
        }
    };
    let original_jobs = module
        .flow_jobs()
        .filter(|jobs| !jobs.is_empty())
        .ok_or_else(|| {
            Error::BadRequest(format!(
                "Step {restart_step_id} has no recorded iterations in the original run"
            ))
        })?;
    let n = original_jobs.len();
    if branch_count.is_some_and(|count| count != n) {
        return Err(Error::BadRequest(format!(
            "Step {restart_step_id} ran {n} branches but now has {}, its failed branches cannot be matched",
            branch_count.unwrap_or(0)
        )));
    }
    let succeeded = sqlx::query_scalar!(
        "SELECT id FROM v2_job_completed
         WHERE workspace_id = $1 AND id = ANY($2) AND (status = 'success' OR status = 'skipped')",
        workspace_id,
        &original_jobs,
    )
    .fetch_all(db)
    .await?;
    // Kept only by the completed status the parallel step judges its own success by: an
    // iteration recovered by the error handler is stored as failed, so keeping it would fail
    // the restarted step again.
    let flow_jobs: Vec<Uuid> = original_jobs
        .iter()
        .map(|job| {
            if succeeded.contains(job) {
                *job
            } else {
                Uuid::nil()
            }
        })
        .collect();
    let kept = flow_jobs.iter().filter(|job| !job.is_nil()).count();
    if kept == n {
        return Err(Error::BadRequest(format!(
            "Step {restart_step_id} has no failed iterations to rerun"
        )));
    }
    // A loop reruns its failed iterations on the items they ran on, read from each iteration's
    // `iter` argument rather than re-evaluating the iterator, so every kept result stays
    // paired with its item.
    let itered = if is_loop {
        let rows = sqlx::query!(
            "SELECT id, args->'iter'->'value' AS \"value: sqlx::types::Json<Box<RawValue>>\"
             FROM v2_job WHERE workspace_id = $1 AND id = ANY($2)",
            workspace_id,
            &original_jobs,
        )
        .fetch_all(db)
        .await?;
        let items = original_jobs
            .iter()
            .map(|job| {
                rows.iter()
                    .find(|r| r.id == *job)
                    .and_then(|r| r.value.as_ref())
                    .map(|v| v.0.clone())
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                Error::BadRequest(format!(
                    "The items step {restart_step_id} iterated over in the original run are no longer stored, rerun all its iterations instead"
                ))
            })?;
        Some(items)
    } else {
        None
    };
    let mut flow_jobs_duration = module
        .flow_jobs_duration()
        .filter(|d| d.started_at.len() == n && d.duration_ms.len() == n)
        .unwrap_or_else(|| FlowJobsDuration::new(n));
    for (i, job) in flow_jobs.iter().enumerate() {
        if job.is_nil() {
            flow_jobs_duration.started_at[i] = None;
            flow_jobs_duration.duration_ms[i] = None;
        }
    }
    Ok(FlowStatusModule::InProgress {
        id: module.id(),
        job: Uuid::nil(),
        iterator: is_loop.then(|| FlowIterator {
            index: kept,
            itered,
            itered_len: Some(n),
            failure_threshold: None,
        }),
        flow_jobs_success: Some(
            flow_jobs
                .iter()
                .map(|j| (!j.is_nil()).then_some(true))
                .collect(),
        ),
        flow_jobs: Some(flow_jobs),
        flow_jobs_duration: Some(flow_jobs_duration),
        flow_jobs_labels: module.flow_jobs_labels(),
        branch_chosen: None,
        branchall: (!is_loop).then(|| BranchAllStatus { branch: kept, len: n }),
        parallel: true,
        while_loop: false,
        progress: None,
        agent_actions: None,
        agent_actions_success: None,
    })
}
