use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::workspaces::invalidate_operator_rights_cache;
use windmill_test_utils::*;

const WS: &str = "test-workspace";

fn operator_client() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str("Bearer OPERATOR_TOKEN_1").unwrap(),
    );
    reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap()
}

async fn set_builder(db: &Pool<Postgres>, flows: bool, apps: bool) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workspace_settings SET operator_settings = $1::text::jsonb WHERE workspace_id = $2",
    )
    .bind(format!(
        r#"{{"builder_flows": {flows}, "builder_apps": {apps}}}"#
    ))
    .bind(WS)
    .execute(db)
    .await?;
    // The rights are read through a process-global 60s cache keyed by workspace id.
    invalidate_operator_rights_cache(WS);
    Ok(())
}

async fn add_script(db: &Pool<Postgres>, hash: i64, path: &str, owner: &str) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema,
             summary, description, lock, extra_perms)
         VALUES ($1, $2, $3, 'x', 'bun', 'script', $4, '{}', '', '', '', '{}')",
    )
    .bind(WS)
    .bind(hash)
    .bind(path)
    .bind(owner)
    .execute(db)
    .await?;
    Ok(())
}

/// A full-code app is deployed multipart, the way the editor and the CLI use it.
fn raw_app(path: &str, mode: &str, runnable_path: &str) -> reqwest::multipart::Form {
    reqwest::multipart::Form::new()
        .part(
            "app",
            reqwest::multipart::Part::text(
                json!({
                    "path": path,
                    "summary": "",
                    "value": {"files": {}, "runnables": {"r": {
                        "name": "r", "type": "runnableByPath", "runType": "script",
                        "path": runnable_path
                    }}},
                    "policy": {"execution_mode": mode, "triggerables_v2": {}}
                })
                .to_string(),
            )
            .mime_str("application/json")
            .unwrap(),
        )
        .part(
            "js",
            reqwest::multipart::Part::text("console.log(1)").file_name("app.js"),
        )
}

fn composition_flow(path: &str) -> serde_json::Value {
    json!({
        "path": path, "summary": "", "description": "", "schema": {},
        "value": {"modules": [{
            "id": "a",
            "value": {"type": "script", "path": "u/operator/some_script", "input_transforms": {}}
        }]}
    })
}

/// The app half of the builder boundary, plus the proof that the two rights are independent: a
/// gate reading "either" would authorize the kind its workspace never granted, which is the whole
/// point of splitting them.
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
async fn test_operator_builder_apps_boundary(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let api = format!("http://localhost:{port}/api/w/{WS}");
    let c = operator_client();

    add_script(&db, 4241, "u/operator/some_script", "operator").await?;
    // `permissions_test` gives the operator fixture no rights on `u/alice/**`.
    add_script(&db, 4243, "u/alice/private", "alice").await?;

    set_builder(&db, false, false).await?;
    let resp = c
        .post(format!("{api}/apps/create_raw"))
        .multipart(raw_app(
            "u/operator/a0",
            "publisher",
            "u/operator/some_script",
        ))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "an operator without the builder right must not create a full-code app"
    );

    set_builder(&db, true, true).await?;

    // `*_raw_source` compiles caller-supplied sources with a bundler job on a worker. It sits
    // beside `create_app_raw`, which builders MAY use, so it is the gate most likely to be opened
    // by mistake later.
    let resp = c
        .post(format!("{api}/apps/create_raw_source"))
        .json(&json!({
            "path": "u/operator/a1", "summary": "",
            "value": {"files": {"index.ts": "console.log(1)"}, "runnables": {}},
            "policy": {"execution_mode": "publisher"}
        }))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not compile app sources on a worker"
    );

    // Low-code apps carry inline scripts, so they stay shut too.
    let resp = c
        .post(format!("{api}/apps/create"))
        .json(&json!({
            "path": "u/operator/a2", "summary": "",
            "value": {"grid": []}, "policy": {"execution_mode": "publisher"}
        }))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not create a low-code app"
    );

    let resp = c
        .post(format!("{api}/apps/create_raw"))
        .multipart(raw_app("u/operator/a3", "publisher", "u/alice/private"))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not deploy an app referencing a runnable it cannot read"
    );

    let resp = c
        .post(format!("{api}/apps/create_raw"))
        .multipart(raw_app("u/operator/a4", "viewer", "u/operator/some_script"))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a builder must not deploy a viewer-mode app: the policy stops bounding what it can invoke"
    );

    let resp = c
        .post(format!("{api}/apps/create_raw"))
        .multipart(raw_app(
            "u/operator/a5",
            "publisher",
            "u/operator/some_script",
        ))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "a builder must be able to deploy a full-code app over readable runnables: {}",
        resp.text().await?
    );
    let sandbox: Option<bool> = sqlx::query_scalar(
        "SELECT (policy->>'sandbox')::boolean FROM app WHERE workspace_id = $1 AND path = $2",
    )
    .bind(WS)
    .bind("u/operator/a5")
    .fetch_one(&db)
    .await?;
    assert_eq!(
        sandbox,
        Some(true),
        "a builder-authored app must be stored sandboxed"
    );

    // `execute_component` looks up `<component>:<path>` with an unrestricted component string, so
    // a key with a second colon still resolves at run time and must not slip past validation.
    let resp = c
        .post(format!("{api}/apps/create_raw"))
        .multipart(
            reqwest::multipart::Form::new()
                .part(
                    "app",
                    reqwest::multipart::Part::text(
                        json!({
                            "path": "u/operator/a6", "summary": "",
                            "value": {"files": {}, "runnables": {}},
                            "policy": {"execution_mode": "publisher", "triggerables_v2": {
                                "x:y:script/u/alice/private": {
                                    "static_inputs": {}, "one_of_inputs": {}
                                }
                            }}
                        })
                        .to_string(),
                    )
                    .mime_str("application/json")
                    .unwrap(),
                )
                .part(
                    "js",
                    reqwest::multipart::Part::text("console.log(1)").file_name("app.js"),
                ),
        )
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "a multi-colon triggerable key must not hide an unreadable runnable from validation"
    );

    // The two rights are independent.
    set_builder(&db, true, false).await?;
    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&composition_flow("u/operator/flows_only"))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "flows-only must still create a flow: {}",
        resp.text().await?
    );
    let resp = c
        .post(format!("{api}/apps/create_raw"))
        .multipart(raw_app(
            "u/operator/flows_only_app",
            "publisher",
            "u/operator/some_script",
        ))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "flows-only must not deploy a full-code app"
    );

    set_builder(&db, false, true).await?;
    let resp = c
        .post(format!("{api}/flows/create"))
        .json(&composition_flow("u/operator/apps_only"))
        .send()
        .await?;
    assert!(
        !resp.status().is_success(),
        "apps-only must not create a flow"
    );
    let resp = c
        .post(format!("{api}/apps/create_raw"))
        .multipart(raw_app(
            "u/operator/apps_only_app",
            "publisher",
            "u/operator/some_script",
        ))
        .send()
        .await?;
    assert!(
        resp.status().is_success(),
        "apps-only must still deploy a full-code app: {}",
        resp.text().await?
    );

    Ok(())
}
