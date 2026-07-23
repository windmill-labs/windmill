//! Regression test for hand-recovery of between-steps zombie flows.
//!
//! When a worker is OOM-killed mid state-transition, the zombie monitor
//! (`handle_zombie_flows` → `cancel_job` with force) reaps the flow: it lands in
//! `v2_job_completed` as `canceled`, with its `flow_status` preserved: the step
//! whose transition was lost stays `InProgress` even though all its children
//! completed successfully. This test reproduces that exact terminal state and
//! asserts that a hand-restart from the stuck step reuses every completed child
//! (no re-run) and the flow reaches success.
//!
//! The reaper itself lives in the `windmill` binary crate and is unreachable
//! from an integration test, so we reproduce the state `cancel_job(force)`
//! leaves behind directly; the fix under test is the restart-resolution path,
//! not the detection query.

#![cfg(feature = "deno_core")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::flow_status::FlowStatus;
use windmill_common::flows::FlowValue;
use windmill_common::jobs::JobPayload;
use windmill_test_utils::*;

/// Child job UUID for a top-level step in a completed flow's `flow_status`
/// (optionally the iteration index for a ForLoop / BranchAll container).
async fn child_job_id_for_step(
    db: &Pool<Postgres>,
    flow_job_id: uuid::Uuid,
    step_id: &str,
    iter: Option<usize>,
) -> uuid::Uuid {
    let raw: serde_json::Value = sqlx::query_scalar!(
        "SELECT flow_status FROM v2_job_completed WHERE id = $1",
        flow_job_id
    )
    .fetch_one(db)
    .await
    .unwrap()
    .expect("flow_status missing");
    let status: FlowStatus = serde_json::from_value(raw).expect("parse flow_status");
    let module = status
        .modules
        .iter()
        .find(|m| m.id() == step_id)
        .expect("step in flow_status");
    match iter {
        Some(i) => module.flow_jobs().expect("flow_jobs")[i],
        None => module.job().expect("job"),
    }
}

/// A between-steps zombie whose fan-out completed but whose final transition was
/// lost can be hand-restarted from the stuck step, reusing every completed child
/// (including the last iteration) and reaching success.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_between_steps_zombie_restart_reuses_all_children(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // A fan-out ForLoop `fanout` (2 iterations) followed by `after`, which
    // consumes the loop's aggregated result. In the zombie scenario `fanout`
    // finished all iterations but its final transition was lost, so `after`
    // never ran.
    let flow_value: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "fanout",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "['a', 'b']" },
                    "skip_failures": false,
                    "parallel": false,
                    "modules": [{
                        "id": "inner",
                        "value": {
                            "type": "rawscript",
                            "language": "deno",
                            "input_transforms": {
                                "v": { "type": "javascript", "expr": "flow_input.iter.value" }
                            },
                            "content": "export function main(v: string) { return v }"
                        }
                    }]
                }
            },
            {
                "id": "after",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "input_transforms": {
                        "loop_res": { "type": "javascript", "expr": "results.fanout" }
                    },
                    "content": "export function main(loop_res: string[]) { return loop_res.join(',') }"
                }
            }
        ]
    }))
    .unwrap();

    // 1. Run to completion to obtain real, successful child jobs.
    let full_run = RunJob::from(JobPayload::RawFlow {
        value: flow_value.clone(),
        path: None,
        restarted_from: None,
    })
    .run_until_complete(&db, false, port)
    .await;
    assert!(full_run.success, "baseline run should succeed");
    assert_eq!(full_run.json_result().unwrap(), json!("a,b"));

    let orig_iter0 = child_job_id_for_step(&db, full_run.id, "fanout", Some(0)).await;
    let orig_iter1 = child_job_id_for_step(&db, full_run.id, "fanout", Some(1)).await;
    let orig_after = child_job_id_for_step(&db, full_run.id, "after", None).await;

    // 2. Reproduce the zombie-reaper's terminal state: cancelled by `monitor`
    //    with `flow_status` frozen mid-transition: `fanout` still `InProgress`
    //    (all iterations done), `after` never reached.
    let mut flow_status: serde_json::Value = sqlx::query_scalar!(
        "SELECT flow_status FROM v2_job_completed WHERE id = $1",
        full_run.id
    )
    .fetch_one(&db)
    .await?
    .expect("flow_status");
    for m in flow_status["modules"].as_array_mut().unwrap() {
        match m["id"].as_str() {
            Some("fanout") => {
                m["type"] = json!("InProgress");
                // A reaped loop keeps its cursor at the last iteration.
                m["iterator"] = json!({ "index": 1, "itered_len": 2 });
            }
            Some("after") => *m = json!({ "type": "WaitingForPriorSteps", "id": "after" }),
            _ => {}
        }
    }
    flow_status["step"] = json!(0);
    sqlx::query!(
        "UPDATE v2_job_completed
         SET status = 'canceled', canceled_by = 'monitor', canceled_reason = 'zombie flow',
             flow_status = $2
         WHERE id = $1",
        full_run.id,
        flow_status,
    )
    .execute(&db)
    .await?;

    // 3. Hand-restart from the stuck step. `fanout` is recognised as a derivable
    //    between-steps zombie (all children succeeded), so it is reused verbatim
    //    and only the dropped transition onward is replayed.
    let restarted = RunJob::from(JobPayload::RestartedFlow {
        completed_job_id: full_run.id,
        step_id: "fanout".into(),
        branch_or_iteration_n: None,
        flow_version: None,
        branch_chosen: None,
        nested: None,
    })
    .run_until_complete(&db, false, port)
    .await;

    // 4. Flow reaches success, reusing the loop's aggregated result.
    assert!(
        restarted.success,
        "restarted zombie flow should succeed: {:?}",
        restarted.json_result()
    );
    assert_eq!(restarted.json_result().unwrap(), json!("a,b"));

    // Every completed loop iteration reuses its original child job (no re-run);
    // only `after`, which never ran, executes fresh.
    let new_iter0 = child_job_id_for_step(&db, restarted.id, "fanout", Some(0)).await;
    let new_iter1 = child_job_id_for_step(&db, restarted.id, "fanout", Some(1)).await;
    let new_after = child_job_id_for_step(&db, restarted.id, "after", None).await;
    assert_eq!(new_iter0, orig_iter0, "loop iteration 0 must be reused");
    assert_eq!(new_iter1, orig_iter1, "loop iteration 1 must be reused");
    assert_ne!(new_after, orig_after, "`after` should run fresh");

    Ok(())
}

