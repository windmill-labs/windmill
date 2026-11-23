//! A failed-only restart of a parallel step reruns only its failed iterations, each in its
//! original slot. Kept in its own binary: the restart checks the worker version, which other
//! test binaries set process-wide.

#![cfg(feature = "deno_core")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::flow_status::{FlowStatus, FlowStatusModule, RestartedFrom};
use windmill_common::flows::FlowValue;
use windmill_common::jobs::JobPayload;
use windmill_test_utils::*;

async fn first_module(db: &Pool<Postgres>, flow_job_id: uuid::Uuid) -> FlowStatusModule {
    let status = sqlx::query_scalar!(
        "SELECT flow_status FROM v2_job_completed WHERE id = $1",
        flow_job_id,
    )
    .fetch_one(db)
    .await
    .unwrap()
    .expect("flow_status missing");
    let status: FlowStatus = serde_json::from_value(status).unwrap();
    status.modules[0].clone()
}

/// A parallel loop over `items` whose second step fails on all but 'x' when run with tag
/// "first". With parallelism 1, the second rerun is queued suspended and must be released by
/// the first one completing.
fn flow(wrap_in_branchone: bool, items: &str) -> FlowValue {
    let parallel_loop = json!({
        "id": "loop",
        "value": {
            "type": "forloopflow",
            "iterator": { "type": "javascript", "expr": items },
            "skip_failures": true,
            "parallel": true,
            "parallelism": { "type": "static", "value": 1 },
            "modules": [{
                "id": "prep",
                "value": {
                    "type": "rawscript",
                    "language": "bun",
                    "input_transforms": {},
                    "content": "export function main() { return 1 }"
                }
            }, {
                "id": "inner",
                "value": {
                    "type": "rawscript",
                    "language": "bun",
                    "input_transforms": {
                        "iter_val": { "type": "javascript", "expr": "flow_input.iter.value" },
                        "tag": { "type": "javascript", "expr": "flow_input.tag" }
                    },
                    "content": "export function main(iter_val: string, tag: string) { if (tag === 'first' && iter_val !== 'x') throw new Error('boom'); return `${tag}:${iter_val}` }"
                }
            }]
        }
    });
    let module = if wrap_in_branchone {
        json!({ "id": "pick", "value": { "type": "branchone", "branches": [], "default": [parallel_loop] } })
    } else {
        parallel_loop
    };
    serde_json::from_value(json!({
        "modules": [module],
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": { "tag": { "type": "string" } },
            "order": ["tag"]
        }
    }))
    .unwrap()
}

async fn run(
    db: &Pool<Postgres>,
    port: u16,
    value: FlowValue,
    tag: &str,
    restarted_from: Option<RestartedFrom>,
) -> CompletedJob {
    RunJob::from(JobPayload::RawFlow { value, path: None, restarted_from })
        .arg("tag", json!(tag))
        .run_until_complete(db, false, port)
        .await
}

/// Only the failed iterations reran, each in its slot and from its failed step: their `prep`
/// step keeps its original job.
async fn assert_only_failed_reran(
    db: &Pool<Postgres>,
    result: serde_json::Value,
    original: &[uuid::Uuid],
    rerun: &[uuid::Uuid],
) {
    assert_eq!(result, json!(["first:x", "second:y", "second:z"]));
    assert_eq!(rerun[0], original[0]);
    for slot in 1..3 {
        assert_ne!(rerun[slot], original[slot]);
        assert_eq!(
            first_module(db, rerun[slot]).await.job(),
            first_module(db, original[slot]).await.job()
        );
    }
}

#[sqlx::test(fixtures("base", "hello"))]
async fn test_restart_parallel_loop_failed_only(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let first_run = run(&db, port, flow(false, "['x', 'y', 'z']"), "first", None).await;
    let original = first_module(&db, first_run.id).await.flow_jobs().unwrap();

    let restart = RestartedFrom {
        flow_job_id: first_run.id,
        step_id: "loop".into(),
        failed_only: true,
        from_failed_step: true,
        ..Default::default()
    };
    // The reruns use the restarted run's items, not the edited iterator's.
    let edited = flow(false, "['z', 'y', 'x']");
    let restarted = run(&db, port, edited, "second", Some(restart)).await;
    let rerun = first_module(&db, restarted.id).await.flow_jobs().unwrap();
    assert_only_failed_reran(&db, restarted.json_result().unwrap(), &original, &rerun).await;
    Ok(())
}

