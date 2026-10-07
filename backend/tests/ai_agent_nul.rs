use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use std::time::Duration;
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::{
    completed_job, in_test_worker, listen_for_completed_jobs, ApiServer, RunJob, StreamFind,
};

const MEMORY_ID: &str = "0199c0de-0000-7000-8000-000000000001";

/// A model that puts a U+0000 in its tool call arguments, then in its answer.
async fn model(Json(body): Json<Value>) -> ([(&'static str, &'static str); 1], String) {
    let finished = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "tool");
    let delta = if finished {
        json!({"role":"assistant", "content":"are\u{0}tired"})
    } else {
        json!({"role":"assistant", "tool_calls":[{
            "index":0, "id":"call_0", "type":"function",
            "function":{"name":"echo", "arguments":"{\"x\":\"a\\u0000b\"}"}
        }]})
    };
    let event = json!({"choices":[{"index":0, "delta":delta, "finish_reason":null}]});
    let end = json!({"choices":[{"index":0, "delta":{}, "finish_reason":if finished {"stop"} else {"tool_calls"}}]});
    (
        [("content-type", "text/event-stream")],
        format!("data: {event}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
    )
}

async fn run_flow(db: &Pool<Postgres>, modules: Value) -> anyhow::Result<Value> {
    let server = ApiServer::start(db.clone()).await?;
    let flow: FlowValue = serde_json::from_value(json!({ "modules": modules }))?;
    let id = RunJob::from(JobPayload::RawFlow {
        value: flow,
        path: Some("u/test/nul".into()),
        restarted_from: None,
    })
    .push(db)
    .await;
    let notifications = listen_for_completed_jobs(db).await;
    tokio::time::timeout(
        Duration::from_secs(45),
        in_test_worker(db, notifications.find(&id), server.addr.port()),
    )
    .await?;
    let job = completed_job(id, db).await;
    server.close().await?;
    assert!(job.success, "{:?}", job.result);
    Ok(job.json_result().expect("flow result"))
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn agent_step_survives_nul_in_model_output(db: Pool<Postgres>) -> anyhow::Result<()> {
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let stub = tokio::spawn(async move {
        let router = Router::new().route("/v1/chat/completions", post(model));
        axum::serve(listener, router).await.unwrap();
    });
    let result = run_flow(
        &db,
        json!([{"id":"agent","value":{
            "type":"aiagent",
            "tools":[{"id":"t0","summary":"echo","value":{
                "type":"rawscript", "language":"bun", "input_transforms":{},
                "content":"export async function main(x: string) { return x }"
            }}],
            "input_transforms":{
                "provider":{"type":"static","value":{"kind":"customai","model":"nul-test","resource":{"base_url":format!("{base}/v1")}}},
                "user_message":{"type":"static","value":"echo"},
                "memory":{"type":"static","value":{"kind":"auto","context_length":10,"memory_id":MEMORY_ID}}
            }
        }}]),
    )
    .await?;
    stub.abort();
    assert_eq!(result["output"], "aretired");
    let tool_message = result["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool message");
    assert_eq!(tool_message["content"], "\"ab\"");
    // The model's own words are also written to a jsonb column the job result never passes through.
    let memory: String = sqlx::query_scalar(
        "SELECT messages::text FROM ai_agent_memory WHERE conversation_id = $1::uuid",
    )
    .bind(MEMORY_ID)
    .fetch_one(&db)
    .await?;
    assert!(memory.contains("aretired"), "{memory}");
    Ok(())
}

/// A step's args are evaluated from the previous step's result as the worker holds it, not as
/// the completed job stored it.
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn flow_step_args_survive_nul_in_previous_result(db: Pool<Postgres>) -> anyhow::Result<()> {
    let result = run_flow(
        &db,
        json!([
            {"id":"a","value":{
                "type":"rawscript", "language":"bun", "input_transforms":{},
                "content":"export async function main() { return 'a\\u0000b' }"
            }},
            {"id":"b","value":{
                "type":"rawscript", "language":"bun",
                "input_transforms":{"x":{"type":"javascript","expr":"results.a"}},
                "content":"export async function main(x: string) { return x }"
            }}
        ]),
    )
    .await?;
    assert_eq!(result, "ab");
    Ok(())
}
