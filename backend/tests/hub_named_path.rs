//! A Hub path named at run time is refused to a member whose role does not write code, unless
//! the Hub access list allows that script, on every route that takes one.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{check_can_name_runnable, RunOrigin},
    roles::RoleCapabilities,
    runnable_ref::RunnableRef,
};
use windmill_test_utils::*;

const DEV: &str = "SECRET_TOKEN_2";
const OP: &str = "OPERATOR_TOKEN";
const HUB_ID: i32 = 9990792;
const REFUSAL: &str = "Your role may not use this Hub script";
const FLOW_REFUSAL: &str = "Hub flows require a role that can write code";

#[sqlx::test(fixtures("base"))]
async fn a_hub_path_named_without_code_is_refused(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");
    let hub_path = format!("hub/{HUB_ID}/test/echo");

    let hub_dir = &*windmill_common::worker::HUB_CACHE_DIR;
    tokio::fs::create_dir_all(hub_dir).await?;
    tokio::fs::write(
        format!("{hub_dir}/{HUB_ID}"),
        json!({
            "content": "echo ok", "lockfile": null, "language": "bash",
            "schema": {}, "summary": null,
        })
        .to_string(),
    )
    .await?;

    sqlx::raw_sql(
        "INSERT INTO password(email, password_hash, login_type, super_admin, verified, name)
            VALUES ('op@windmill.dev', 'x', 'password', false, true, 'Op');
         INSERT INTO usr(workspace_id, email, username, is_admin, operator, role)
            VALUES ('test-workspace', 'op@windmill.dev', 'op', false, true, 'Operator');
         INSERT INTO token(token_hash, token_prefix, token, email, label, super_admin)
            VALUES (encode(sha256('OPERATOR_TOKEN'::bytea), 'hex'), 'OPERATOR_T', 'OPERATOR_TOKEN',
                    'op@windmill.dev', 'op', false);
         INSERT INTO app (id, workspace_id, path, summary, policy, versions)
            VALUES (999792, 'test-workspace', 'u/test-user/vapp', '',
                    '{\"execution_mode\": \"viewer\", \"triggerables_v2\": {}}'::jsonb, '{}');",
    )
    .execute(&db)
    .await?;
    // What a schedule with a handler pushes for a Hub script: batch rerun reaches its path.
    for (id, user) in [
        ("2aa0c0de-0000-4000-8000-000000000791", "test-user-2"),
        ("2aa0c0de-0000-4000-8000-000000000792", "op"),
    ] {
        sqlx::query(
            "INSERT INTO v2_job(id, workspace_id, kind, runnable_path, created_by, permissioned_as,
                permissioned_as_email, args, raw_flow)
             VALUES ($1::uuid, 'test-workspace', 'singlestepflow', $2, $3, 'u/' || $3, '', '{}',
                jsonb_build_object('modules', jsonb_build_array(jsonb_build_object(
                    'id', 'a', 'value', jsonb_build_object('type', 'script', 'path', $2::text)))))",
        )
        .bind(id)
        .bind(&hub_path)
        .bind(user)
        .execute(&db)
        .await?;
    }

    let client = reqwest::Client::new();
    let send = |token: &'static str, route: &'static str| {
        let (ws, client, hub_path) = (ws.clone(), client.clone(), hub_path.clone());
        let rerun_id = if token == OP { "792" } else { "791" };
        async move {
            let req =
                match route {
                    "run/p" => client
                        .post(format!("{ws}/jobs/run/p/{hub_path}"))
                        .json(&json!({})),
                    "run_wait_result/p POST" => client
                        .post(format!("{ws}/jobs/run_wait_result/p/{hub_path}"))
                        .json(&json!({})),
                    "run_wait_result/p GET" => {
                        client.get(format!("{ws}/jobs/run_wait_result/p/{hub_path}"))
                    }
                    "run_and_stream/p POST" => client
                        .post(format!("{ws}/jobs/run_and_stream/p/{hub_path}"))
                        .json(&json!({})),
                    "run_and_stream/p GET" => {
                        client.get(format!("{ws}/jobs/run_and_stream/p/{hub_path}"))
                    }
                    "dynamic_select" => client
                        .post(format!("{ws}/jobs/run/dynamic_select"))
                        .json(&json!({ "entrypoint_function": "main", "runnable_ref":
                        { "source": "deployed", "path": hub_path, "runnable_kind": "script" } })),
                    "batch_rerun" => client.post(format!("{ws}/jobs/run/batch_rerun_jobs")).json(
                        &json!({
                            "job_ids": [format!("2aa0c0de-0000-4000-8000-000000000{rerun_id}")],
                            "script_options_by_path": { hub_path: { "use_latest_version": true } },
                            "flow_options_by_path": {},
                        }),
                    ),
                    "app component" => client
                        .post(format!("{ws}/apps_u/execute_component/u/test-user/vapp"))
                        .json(&json!({
                            "component": "a", "path": format!("script/{hub_path}"), "args": {}
                        })),
                    // A flow at `hub/123` has no Hub id, so only the path marks it as Hub; the
                    // developer gets past the check to the workspace lookup, which finds none.
                    "app component flow" => client
                        .post(format!("{ws}/apps_u/execute_component/u/test-user/vapp"))
                        .json(&json!({ "component": "a", "path": "flow/hub/123", "args": {} })),
                    _ => unreachable!(),
                };
            let resp = req.bearer_auth(token).send().await.unwrap();
            let status = resp.status();
            (status, resp.text().await.unwrap())
        }
    };

    let routes = [
        "run/p",
        "run_wait_result/p POST",
        "run_wait_result/p GET",
        "run_and_stream/p POST",
        "run_and_stream/p GET",
        "dynamic_select",
        "batch_rerun",
        "app component",
        "app component flow",
    ];
    in_test_worker(
        &db,
        async {
            for route in routes {
                let (status, body) = send(OP, route).await;
                let reason = if route == "app component flow" {
                    FLOW_REFUSAL
                } else {
                    REFUSAL
                };
                // Batch rerun reports every failure as the same "Couldn't re-run" line.
                let refused = (status == 403 && body.contains(reason))
                    || (route == "batch_rerun" && body.starts_with("Error: "));
                assert!(refused, "op on {route}: {status} {body}");

                let (status, body) = send(DEV, route).await;
                let ran = if route == "app component flow" {
                    status == 404 && body.contains("flow not found")
                } else {
                    status.is_success() && !body.contains("Error")
                };
                assert!(ran, "dev on {route}: {status} {body}");
            }
        },
        port,
    )
    .await;

    let op = windmill_api_auth::ApiAuthed {
        email: "op@windmill.dev".into(),
        username: "op".into(),
        is_operator: true,
        ..Default::default()
    };
    let preset = RunnableRef::from_path("hub/2431/slack/schedule-error-handler-slack", false)?;
    let origin = RunOrigin::NamedBy(op.to_authed_ref());
    check_can_name_runnable(&origin, &preset, &db, "test-workspace").await?;
    Ok(())
}

