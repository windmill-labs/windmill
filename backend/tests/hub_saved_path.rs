//! A Hub path saved into a trigger or a schedule is checked when it is chosen, against the member
//! saving it: refused without `code`, and only on the paths the save changes.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const DEV: &str = "SECRET_TOKEN_2";
const OP: &str = "OPERATOR_TOKEN";
const HUB: &str = "hub/990000802/test/echo";
const OTHER_HUB: &str = "hub/990000803/test/echo";
const PRESET: &str = "hub/2431/slack/schedule-error-handler-slack";
const REFUSAL: &str = "Hub scripts and flows require a role that can write code";

#[cfg(feature = "http_trigger")]
fn http(path: &str, script_path: &str, error_handler: Option<&str>) -> Value {
    json!({
        "path": path, "script_path": script_path, "is_flow": false, "mode": "disabled",
        "route_path": path.replace('/', "_"), "request_type": "async",
        "authentication_method": "none", "http_method": "post", "is_static_website": false,
        "workspaced_route": true, "wrap_body": false, "raw_string": false,
        "error_handler_path": error_handler,
    })
}

#[cfg(feature = "websocket")]
fn websocket(path: &str, url: &str, initial: Option<&str>) -> Value {
    json!({
        "path": path, "script_path": "f/shared/s", "is_flow": false, "mode": "disabled",
        "url": url, "filters": [], "can_return_message": false, "can_return_error_result": false,
        "initial_messages": initial.map(|p|
            json!([{"runnable_result": {"path": p, "args": {}, "is_flow": false}}])),
    })
}

fn schedule(path: &str, script_path: &str, on_failure: Option<&str>) -> Value {
    json!({
        "path": path, "schedule": "0 0 0 * * *", "timezone": "UTC", "script_path": script_path,
        "is_flow": false, "enabled": false, "on_failure": on_failure,
    })
}

