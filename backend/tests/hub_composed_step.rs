//! A builder choosing a Hub step in a flow, an agent or an app is held to the run-time rule for
//! naming a Hub path: refused without `code` or a grant, allowed with either.

use reqwest::multipart;
use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const CODER: &str = "SECRET_TOKEN_2";
const OP: &str = "SECRET_TOKEN_3";
const HUB_PATH: &str = "hub/990000794/test/echo";
const REFUSAL: &str = "Your role may not use this Hub script";

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

/// An agent save checks its Hub tools inside its transaction, so the check must not need a second
/// connection: on a one-connection pool, with the member's role never read before, the save is
/// refused rather than left waiting, from a job's token (sent first, while nothing is cached) as from
/// a personal one. Its own operator, so no other test has warmed the role cache for it; the
/// overrides match the other test's, since the workspace's roles are cached across tests.
#[sqlx::test(fixtures("base"))]
async fn a_hub_step_is_refused_on_a_one_connection_pool(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        r#"UPDATE workspace_settings SET role_overrides = '{
             "operator": {"capabilities": {"flows": "compose", "apps": "compose"}},
             "developer": {"capabilities": {"flows": "compose", "apps": "compose"}}
           }' WHERE workspace_id = 'test-workspace';
           INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
             VALUES ('solo@windmill.dev', 'x', 'password', false, true, 'Solo');
           INSERT INTO usr(workspace_id, email, username, is_admin, operator, role)
             VALUES ('test-workspace', 'solo@windmill.dev', 'solo', false, true, 'Operator');
           INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
             VALUES (encode(sha256('SOLO_TOKEN'::bytea), 'hex'), 'SOLO_TOKEN', 'SOLO_TOKEN',
                     'solo@windmill.dev', 'solo', false);"#,
    )
    .execute(&db)
    .await?;
    let one = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_with((*db.connect_options()).clone())
        .await?;
    let server = ApiServer::start(one).await?;
    set_jwt_secret().await;
    let job_token = windmill_common::auth::create_jwt_token(
        windmill_common::db::Authed {
            email: "solo@windmill.dev".to_string(),
            username: "solo".to_string(),
            is_admin: false,
            is_operator: true,
            groups: vec![],
            folders: vec![],
            scopes: None,
            token_prefix: None,
            mcp_client: None,
            capabilities: None,
        },
        "test-workspace",
        3600,
        None,
        None,
        None,
        None,
    )
    .await?;
    for token in [job_token, "SOLO_TOKEN".to_string()] {
        let resp = reqwest::Client::new()
            .post(format!(
                "http://localhost:{}/api/w/test-workspace/resources/create",
                server.addr.port()
            ))
            .bearer_auth(&token)
            .json(&json!({
                "path": "u/solo/agent", "resource_type": "ai_agent",
                "value": {"tools": [{"id": "t", "value": {"tool_type": "flowmodule",
                    "type": "script", "path": HUB_PATH, "input_transforms": {}}}]},
            }))
            .send()
            .await?;
        let (status, body) = (resp.status(), resp.text().await?);
        assert!(
            status == 403 && body.contains(&format!("{HUB_PATH}: {REFUSAL}")),
            "{status} {body}"
        );
    }
    Ok(())
}

/// A save checks its Hub steps inside its transaction, so the Hub lookup must not need a second
/// connection: with the only one of the pool held, a granted step passes and a step the list does
/// not cover is refused with the reason. The Hub is asked once, whatever the steps.
#[sqlx::test(fixtures("base"))]
async fn the_hub_check_needs_no_second_connection(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let meta = |id: i32, read_only: bool| {
        json!({ "version_id": id, "ask_id": 1, "app": "Slack", "read_only": read_only,
            "script_approved": true, "version_ever_approved": true })
    };
    let (_, calls) = start_stub_hub(vec![
        meta(990000821, true),
        meta(990000822, false),
        meta(990000823, true),
    ])
    .await;
    let one = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect_with((*db.connect_options()).clone())
        .await?;
    let authed = windmill_api_auth::ApiAuthed {
        email: "test3@windmill.dev".into(),
        username: "test-user-3".into(),
        is_operator: true,
        capabilities: Some(windmill_common::roles::RoleCapabilities {
            hub: serde_json::from_value(json!({"slack": "read"}))?,
            ..windmill_common::roles::RoleCapabilities::OPERATOR
        }),
        ..Default::default()
    };
    for (ids, refusal) in [
        ([990000821, 990000823], None),
        (
            [990000821, 990000822],
            Some("hub/990000822/slack/list: This Hub script can write"),
        ),
    ] {
        let steps = ids
            .iter()
            .map(|id| {
                windmill_common::runnable_ref::RunnableRef::from_path(
                    &format!("hub/{id}/slack/list"),
                    false,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        calls.store(0, std::sync::atomic::Ordering::SeqCst);
        let tx = one.begin().await?;
        let res = windmill_api_auth::composed::check_builder_can_name_hub(
            &steps,
            &authed,
            &one,
            "test-workspace",
        )
        .await;
        tx.rollback().await?;
        match (res, refusal) {
            (Ok(()), None) => {}
            (Err(e), Some(text)) if e.to_string().contains(text) => {}
            (res, _) => panic!("{ids:?}: {res:?}"),
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
    Ok(())
}