/// The rule reads the capabilities authentication resolved and queries nothing; only a restricted
/// caller built without a role lookup reads the role, and is still refused.
#[sqlx::test(fixtures("base"))]
async fn the_rule_reads_code_from_the_caller(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::raw_sql(
        "INSERT INTO usr(workspace_id, email, username, is_admin, operator, role)
            VALUES ('test-workspace', 'op@windmill.dev', 'op', false, true, 'Operator');",
    )
    .execute(&db)
    .await?;
    let hub = RunnableRef::from_path(&format!("hub/{HUB_ID}/test/echo"), false)?;
    let op = |capabilities| windmill_api_auth::ApiAuthed {
        email: "op@windmill.dev".into(),
        username: "op".into(),
        is_operator: true,
        capabilities,
        ..Default::default()
    };
    let check = |authed: windmill_api_auth::ApiAuthed, db: Pool<Postgres>| {
        let hub = hub.clone();
        async move {
            let origin = RunOrigin::NamedBy(authed.to_authed_ref());
            check_can_name_runnable(&origin, &hub, &db, "test-workspace").await
        }
    };

    let unresolved = check(op(None), db.clone()).await;
    assert!(
        matches!(&unresolved, Err(e) if e.to_string().contains(REFUSAL)),
        "{unresolved:?}"
    );

    // Any query on this pool fails.
    let dead = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")?;
    let refused = check(op(Some(RoleCapabilities::OPERATOR)), dead.clone()).await;
    assert!(
        matches!(&refused, Err(e) if e.to_string().contains(REFUSAL)),
        "{refused:?}"
    );
    let coder = RoleCapabilities { code: true, ..RoleCapabilities::OPERATOR };
    check(op(Some(coder)), dead).await?;
    Ok(())
}

const SLACK_READ: i32 = 990000801;
const SLACK_WRITE: i32 = 990000802;
const SLACK_SCRIPT_UNAPPROVED: i32 = 990000803;
const SLACK_VERSION_UNAPPROVED: i32 = 990000805;
const GITHUB: i32 = 990000804;

fn meta(
    version_id: i32,
    app: &str,
    read_only: bool,
    script_approved: bool,
    version_ever_approved: bool,
) -> serde_json::Value {
    json!({ "version_id": version_id, "ask_id": 1, "app": app, "read_only": read_only,
        "script_approved": script_approved, "version_ever_approved": version_ever_approved })
}