/// A nested restart carries `failed_only` down to the parallel step its chain ends at.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_restart_nested_parallel_loop_failed_only(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let first_run = run(&db, port, flow(true, "['x', 'y', 'z']"), "first", None).await;
    let first_branch = first_module(&db, first_run.id).await.job().unwrap();
    let original = first_module(&db, first_branch).await.flow_jobs().unwrap();

    let restart = RestartedFrom {
        flow_job_id: first_run.id,
        step_id: "pick".into(),
        branch_chosen: first_module(&db, first_run.id).await.branch_chosen(),
        nested: Some(Box::new(RestartedFrom {
            flow_job_id: first_branch,
            step_id: "loop".into(),
            failed_only: true,
            from_failed_step: true,
            ..Default::default()
        })),
        ..Default::default()
    };
    let restarted = run(
        &db,
        port,
        flow(true, "['x', 'y', 'z']"),
        "second",
        Some(restart),
    )
    .await;
    let branch = first_module(&db, restarted.id).await.job().unwrap();
    let rerun = first_module(&db, branch).await.flow_jobs().unwrap();
    assert_only_failed_reran(&db, restarted.json_result().unwrap(), &original, &rerun).await;
    Ok(())
}

/// A restart reuses earlier steps by position and runs only the steps the original run had, so
/// an inline loop body whose steps changed starts each failed iteration over.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_restart_edited_body_starts_failed_iterations_over(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let first_run = run(&db, port, flow(false, "['x', 'y', 'z']"), "first", None).await;
    let original = first_module(&db, first_run.id).await.flow_jobs().unwrap();

    // A step appended after the failed `inner`, returning what `inner` does.
    let mut edited = serde_json::to_value(flow(false, "['x', 'y', 'z']"))?;
    let body = edited["modules"][0]["value"]["modules"]
        .as_array_mut()
        .unwrap();
    let mut extra = body[1].clone();
    extra["id"] = json!("extra");
    body.push(extra);
    let edited: FlowValue = serde_json::from_value(edited)?;

    let restart = RestartedFrom {
        flow_job_id: first_run.id,
        step_id: "loop".into(),
        failed_only: true,
        from_failed_step: true,
        ..Default::default()
    };
    let restarted = run(&db, port, edited, "second", Some(restart)).await;
    assert_eq!(
        restarted.json_result().unwrap(),
        json!(["first:x", "second:y", "second:z"])
    );
    let rerun = first_module(&db, restarted.id).await.flow_jobs().unwrap();
    for slot in 1..3 {
        assert_ne!(
            first_module(&db, rerun[slot]).await.job(),
            first_module(&db, original[slot]).await.job()
        );
    }
    Ok(())
}

/// Without `from_failed_step`, each failed iteration reruns from its first step.
#[sqlx::test(fixtures("base", "hello"))]
async fn test_restart_failed_only_starts_over_by_default(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let first_run = run(&db, port, flow(false, "['x', 'y', 'z']"), "first", None).await;
    let original = first_module(&db, first_run.id).await.flow_jobs().unwrap();

    let restart = RestartedFrom {
        flow_job_id: first_run.id,
        step_id: "loop".into(),
        failed_only: true,
        ..Default::default()
    };
    let restarted = run(
        &db,
        port,
        flow(false, "['x', 'y', 'z']"),
        "second",
        Some(restart),
    )
    .await;
    assert_eq!(
        restarted.json_result().unwrap(),
        json!(["first:x", "second:y", "second:z"])
    );
    let rerun = first_module(&db, restarted.id).await.flow_jobs().unwrap();
    assert_eq!(rerun[0], original[0]);
    for slot in 1..3 {
        assert_ne!(
            first_module(&db, rerun[slot]).await.job(),
            first_module(&db, original[slot]).await.job()
        );
    }
    Ok(())
}
