use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use sqlx::{types::Uuid, Pool, Postgres};
use std::time::Duration;
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::{
    completed_job, in_test_worker, listen_for_completed_jobs, ApiServer, RunJob, StreamFind,
};

async fn model(Json(body): Json<Value>) -> ([(&'static str, &'static str); 1], String) {
    let answered = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "tool");
    let delta = if answered {
        json!({"role":"assistant", "content":"done"})
    } else {
        json!({"role":"assistant", "tool_calls":[{
            "index":0, "id":"call_0", "type":"function",
            "function":{"name":"delete_database", "arguments":"{}"}
        }]})
    };
    let event = json!({"choices":[{"index":0, "delta":delta, "finish_reason":null}]});
    let end = json!({"choices":[{"index":0, "delta":{}, "finish_reason":if answered {"stop"} else {"tool_calls"}}]});
    (
        [("content-type", "text/event-stream")],
        format!("data: {event}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
    )
}

/// The agent parks on a call that requires approval, is answered through the endpoint the run
/// page and the chat use, and resumes: an approved call runs, a rejected one never does.
async fn answer(db: Pool<Postgres>, approved: bool) -> anyhow::Result<Value> {
    std::env::set_var("ALLOW_PRIVATE_AI_BASE_URLS", "true");
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let base = format!("http://{}", listener.local_addr()?);
    let stub = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route("/v1/chat/completions", post(model)),
        )
        .await
        .unwrap();
    });
    let flow: FlowValue = serde_json::from_value(json!({"modules":[{"id":"agent","value":{
        "type":"aiagent",
        "tools":[{"id":"t0", "summary":"delete_database", "requires_approval":true, "value":{
            "type":"rawscript", "language":"bun", "input_transforms":{},
            "content":"export async function main() { return 'ran' }"
        }}],
        "input_transforms":{
            "provider":{"type":"static","value":{"kind":"customai","model":"approval-test","resource":{"base_url":format!("{base}/v1")}}},
            "user_message":{"type":"static","value":"delete it"},
            "max_iterations":{"type":"static","value":3}
        }
    }}]}))?;
    let id = RunJob::from(JobPayload::RawFlow {
        value: flow,
        path: Some("u/test/agent_approval".into()),
        restarted_from: None,
    })
    .push(&db)
    .await;
    let notifications = listen_for_completed_jobs(&db).await;
    let db_ = db.clone();
    let answered = async move {
        let agent = loop {
            let parked: Option<Uuid> = sqlx::query_scalar(
                "SELECT q.id FROM v2_job_queue q JOIN v2_job j USING (id)
                JOIN ai_agent_checkpoint c ON c.job_id = q.id
                WHERE j.parent_job = $1 AND q.suspend > 0",
            )
            .bind(id)
            .fetch_optional(&db_)
            .await
            .unwrap();
            if let Some(agent) = parked {
                break agent;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        };
        let token = windmill_common::auth::create_token_for_owner(
            &db_,
            "test-workspace",
            "u/test-user",
            "approval-test",
            100,
            "test@windmill.dev",
            &Uuid::nil(),
            None,
            None,
        )
        .await
        .unwrap();
        let response = reqwest::Client::new()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/jobs_u/flow/resume_suspended/{agent}"
            ))
            .bearer_auth(token)
            .json(&json!({"approved":approved, "payload":{"call_id":"call_0", "comment":"not today"}}))
            .send()
            .await
            .unwrap();
        assert!(
            response.status().is_success(),
            "{}",
            response.text().await.unwrap()
        );
        notifications.find(&id).await;
    };
    tokio::time::timeout(Duration::from_secs(60), in_test_worker(&db, answered, port)).await?;

    let result = completed_job(id, &db).await;
    assert!(result.success, "{:?}", result.result);
    let tool_jobs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM v2_job WHERE parent_job IN (SELECT id FROM v2_job WHERE parent_job = $1)",
    )
    .bind(id)
    .fetch_one(&db)
    .await?;
    assert_eq!(tool_jobs, approved as i64, "only an approved call runs");
    stub.abort();
    server.close().await?;
    let result = result.json_result().expect("agent result");
    Ok(result["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("the call's answer")["content"]
        .clone())
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn approved_call_runs_after_the_agent_resumes(db: Pool<Postgres>) -> anyhow::Result<()> {
    assert_eq!(answer(db, true).await?, "\"ran\"");
    Ok(())
}

#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn rejected_call_reaches_the_model_as_not_run(db: Pool<Postgres>) -> anyhow::Result<()> {
    let content = answer(db, false).await?;
    let content = content.as_str().unwrap();
    assert!(
        content.starts_with("Not run") && content.contains("not today"),
        "{content}"
    );
    Ok(())
}
