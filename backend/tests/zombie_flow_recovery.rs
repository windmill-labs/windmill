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
use windmill_common::flow_status::{BranchChosen, FlowStatus, RestartedFrom};
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

    // Run to completion to obtain real, successful child jobs.
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

    // Reproduce the zombie-reaper's terminal state: cancelled by `monitor` with
    // `flow_status` frozen mid-transition: `fanout` still `InProgress` (all
    // iterations done), `after` never reached.
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

    // Hand-restart from the stuck step. `fanout` is recognised as a derivable
    // between-steps zombie (all children succeeded), so it is reused verbatim and
    // only the dropped transition onward is replayed. `Some(0)` is the exact value the
    // run page's "Re-start from" button sends (a whole-step restart), not `None`.
    let restarted = RunJob::from(JobPayload::RestartedFlow {
        completed_job_id: full_run.id,
        step_id: "fanout".into(),
        branch_or_iteration_n: Some(0),
        flow_version: None,
        branch_chosen: None,
        nested: None,
    })
    .run_until_complete(&db, false, port)
    .await;

    // Flow reaches success, reusing the loop's aggregated result.
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

/// A nested restart request targets an inner step of the restart-step container.
/// Even when that container is an eligible between-steps zombie, reuse must NOT
/// fire (it would skip the whole container and ignore the explicit nested target).
/// The inner step must re-run.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_nested_restart_not_swallowed_by_zombie_reuse(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // `branch` is a BranchOne (single child, so branch_or_iteration_n is None on
    // restart: the exact shape that would trip zombie reuse) with two inner steps,
    // followed by a downstream `after`.
    let flow_value: FlowValue = serde_json::from_value(json!({
        "modules": [
            {
                "id": "branch",
                "value": {
                    "type": "branchone",
                    "default": [],
                    "branches": [{
                        "expr": "true",
                        "modules": [
                            {
                                "id": "inner_first",
                                "value": {
                                    "type": "rawscript", "language": "deno",
                                    "input_transforms": {},
                                    "content": "export function main() { return 'first' }"
                                }
                            },
                            {
                                "id": "inner_second",
                                "value": {
                                    "type": "rawscript", "language": "deno",
                                    "input_transforms": {
                                        "first": { "type": "javascript", "expr": "results.inner_first" }
                                    },
                                    "content": "export function main(first: string) { return `${first}|second` }"
                                }
                            }
                        ]
                    }]
                }
            },
            {
                "id": "after",
                "value": {
                    "type": "rawscript", "language": "deno",
                    "input_transforms": {
                        "b": { "type": "javascript", "expr": "results.branch" }
                    },
                    "content": "export function main(b: string) { return `after:${b}` }"
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
    assert_eq!(full_run.json_result().unwrap(), json!("after:first|second"));
    let branch_child = child_job_id_for_step(&db, full_run.id, "branch", None).await;
    let orig_inner_second = child_job_id_for_step(&db, branch_child, "inner_second", None).await;

    // Reap `branch` as a between-steps zombie (its child completed, transition lost);
    // `after` never reached.
    let mut flow_status: serde_json::Value = sqlx::query_scalar!(
        "SELECT flow_status FROM v2_job_completed WHERE id = $1",
        full_run.id
    )
    .fetch_one(&db)
    .await?
    .expect("flow_status");
    for m in flow_status["modules"].as_array_mut().unwrap() {
        match m["id"].as_str() {
            Some("branch") => m["type"] = json!("InProgress"),
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

    // Nested restart: re-run `inner_second` inside `branch`. Zombie reuse must step
    // aside so the nested chain is honored.
    let restarted = RunJob::from(JobPayload::RestartedFlow {
        completed_job_id: full_run.id,
        step_id: "branch".into(),
        branch_or_iteration_n: None,
        flow_version: None,
        branch_chosen: Some(BranchChosen::Branch { branch: 0 }),
        nested: Some(Box::new(RestartedFrom {
            flow_job_id: branch_child,
            step_id: "inner_second".into(),
            branch_or_iteration_n: None,
            flow_version: None,
            branch_chosen: None,
            nested: None,
        })),
    })
    .run_until_complete(&db, false, port)
    .await;

    assert!(
        restarted.success,
        "nested restart of a zombie container should succeed: {:?}",
        restarted.json_result()
    );
    assert_eq!(
        restarted.json_result().unwrap(),
        json!("after:first|second")
    );
    let new_branch_child = child_job_id_for_step(&db, restarted.id, "branch", None).await;
    let new_inner_second = child_job_id_for_step(&db, new_branch_child, "inner_second", None).await;
    assert_ne!(
        new_inner_second, orig_inner_second,
        "the nested target inner_second must re-run, not be skipped by zombie reuse"
    );

    Ok(())
}

/// A raw-flow (editor preview) restart queues the request's CURRENT definition, which the editor
/// allows to differ from the completed run. Zombie reuse must not fire there: it would validate the
/// stored step and synthesize Success from the old children, skipping the user's edit. The edited
/// step must re-run and downstream must observe its new result.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_raw_flow_restart_does_not_reuse_edited_step(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let flow_of = |suffix: &str| -> FlowValue {
        serde_json::from_value(json!({
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
                                "type": "rawscript", "language": "deno",
                                "input_transforms": {
                                    "v": { "type": "javascript", "expr": "flow_input.iter.value" }
                                },
                                "content": format!("export function main(v: string) {{ return v + '{suffix}' }}")
                            }
                        }]
                    }
                },
                {
                    "id": "after",
                    "value": {
                        "type": "rawscript", "language": "deno",
                        "input_transforms": {
                            "loop_res": { "type": "javascript", "expr": "results.fanout" }
                        },
                        "content": "export function main(loop_res: string[]) { return loop_res.join(',') }"
                    }
                }
            ]
        }))
        .unwrap()
    };

    let full_run =
        RunJob::from(JobPayload::RawFlow { value: flow_of(""), path: None, restarted_from: None })
            .run_until_complete(&db, false, port)
            .await;
    assert!(full_run.success);
    assert_eq!(full_run.json_result().unwrap(), json!("a,b"));
    let orig_iter0 = child_job_id_for_step(&db, full_run.id, "fanout", Some(0)).await;

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
                m["iterator"] = json!({ "index": 1, "itered_len": 2 });
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

    let restarted = RunJob::from(JobPayload::RawFlow {
        value: flow_of("X"),
        path: None,
        restarted_from: Some(RestartedFrom {
            flow_job_id: full_run.id,
            step_id: "fanout".into(),
            branch_or_iteration_n: None,
            flow_version: None,
            branch_chosen: None,
            nested: None,
        }),
    })
    .run_until_complete(&db, false, port)
    .await;

    // The edited step must run: results reflect the new definition, not the reused old children.
    assert!(
        restarted.success,
        "edited raw-flow restart should succeed: {:?}",
        restarted.json_result()
    );
    assert_eq!(restarted.json_result().unwrap(), json!("aX,bX"));
    let new_iter0 = child_job_id_for_step(&db, restarted.id, "fanout", Some(0)).await;
    assert_ne!(
        new_iter0, orig_iter0,
        "the edited fanout step must re-run, not be reused"
    );

    Ok(())
}

/// Only a flow reaped by the zombie monitor (canceled_by = 'monitor') is eligible for reuse. A
/// plain force-cancel at the same boundary (a child succeeded, its parent transition not yet
/// landed) yields the identical InProgress/all-success shape but must keep restart-from-step
/// semantics: the selected step re-runs.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_non_monitor_cancel_is_not_reused(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

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
                            "type": "rawscript", "language": "deno",
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
                    "type": "rawscript", "language": "deno",
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

    // Same frozen-transition shape as a zombie, but canceled by a USER, not the monitor.
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
                m["iterator"] = json!({ "index": 1, "itered_len": 2 });
            }
            Some("after") => *m = json!({ "type": "WaitingForPriorSteps", "id": "after" }),
            _ => {}
        }
    }
    flow_status["step"] = json!(0);
    sqlx::query!(
        "UPDATE v2_job_completed SET status = 'canceled', canceled_by = 'admin',
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

    assert!(restarted.success);
    let new_iter0 = child_job_id_for_step(&db, restarted.id, "fanout", Some(0)).await;
    assert_ne!(
        new_iter0, orig_iter0,
        "a non-monitor cancel must re-run the step, not reuse the child"
    );

    Ok(())
}
