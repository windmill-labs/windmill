//! An app viewer names a flow's memory with `run_query_params.memory_id`, and that id must
//! never reach another viewer's memory: a chat-mode flow files the turn in a conversation
//! owned by the viewer, and any other flow keys its memory per app and viewer.
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)
//!   test-user-2 (developer, token SECRET_TOKEN_2)

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const TOKENS: [&str; 2] = ["SECRET_TOKEN", "SECRET_TOKEN_2"];
const MEMORY_ID: &str = "8b7c3d52-0a4e-4f6b-9d1a-2c3e4f5a6b7c";

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Deploys a flow and an app that runs it.
async fn setup(ws: &str, flow_path: &str, chat: bool) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let resp = authed(client.post(format!("{ws}/flows/create")), TOKENS[0])
        .json(&json!({
            "path": flow_path,
            "summary": "",
            "value": { "modules": [], "chat_input_enabled": chat },
            "schema": { "type": "object", "properties": {} }
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    let resp = authed(client.post(format!("{ws}/apps/create")), TOKENS[0])
        .json(&json!({
            "path": format!("{flow_path}_app"),
            "summary": "",
            "value": {},
            "policy": {
                "execution_mode": "anonymous",
                "triggerables_v2": {
                    format!("run:flow/{flow_path}"): { "static_inputs": {}, "one_of_inputs": {} }
                }
            }
        }))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    Ok(())
}

async fn run(ws: &str, token: &str, flow_path: &str) -> anyhow::Result<reqwest::Response> {
    Ok(authed(
        reqwest::Client::new().post(format!("{ws}/apps_u/execute_component/{flow_path}_app")),
        token,
    )
    .json(&json!({
        "component": "run",
        "path": format!("flow/{flow_path}"),
        "args": { "user_message": "hi" },
        "run_query_params": { "memory_id": MEMORY_ID }
    }))
    .send()
    .await?)
}

// One test, one database: the server caches flow versions by id across the tests of a
// process, and two fresh databases number their first flow version alike.
#[sqlx::test(fixtures("base"))]
async fn a_viewer_cannot_reach_anothers_flow_memory(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );

    let plain = "u/test-user/plain";
    setup(&ws, plain, false).await?;
    let mut keys = vec![];
    for token in TOKENS {
        let resp = run(&ws, token, plain).await?;
        assert!(resp.status().is_success(), "{}", resp.text().await?);
        let job_id: uuid::Uuid = resp.text().await?.parse()?;
        let key: Option<String> =
            sqlx::query_scalar("SELECT flow_status->>'memory_id' FROM v2_job_status WHERE id = $1")
                .bind(job_id)
                .fetch_one(&db)
                .await?;
        keys.push(key.expect("the run keeps a memory key"));
    }
    assert_ne!(
        keys[0], keys[1],
        "two viewers naming one id share no memory"
    );
    assert!(
        keys.iter().all(|k| k != MEMORY_ID),
        "the named id is never the key"
    );

    let chat = "u/test-user/chat";
    setup(&ws, chat, true).await?;
    let resp = run(&ws, TOKENS[0], chat).await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);
    let owner: String =
        sqlx::query_scalar("SELECT created_by FROM flow_conversation WHERE id = $1::uuid")
            .bind(MEMORY_ID)
            .fetch_one(&db)
            .await?;
    assert_eq!(owner, "test-user");
    let resp = run(&ws, TOKENS[1], chat).await?;
    assert!(
        !resp.status().is_success(),
        "another viewer's turn is refused"
    );
    Ok(())
}