#[sqlx::test(fixtures("base"))]
async fn a_saved_hub_path_follows_the_code_capability(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        r#"INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
             VALUES ('op@windmill.dev', 'x', 'password', false, true, 'Op');
           INSERT INTO usr(workspace_id, email, username, is_admin, operator, role)
             VALUES ('test-workspace', 'op@windmill.dev', 'op', false, true, 'Operator');
           INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
             VALUES (encode(sha256('OPERATOR_TOKEN'::bytea), 'hex'), 'OPERATOR_T',
                     'OPERATOR_TOKEN', 'op@windmill.dev', 'op', false);
           INSERT INTO folder(workspace_id, name, display_name, owners, extra_perms)
             VALUES ('test-workspace', 'shared', 'shared', '{}',
                     '{"u/op": true, "u/test-user-2": true}');"#,
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let send = async |token: &str, route: &str, body: Value| {
        let resp = client
            .post(format!("{ws}/{route}"))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .unwrap();
        (resp.status().as_u16(), resp.text().await.unwrap())
    };

    // Rows a developer saved, for an operator to edit.
    #[cfg(feature = "websocket")]
    {
        let ws = websocket(
            "f/shared/dev_ws",
            &format!("$script:{HUB}"),
            Some(OTHER_HUB),
        );
        let (s, b) = send(DEV, "websocket_triggers/create", ws).await;
        assert_eq!(s, 201, "{b}");
    }
    #[cfg(feature = "http_trigger")]
    {
        let (s, b) = send(
            DEV,
            "http_triggers/create",
            http("f/shared/dev_route", HUB, None),
        )
        .await;
        assert_eq!(s, 201, "{b}");
    }
    for path in ["f/shared/dev_sched", "f/shared/plain_sched"] {
        let on_failure = (path == "f/shared/dev_sched").then(|| format!("script/{HUB}"));
        let (s, b) = send(
            DEV,
            "schedules/create",
            schedule(path, "f/shared/s", on_failure.as_deref()),
        )
        .await;
        assert!(s < 300, "{b}");
    }
    let edit = |on_failure: &str| {
        json!({"schedule": "0 0 1 * * *", "timezone": "UTC",
               "on_failure": format!("script/{on_failure}")})
    };

    // (case, route, body for a path): refused to the operator.
    #[allow(unused_mut)]
    let mut refused: Vec<(&str, String, Box<dyn Fn(&str) -> Value>)> = vec![
        (
            "schedule create, Hub target",
            "schedules/create".into(),
            Box::new(|p| schedule(p, HUB, None)),
        ),
        (
            "schedule create, Hub on_failure",
            "schedules/create".into(),
            Box::new(|p| schedule(p, "f/shared/s", Some(&format!("script/{HUB}")))),
        ),
        (
            "schedule edit, adding a Hub on_failure",
            "schedules/update/f/shared/plain_sched".into(),
            Box::new(|_| edit(HUB)),
        ),
    ];
    #[cfg(feature = "http_trigger")]
    refused.extend([
        (
            "HTTP create, Hub target",
            "http_triggers/create".into(),
            Box::new(|p: &str| http(p, HUB, None)) as Box<dyn Fn(&str) -> Value>,
        ),
        (
            "HTTP create, Hub error handler",
            "http_triggers/create".into(),
            Box::new(|p: &str| http(p, "f/shared/s", Some(HUB))),
        ),
        (
            "HTTP bulk create, Hub target",
            "http_triggers/create_many".into(),
            Box::new(|p: &str| {
                json!([
                    http(&format!("{p}_a"), "f/shared/s", None),
                    http(p, HUB, None)
                ])
            }),
        ),
    ]);
    #[cfg(feature = "websocket")]
    refused.extend([
        (
            "WebSocket create, Hub URL",
            "websocket_triggers/create".into(),
            Box::new(|p: &str| websocket(p, &format!("$script:{HUB}"), None))
                as Box<dyn Fn(&str) -> Value>,
        ),
        (
            "WebSocket create, Hub initial message",
            "websocket_triggers/create".into(),
            Box::new(|p: &str| websocket(p, "$flow:f/shared/url", Some(HUB))),
        ),
        (
            "WebSocket capture config, Hub URL",
            "capture/set_config".into(),
            Box::new(|p: &str| {
                json!({"trigger_kind": "websocket", "path": p, "is_flow": false,
                       "trigger_config": {"url": format!("$script:{HUB}")}})
            }),
        ),
    ]);

    for (i, (case, route, body)) in refused.iter().enumerate() {
        let (status, text) = send(OP, route, body(&format!("f/shared/op_{i}"))).await;
        assert!(
            status == 403 && text.contains(&format!("{HUB}: {REFUSAL}")),
            "operator, {case}: {status} {text}"
        );
    }
    let (case, route, body) = &refused[0];
    let (status, text) = send(DEV, route, body("f/shared/dev_0")).await;
    assert!(status < 300, "developer, {case}: {status} {text}");
    #[cfg(feature = "http_trigger")]
    {
        let written: i64 =
            sqlx::query_scalar("SELECT count(*) FROM http_trigger WHERE path LIKE 'f/shared/op_%'")
                .fetch_one(&db)
                .await?;
        assert_eq!(written, 0, "a refused save wrote a row");
    }

    // Allowed to the operator: what the row already holds, and a preset handler.
    #[allow(unused_mut)]
    let mut allowed: Vec<(&str, &str, Value)> = vec![
        (
            "schedule edit, Hub on_failure unchanged",
            "schedules/update/f/shared/dev_sched",
            edit(HUB),
        ),
        (
            "schedule create, preset on_failure",
            "schedules/create",
            schedule(
                "f/shared/op_preset",
                "f/shared/s",
                Some(&format!("script/{PRESET}")),
            ),
        ),
    ];
    #[cfg(feature = "http_trigger")]
    allowed.push((
        "HTTP update, Hub target unchanged, renamed",
        "http_triggers/update/f/shared/dev_route",
        http("f/shared/dev_route_renamed", HUB, None),
    ));
    #[cfg(feature = "websocket")]
    allowed.push((
        "WebSocket update, Hub URL and initial message unchanged, renamed",
        "websocket_triggers/update/f/shared/dev_ws",
        websocket(
            "f/shared/dev_ws_renamed",
            &format!("$script:{HUB}"),
            Some(OTHER_HUB),
        ),
    ));
    for (case, route, body) in allowed {
        let (status, text) = send(OP, route, body).await;
        assert!(status < 300, "operator, {case}: {status} {text}");
    }

    #[cfg(feature = "http_trigger")]
    {
        let case = "HTTP update to a different Hub target";
        let (status, text) = send(
            OP,
            "http_triggers/update/f/shared/dev_route_renamed",
            http("f/shared/dev_route_renamed", OTHER_HUB, None),
        )
        .await;
        assert!(
            status == 403 && text.contains(&format!("{OTHER_HUB}: {REFUSAL}")),
            "operator, {case}: {status} {text}"
        );
    }
    Ok(())
}