/// A serial for-loop reaped *between* iterations (an all-success prefix, but the
/// cursor not yet at the last iteration) must NOT be treated as complete: reuse
/// would silently drop the remaining iterations. A downstream `after` step makes
/// the loop non-final, so the ONLY thing that can prevent reuse here is the
/// cursor-completeness guard; if it regresses, `after` would consume a truncated
/// loop result and this test fails. Restart must re-run the whole loop instead.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_mid_iteration_zombie_not_reused(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow_value: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "fanout",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "['a', 'b', 'c']" },
                    "skip_failures": false,
                    "parallel": false,
                    "modules": [{
                        "id": "inner",
                        "value": {
                            "type": "rawscript",
                            "language": "deno",
                            "input_transforms": {
                                "v": { "type": "javascript", "expr": "flow_input.iter.value" }
                            },
                            "content": "export function main(v: string) { return v }"
                        }
                    }]
                }
            },
            {
                "id": "after",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "input_transforms": {
                        "loop_res": { "type": "javascript", "expr": "results.fanout" }
                    },
                    "content": "export function main(loop_res: string[]) { return loop_res.join(',') }"
                }
            }
        ]
    }))
    .unwrap();

    let full_run = RunJob::from(JobPayload::RawFlow {
        value: flow_value.clone(),
        path: None,
        restarted_from: None,
    })
    .run_until_complete(&db, false, port)
    .await;
    assert!(full_run.success);
    let orig_iter0 = child_job_id_for_step(&db, full_run.id, "fanout", Some(0)).await;

    // Reap after iteration 0: the loop is InProgress with the cursor still on
    // iteration 0 (of 3), only iteration 0 recorded; `after` never reached.
    let mut flow_status: serde_json::Value = sqlx::query_scalar!(
        "SELECT flow_status FROM v2_job_completed WHERE id = $1",
        full_run.id
    )
    .fetch_one(&db)
    .await?
    .expect("flow_status");
    for m in flow_status["modules"].as_array_mut().unwrap() {
        match m["id"].as_str() {
            Some("fanout") => {
                m["type"] = json!("InProgress");
                m["iterator"] = json!({ "index": 0, "itered_len": 3 });
                m["flow_jobs"] = json!([m["flow_jobs"][0]]);
                m["flow_jobs_success"] = json!([true]);
            }
            Some("after") => *m = json!({ "type": "WaitingForPriorSteps", "id": "after" }),
            _ => {}
        }
    }
    flow_status["step"] = json!(0);
    sqlx::query!(
        "UPDATE v2_job_completed SET status = 'canceled', canceled_by = 'monitor',
         flow_status = $2 WHERE id = $1",
        full_run.id,
        flow_status,
    )
    .execute(&db)
    .await?;

    let restarted = RunJob::from(JobPayload::RestartedFlow {
        completed_job_id: full_run.id,
        step_id: "fanout".into(),
        branch_or_iteration_n: None,
        flow_version: None,
        branch_chosen: None,
        nested: None,
    })
    .run_until_complete(&db, false, port)
    .await;

    // The loop re-runs from scratch: all three iterations execute (so `after` sees
    // "a,b,c", not a truncated "a"), and iteration 0 is a fresh job.
    assert!(
        restarted.success,
        "restart should re-run the loop and succeed: {:?}",
        restarted.json_result()
    );
    assert_eq!(restarted.json_result().unwrap(), json!("a,b,c"));
    let new_iter0 = child_job_id_for_step(&db, restarted.id, "fanout", Some(0)).await;
    assert_ne!(
        new_iter0, orig_iter0,
        "iteration 0 must re-run, not be reused"
    );

    Ok(())
}
