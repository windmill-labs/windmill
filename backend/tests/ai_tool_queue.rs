use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::{
    completed_job, in_test_worker, listen_for_completed_jobs, ApiServer, RunJob, StreamFind,
};

async fn model(Json(body): Json<Value>) -> ([(&'static str, &'static str); 1], String) {
    let finished = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "tool");
    let delta = if finished {
        json!({"role":"assistant", "content":"done"})
    } else {
        json!({"role":"assistant", "tool_calls": (0..3).map(|i| json!({
            "index":i, "id":format!("call_{i}"), "type":"function",
            "function":{"name":format!("tool_{i}"), "arguments":"{}"}
        })).collect::<Vec<_>>()})
    };
    let event = json!({"choices":[{"index":0, "delta":delta, "finish_reason":null}]});
    let end = json!({"choices":[{"index":0, "delta":{}, "finish_reason":if finished {"stop"} else {"tool_calls"}}]});
    (
        [("content-type", "text/event-stream")],
        format!("data: {event}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
    )
}

async fn run_batch(db: Pool<Postgres>, parallel: bool, limited: bool) -> anyhow::Result<()> {
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let server = ApiServer::start(db.clone()).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let gate = Arc::new(Notify::new());
    let router = Router::new()
        .route("/v1/chat/completions", post(model))
        .route(
            "/first",
            get(|State(gate): State<Arc<Notify>>| async move {
                gate.notified().await;
                "ok"
            }),
        )
        .route(
            "/second",
            get(|State(gate): State<Arc<Notify>>| async move {
                gate.notify_one();
                "ok"
            }),
        )
        .with_state(gate);
    let stub = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let tools: Vec<Value> = (0..3).map(|i| {
        let wait = if parallel && !limited && i < 2 {
            format!("await fetch('{base}/{}');", if i == 0 {"first"} else {"second"})
        } else { String::new() };
        let finish = if i == 2 { "throw new Error('expected tool failure');".to_string() }
            else { format!("return {i};") };
        json!({"id":format!("t{i}"),"summary":format!("tool_{i}"),"value":{
            "type":"rawscript", "language":"bun", "input_transforms":{},
            "tag": if parallel { "bun" } else { "unserved-tool-tag" },
            "concurrent_limit": if limited { Some(1) } else { None },
            "custom_concurrency_key": if limited { Some("agent-tool-test") } else { None },
            "content":format!("export async function main() {{ {wait} await Bun.sleep({}); {finish} }}", if i == 0 {200} else {0})
        }})
    }).collect();
    let flow: FlowValue = serde_json::from_value(json!({"modules":[{"id":"agent","value":{
        "type":"aiagent", "tools":tools, "input_transforms":{
            "provider":{"type":"static","value":{"kind":"customai","model":"queue-test","resource":{"base_url":format!("{base}/v1")}}},
            "user_message":{"type":"static","value":"run the tools"},
            "max_iterations":{"type":"static","value":3}
        }
    }}]}))?;
    let id = RunJob::from(JobPayload::RawFlow {
        value: flow,
        path: Some("u/test/agent_queue".into()),
        restarted_from: None,
    })
    .push(&db)
    .await;
    let notifications = listen_for_completed_jobs(&db).await;
    let wait = async {
        if parallel {
            in_test_worker(&db, notifications.find(&id), server.addr.port()).await;
        } else {
            notifications.find(&id).await;
        }
    };
    tokio::time::timeout(
        Duration::from_secs(45),
        in_test_worker(&db, wait, server.addr.port()),
    )
    .await?;
    let result = completed_job(id, &db).await;
    assert!(result.success, "{:?}", result.result);
    let result = result.json_result().expect("agent result");
    let messages: Vec<_> = result["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["role"] == "tool")
        .collect();
    assert_eq!(messages.len(), 3);
    for (index, message) in messages.iter().enumerate() {
        assert_eq!(message["tool_call_id"], format!("call_{index}"));
    }
    assert_eq!(messages[0]["content"], "0");
    assert_eq!(messages[1]["content"], "1");
    assert!(messages[2]["content"]
        .as_str()
        .unwrap()
        .contains("expected tool failure"));
    let (parent_id, parent_worker): (uuid::Uuid, String) = sqlx::query_as(
        "SELECT j.id, c.worker FROM v2_job j JOIN v2_job_completed c USING(id) WHERE j.parent_job = $1"
    ).bind(id).fetch_one(&db).await?;
    let children: Vec<(String, String, bool)> = sqlx::query_as(
        "SELECT j.runnable_path, c.worker, c.status = 'success' FROM v2_job j JOIN v2_job_completed c USING(id) WHERE j.parent_job = $1 ORDER BY j.runnable_path"
    ).bind(parent_id).fetch_all(&db).await?;
    assert_eq!(children.len(), 3);
    assert_eq!(
        children[0].1, parent_worker,
        "first tool must stay on the parent worker"
    );
    assert_eq!(
        children.iter().map(|c| c.2).collect::<Vec<_>>(),
        vec![true, true, false]
    );
    if limited {
        let overlapping: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM v2_job j1 JOIN v2_job_completed c1 ON c1.id = j1.id
            JOIN v2_job j2 ON j2.parent_job = j1.parent_job AND j2.id > j1.id
            JOIN v2_job_completed c2 ON c2.id = j2.id
            WHERE j1.parent_job = $1 AND c1.started_at < c2.completed_at AND c2.started_at < c1.completed_at"
        ).bind(parent_id).fetch_one(&db).await?;
        assert_eq!(
            overlapping, 0,
            "the reserved first job must count toward the shared limit"
        );
    } else if parallel {
        assert_ne!(
            children[1].1, parent_worker,
            "second tool must unblock the first from another worker"
        );
    } else {
        assert!(children.iter().all(|child| child.1 == parent_worker));
    }
    stub.abort();
    server.close().await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn parent_drains_tools_without_another_worker(db: Pool<Postgres>) -> anyhow::Result<()> {
    run_batch(db, false, false).await
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn parent_keeps_first_tool_while_another_worker_runs_siblings(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    run_batch(db, true, false).await
}

#[cfg(all(feature = "enterprise", feature = "private"))]
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn reserved_tool_obeys_shared_concurrency_limit(db: Pool<Postgres>) -> anyhow::Result<()> {
    run_batch(db, true, true).await
}
