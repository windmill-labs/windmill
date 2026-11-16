//! A builder choosing a Hub step in a flow, an agent or an app is held to the run-time rule for
//! naming a Hub path: refused without `code`, allowed with it.

use reqwest::multipart;
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const CODER: &str = "SECRET_TOKEN_2";
const OP: &str = "SECRET_TOKEN_3";
const HUB_PATH: &str = "hub/990000794/test/echo";
const REFUSAL: &str = "Hub scripts and flows require a role that can write code";

fn flow(path: &str) -> Value {
    json!({"modules": [{"id": "a", "value": {"type": "script", "path": path, "input_transforms": {}}}]})
}

#[sqlx::test(fixtures("base"))]
async fn a_hub_step_follows_the_code_capability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        r#"UPDATE workspace_settings SET role_overrides = '{
             "operator": {"capabilities": {"flows": "compose", "apps": "compose"}},
             "developer": {"capabilities": {"flows": "compose", "apps": "compose"}}
           }' WHERE workspace_id = 'test-workspace';
           UPDATE usr SET operator = true WHERE username = 'test-user-3';"#,
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    let send = |token: &'static str, case: &'static str| {
        let (ws, client) = (ws.clone(), client.clone());
        let user = if token == OP {
            "test-user-3"
        } else {
            "test-user-2"
        };
        async move {
            let req = match case {
                "flow create" => client.post(format!("{ws}/flows/create")).json(&json!({
                    "path": format!("u/{user}/hub_step"), "summary": "", "value": flow(HUB_PATH),
                })),
                "flow preview" => client
                    .post(format!("{ws}/jobs/run/preview_flow"))
                    .json(&json!({ "value": flow(HUB_PATH), "args": {} })),
                "agent write" => client.post(format!("{ws}/resources/create")).json(&json!({
                    "path": format!("u/{user}/agent"), "resource_type": "ai_agent",
                    "value": {"tools": [{"id": "t", "value": {"tool_type": "flowmodule",
                        "type": "script", "path": HUB_PATH, "input_transforms": {}}}]},
                })),
                "app create" => {
                    let app = json!({
                        "path": format!("u/{user}/app"), "summary": "",
                        "value": {"files": {}, "runnables": {"a": {"name": "a",
                            "type": "runnableByPath", "path": HUB_PATH, "runType": "hubscript"}}},
                        "policy": {"execution_mode": "publisher", "sandbox": true,
                            "triggerables_v2": {}},
                    });
                    client.post(format!("{ws}/apps/create_raw")).multipart(
                        multipart::Form::new()
                            .part(
                                "app",
                                multipart::Part::text(app.to_string())
                                    .mime_str("application/json")
                                    .unwrap(),
                            )
                            .part("js", multipart::Part::text("console.log('app')")),
                    )
                }
                // A preset handler is allowed at run time to every role, so it is as a step.
                "flow create, preset handler" => {
                    client.post(format!("{ws}/flows/create")).json(&json!({
                        "path": format!("u/{user}/preset_step"), "summary": "",
                        "value": flow("hub/2431/slack/schedule-error-handler-slack"),
                    }))
                }
                _ => unreachable!(),
            };
            let resp = req.bearer_auth(token).send().await.unwrap();
            (resp.status(), resp.text().await.unwrap())
        }
    };

    for case in ["flow create", "flow preview", "agent write", "app create"] {
        let (status, body) = send(OP, case).await;
        assert!(
            status == 403 && body.contains(&format!("{HUB_PATH}: {REFUSAL}")),
            "operator, {case}: {status} {body}"
        );
        let (status, body) = send(CODER, case).await;
        assert!(status.is_success(), "code role, {case}: {status} {body}");
    }
    let (status, body) = send(OP, "flow create, preset handler").await;
    assert!(
        status.is_success(),
        "operator, preset handler: {status} {body}"
    );
    Ok(())
}
