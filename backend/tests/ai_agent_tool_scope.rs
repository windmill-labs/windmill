use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use std::time::Duration;
use uuid::Uuid;
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::{
    completed_job, in_test_worker, listen_for_completed_jobs, ApiServer, RunJob, StreamFind,
};

const CONVERSATION: Uuid = Uuid::from_u128(0xc0de);

/// A model that calls every tool it is offered once, then answers.
async fn model(Json(body): Json<Value>) -> ([(&'static str, &'static str); 1], String) {
    let finished = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "tool");
    let delta = if finished {
        json!({"role":"assistant", "content":"done"})
    } else {
        let calls: Vec<Value> = body["tools"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, tool)| {
                json!({
                    "index":i, "id":format!("call_{i}"), "type":"function",
                    "function":{"name":tool["function"]["name"], "arguments":"{}"}
                })
            })
            .collect();
        json!({"role":"assistant", "tool_calls":calls})
    };
    let event = json!({"choices":[{"index":0, "delta":delta, "finish_reason":null}]});
    let end = json!({"choices":[{"index":0, "delta":{}, "finish_reason":if finished {"stop"} else {"tool_calls"}}]});
    (
        [("content-type", "text/event-stream")],
        format!("data: {event}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
    )
}

fn record_tool(id: &str) -> Value {
    json!({"id":id,"summary":"record","value":{
        "type":"rawscript", "language":"bun",
        "input_transforms":{
            "topic":{"type":"javascript","expr":"flow_input.iter.value"},
            "prev":{"type":"javascript","expr":"results.a"},
            "pr":{"type":"javascript","expr":"previous_result"},
            "state":{"type":"javascript","expr":"flow_user_state('k')"}
        },
        "content":"export async function main(topic: string, prev: string, pr: string, state: string) { return `${topic}|${prev}|${pr}|${state}` }"
    }})
}

/// A loop body runs as its own flow job, as a subflow does: a tool's `flow_input` and `results`
/// must be that flow's, as a regular step's are, not the root run's. A nested agent's tools read
/// the same flow, past the agent calling it, and the run's user state. An agent first in its flow
/// gets that flow's args as `previous_result`, as a regular first step does. In a chat run, only the calling agent writes to the
/// conversation, and every row it writes belongs to the run.
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn agent_tool_reads_inputs_of_the_flow_running_the_step(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let server = ApiServer::start(db.clone()).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let stub = tokio::spawn(async move {
        let router = Router::new().route("/v1/chat/completions", post(model));
        axum::serve(listener, router).await.unwrap();
    });
    let agent_inputs = json!({
        "provider":{"type":"static","value":{"kind":"customai","model":"scope-test","resource":{"base_url":format!("{base}/v1")}}},
        "user_message":{"type":"static","value":"record"}
    });
    let flow: FlowValue = serde_json::from_value(
        json!({"chat_input_enabled":true,"modules":[{"id":"loop","value":{
            "type":"forloopflow",
            "iterator":{"type":"javascript","expr":"['t1', 't2']"},
            "skip_failures":false,
            "modules":[
                {"id":"first","value":{
                    "type":"aiagent",
                    "tools":[{"id":"first_record","summary":"record","value":{
                        "type":"rawscript", "language":"bun",
                        "input_transforms":{"pr":{"type":"javascript","expr":"previous_result"}},
                        "content":"export async function main(pr: any) { return `first|${pr.iter.value}` }"
                    }}],
                    "input_transforms":agent_inputs.clone()
                }},
                {"id":"a","value":{
                    "type":"rawscript", "language":"bun",
                    "input_transforms":{"topic":{"type":"javascript","expr":"flow_input.iter.value"}},
                    "content":"export async function main(topic: string) { return 'seed-' + topic }"
                }},
                {"id":"agent","value":{
                    "type":"aiagent",
                    "tools":[
                        record_tool("record"),
                        {"id":"inner","summary":"inner","value":{
                            "type":"aiagent",
                            "tools":[record_tool("inner_record")],
                            "input_transforms":agent_inputs.clone()
                        }}
                    ],
                    "input_transforms":agent_inputs
                }}
            ]
        }}]}),
    )?;
    let id = RunJob::from(JobPayload::RawFlow {
        value: flow,
        path: Some("u/test/agent_tool_scope".into()),
        restarted_from: None,
    })
    .push(&db)
    .await;
    // What the API does for a chat turn: open the conversation and give the run its id.
    sqlx::query(
        "INSERT INTO flow_conversation (id, workspace_id, flow_path, created_by)
         VALUES ($1, 'test-workspace', 'u/test/agent_tool_scope', 'test-user')",
    )
    .bind(CONVERSATION)
    .execute(&db)
    .await?;
    sqlx::query(
        "UPDATE v2_job_status SET flow_status = jsonb_set(flow_status, '{memory_id}', to_jsonb($2::uuid))
            || jsonb_build_object('user_states', jsonb_build_object('k', 'v'))
         WHERE id = $1",
    )
    .bind(id)
    .bind(CONVERSATION)
    .execute(&db)
    .await?;
    let notifications = listen_for_completed_jobs(&db).await;
    tokio::time::timeout(
        Duration::from_secs(60),
        in_test_worker(&db, notifications.find(&id), server.addr.port()),
    )
    .await?;
    let job = completed_job(id, &db).await;
    stub.abort();
    server.close().await?;
    assert!(job.success, "{:?}", job.result);
    let recorded: Vec<(String, Value)> = sqlx::query_as(
        "SELECT j.runnable_path, c.result FROM v2_job j JOIN v2_job_completed c USING(id)
        WHERE j.runnable_path LIKE '%/tools/%record' ORDER BY c.result::text, j.runnable_path",
    )
    .fetch_all(&db)
    .await?
    .into_iter()
    .map(|(path, result): (String, sqlx::types::Json<Value>)| (path, result.0))
    .collect();
    let results: Vec<&Value> = recorded.iter().map(|(_, r)| r).collect();
    assert_eq!(
        results,
        vec![
            &json!("first|t1"),
            &json!("first|t2"),
            &json!("t1|seed-t1|seed-t1|v"),
            &json!("t1|seed-t1|seed-t1|v"),
            &json!("t2|seed-t2|seed-t2|v"),
            &json!("t2|seed-t2|seed-t2|v")
        ],
        "{recorded:?}"
    );
    // A row the nested agent wrote would carry the calling agent's job as its turn.
    let turns: Vec<Uuid> = sqlx::query_scalar(
        "SELECT turn_job_id FROM flow_conversation_message WHERE conversation_id = $1",
    )
    .bind(CONVERSATION)
    .fetch_all(&db)
    .await?;
    assert!(!turns.is_empty());
    assert!(turns.iter().all(|turn| *turn == id), "{turns:?}");
    Ok(())
}
