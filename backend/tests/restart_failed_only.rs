//! A failed-only restart of a parallel step reruns only its failed iterations, each in its
//! original slot. Kept in its own binary: the restart checks the worker version, which other
//! test binaries set process-wide.

#![cfg(feature = "deno_core")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::flow_status::{FlowStatus, RestartedFrom};
use windmill_common::flows::FlowValue;
use windmill_common::jobs::JobPayload;
use windmill_test_utils::*;

async fn loop_children(db: &Pool<Postgres>, flow_job_id: uuid::Uuid) -> Vec<uuid::Uuid> {
    let status = sqlx::query_scalar!(
        "SELECT flow_status FROM v2_job_completed WHERE id = $1",
        flow_job_id,
    )
    .fetch_one(db)
    .await
    .unwrap()
    .expect("flow_status missing");
    let status: FlowStatus = serde_json::from_value(status).unwrap();
    status.modules[0]
        .flow_jobs()
        .expect("loop has no flow_jobs")
}

#[sqlx::test(fixtures("base", "hello"))]
async fn test_restart_parallel_loop_failed_only(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // With parallelism 1, the second rerun is queued suspended and must be released by the
    // first one completing.
    let flow_value: FlowValue = serde_json::from_value(json!({
        "modules": [{
            "id": "loop",
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "javascript", "expr": "['x', 'y', 'z']" },
                "skip_failures": true,
                "parallel": true,
                "parallelism": { "type": "static", "value": 1 },
                "modules": [{
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
        }],
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": { "tag": { "type": "string" } },
            "order": ["tag"]
        }
    }))
    .unwrap();

    let first_run = RunJob::from(JobPayload::RawFlow {
        value: flow_value.clone(),
        path: None,
        restarted_from: None,
    })
    .arg("tag", json!("first"))
    .run_until_complete(&db, false, port)
    .await;
    let original = loop_children(&db, first_run.id).await;

    let restarted = RunJob::from(JobPayload::RawFlow {
        value: flow_value,
        path: None,
        restarted_from: Some(RestartedFrom {
            flow_job_id: first_run.id,
            step_id: "loop".into(),
            failed_only: true,
            ..Default::default()
        }),
    })
    .arg("tag", json!("second"))
    .run_until_complete(&db, false, port)
    .await;
    assert_eq!(
        restarted.json_result().unwrap(),
        json!(["first:x", "second:y", "second:z"])
    );
    let rerun = loop_children(&db, restarted.id).await;
    assert_eq!(rerun[0], original[0]);
    assert_ne!(rerun[1], original[1]);
    assert_ne!(rerun[2], original[2]);
    Ok(())
}
