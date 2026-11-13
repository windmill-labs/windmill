//! A Hub path named at run time is refused to a member whose role cannot write code, on every
//! route that takes one.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{check_can_name_runnable, RunOrigin},
    runnable_ref::RunnableRef,
};
use windmill_test_utils::*;

const DEV: &str = "SECRET_TOKEN_2";
const OP: &str = "OPERATOR_TOKEN";
const HUB_ID: i32 = 990000792;
const REFUSAL: &str = "Hub scripts and flows require a role that can write code";

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
                // Batch rerun reports every failure as the same "Couldn't re-run" line.
                let refused = (status == 403 && body.contains(REFUSAL))
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