/// A role without code, where Slack is allowed at read, names Slack's approved read-only Hub scripts and
/// nothing else, each refusal saying why; a role with nothing allowed never asks the Hub.
#[sqlx::test(fixtures("base", "hub_grant"))]
async fn a_hub_grant_admits_approved_scripts_of_its_integration(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let (hub, calls) = start_stub_hub(vec![
        meta(SLACK_READ, "Slack", true, true, true),
        meta(SLACK_WRITE, "slack", false, true, true),
        meta(SLACK_SCRIPT_UNAPPROVED, "slack", true, false, true),
        meta(SLACK_VERSION_UNAPPROVED, "slack", true, true, false),
        meta(GITHUB, "github", true, true, true),
    ])
    .await;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!("http://localhost:{}/api/w/hub-grant", server.addr.port());
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{ws}/jobs/run/p/hub/{SLACK_READ}/slack/list"))
        .bearer_auth("SECRET_TOKEN_3")
        .json(&json!({}))
        .send()
        .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    let caps = |hub: serde_json::Value| RoleCapabilities {
        hub: serde_json::from_value(hub).unwrap(),
        ..RoleCapabilities::OPERATOR
    };
    let check = |caps: RoleCapabilities, id: i32| {
        let db = db.clone();
        async move {
            let authed = windmill_api_auth::ApiAuthed {
                email: "test3@windmill.dev".into(),
                username: "test-user-3".into(),
                is_operator: true,
                capabilities: Some(caps),
                ..Default::default()
            };
            let hub = RunnableRef::from_path(&format!("hub/{id}/slack/x"), false).unwrap();
            check_can_name_runnable(
                &RunOrigin::NamedBy(authed.to_authed_ref()),
                &hub,
                &db,
                "hub-grant",
            )
            .await
            .map_err(|e| e.to_string())
        }
    };
    let slack_read = || caps(json!({"slack": "read"}));
    for (id, reason) in [
        (
            SLACK_WRITE,
            "can write to `slack` and your role may only use its read-only scripts",
        ),
        (SLACK_SCRIPT_UNAPPROVED, "is not approved on the Hub"),
        (SLACK_VERSION_UNAPPROVED, "is not approved on the Hub"),
        (GITHUB, "may not use the Hub scripts of `github`"),
        (990000806, "is not on the Hub"),
    ] {
        let err = check(slack_read(), id).await.unwrap_err();
        assert!(err.contains(reason), "{id}: {err}");
    }
    check(caps(json!({"slack": "write"})), SLACK_WRITE)
        .await
        .unwrap();

    let before = calls.load(std::sync::atomic::Ordering::SeqCst);
    let err = check(RoleCapabilities::OPERATOR, 990000809)
        .await
        .unwrap_err();
    assert!(err.contains(REFUSAL), "{err}");
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), before);

    // Another Hub's script of the same id, cached above, is not what this Hub's approval admits.
    let (other_hub, _) = start_stub_hub(vec![]).await;
    let script = windmill_common::scripts::get_full_hub_script_by_id(
        SLACK_READ,
        &windmill_common::utils::HTTP_CLIENT,
        None,
    )
    .await?;
    assert_eq!(script.content, format!("echo {other_hub}"));

    for (url, id, reason) in [
        (
            "http://127.0.0.1:1".to_string(),
            990000810,
            "the Hub is unreachable",
        ),
        (
            format!("{hub}/predates-access-meta"),
            990000811,
            "needs an update",
        ),
    ] {
        windmill_common::HUB_BASE_URL.store(std::sync::Arc::new(url));
        let err = check(slack_read(), id).await.unwrap_err();
        assert!(err.contains(reason), "{id}: {err}");
    }

    Ok(())
}

/// Only an admin writes the workspace's list. An empty one is not the instance's: it allows nothing.
/// Its own workspace: roles are cached per workspace id across the tests.
#[sqlx::test(fixtures("base"))]
async fn a_workspace_replaces_its_hub_access_list(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        "INSERT INTO workspace (id, name, owner) VALUES ('hub-list', 'hub-list', 'test-user');
         INSERT INTO workspace_key (workspace_id, kind, key) VALUES ('hub-list', 'cloud', 'test-key');
         INSERT INTO workspace_settings (workspace_id) VALUES ('hub-list');
         INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES ('hub-list', 'all', '', '{}');
         INSERT INTO usr (workspace_id, email, username, is_admin, operator, role) VALUES
            ('hub-list', 'test@windmill.dev', 'test-user', true, false, 'Admin'),
            ('hub-list', 'test2@windmill.dev', 'test-user-2', false, false, 'User');",
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/hub-list/workspaces",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let write = |token: &'static str, list: serde_json::Value| {
        client
            .post(format!("{ws}/hub_access"))
            .bearer_auth(token)
            .json(&json!({ "hub_access": list }))
            .send()
    };
    let status = write(DEV, json!({"slack": "write"})).await?.status();
    assert_eq!(status, 403);
    let typo = client
        .post(format!("{ws}/hub_access"))
        .bearer_auth("SECRET_TOKEN")
        .json(&json!({ "hubAccess": {} }))
        .send()
        .await?;
    assert_eq!(typo.status(), 422);
    for (list, stored) in [
        (json!({"Slack": "read"}), json!({"slack": "read"})),
        (json!({}), json!({})),
        (json!(null), json!(null)),
    ] {
        let resp = write("SECRET_TOKEN", list).await?;
        assert!(resp.status().is_success(), "{}", resp.text().await?);
        let roles: serde_json::Value = client
            .get(format!("{ws}/roles"))
            .bearer_auth("SECRET_TOKEN")
            .send()
            .await?
            .json()
            .await?;
        assert_eq!(roles["hub_access"], stored);
        let operator = roles["roles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "operator")
            .unwrap();
        let effective = if stored.is_null() { json!({}) } else { stored };
        assert_eq!(operator["capabilities"]["hub"], effective);
    }
    Ok(())
}
