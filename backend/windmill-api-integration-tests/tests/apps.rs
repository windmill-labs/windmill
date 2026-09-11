use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn app_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/apps/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(app_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

fn new_app(path: &str, summary: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "value": {
            "type": "rawapp",
            "inline_script": null
        },
        "policy": {
            "execution_mode": "anonymous",
            "triggerables": {},
            "on_behalf_of": null,
            "on_behalf_of_email": null
        }
    })
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_app_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/apps");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_app("u/test-user/test_app", "Test app"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    // create second app
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_app("u/test-user/another_app", "Another app"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create another: {}", resp.text().await?);

    // --- exists ---
    let resp = authed_get(port, "exists", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get by path ---
    let resp = authed_get(port, "get/p", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_app");
    assert_eq!(body["summary"], "Test app");

    // get not found
    let resp = authed_get(port, "get/p", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- get lite ---
    let resp = authed_get(port, "get/lite", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 2,
        "expected at least 2 apps, got {}",
        list.len()
    );
    assert!(list.iter().any(|a| a["path"] == "u/test-user/test_app"));

    // --- list_search ---
    let resp = authed(client().get(format!("{base}/list_search")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());

    // --- history ---
    let resp = authed_get(port, "history/p", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!history.is_empty());

    // --- get by version ---
    let version = &history[0]["version"];
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/apps/get/v/{version}"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    // --- custom_path_exists ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/w/test-workspace/apps/custom_path_exists/nonexistent"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- secret_of ---
    let resp = authed_get(port, "secret_of", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let secret_id = resp.text().await?;
    assert!(!secret_id.is_empty());

    // --- get_latest_version ---
    let resp = authed_get(port, "get_latest_version", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);

    // --- public_app (unauthed, by secret) ---
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/apps_u/public_app/{secret_id}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "public_app: {}", resp.text().await?);

    // --- secret_of_latest_version ---
    let resp = authed_get(port, "secret_of_latest_version", "u/test-user/test_app").await;
    assert_eq!(resp.status(), 200);
    let secret = resp.text().await?;
    assert!(!secret.is_empty());

    // --- list_paths_from_workspace_runnable ---
    let resp = authed(client().get(format!(
        "{base}/list_paths_from_workspace_runnable/script/u/test-user/test_app"
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "list_paths_from_workspace_runnable: {}",
        resp.text().await?
    );

    // --- history_update ---
    let app_body = authed_get(port, "get/p", "u/test-user/test_app").await;
    let app = app_body.json::<serde_json::Value>().await?;
    let app_id = &app["id"];
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/apps/history_update/a/{app_id}/v/{version}"
    )))
    .json(&json!({"deployment_msg": "deployed v1"}))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "history_update: {}", resp.text().await?);

    // --- update ---
    let resp = authed(client().post(app_url(port, "update", "u/test-user/test_app")))
        .json(&json!({
            "summary": "Updated app",
            "policy": {
                "execution_mode": "anonymous",
                "triggerables": {},
                "on_behalf_of": null,
                "on_behalf_of_email": null
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);

    // verify update
    let resp = authed_get(port, "get/p", "u/test-user/test_app").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated app");

    // --- delete ---
    let resp = authed(client().delete(app_url(port, "delete", "u/test-user/another_app")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "exists", "u/test-user/another_app").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // ===== Hub endpoints (require external network, expect 500 or 200) =====

    // --- hub/list ---
    let resp = authed(client().get(format!("http://localhost:{port}/api/apps/hub/list")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/list: unexpected status {}",
        resp.status()
    );

    // --- hub/get ---
    let resp = authed(client().get(format!("http://localhost:{port}/api/apps/hub/get/1")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get: unexpected status {}",
        resp.status()
    );

    // --- hub/get_raw ---
    let resp = authed(client().get(format!("http://localhost:{port}/api/apps/hub/get_raw/1")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get_raw: unexpected status {}",
        resp.status()
    );

    Ok(())
}

#[cfg(feature = "enterprise")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_public_app_by_custom_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/apps");

    // create app with anonymous execution mode
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_app("u/test-user/custom_path_app", "Custom path app"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    // set custom_path on the app
    let resp = authed(client().post(app_url(port, "update", "u/test-user/custom_path_app")))
        .json(&serde_json::json!({
            "custom_path": "my-custom-app",
            "policy": {
                "execution_mode": "anonymous",
                "triggerables": {},
                "on_behalf_of": null,
                "on_behalf_of_email": null
            }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "update custom_path: {}",
        resp.text().await?
    );

    // fetch via public_app_by_custom_path (no workspace prefix: CLOUD_HOSTED=false, APP_WORKSPACED_ROUTE=false)
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/apps_u/public_app_by_custom_path/my-custom-app"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "public_app_by_custom_path: {}",
        resp.text().await?
    );

    // verify response contains expected fields
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/apps_u/public_app_by_custom_path/my-custom-app"
        ))
        .send()
        .await
        .unwrap();
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/custom_path_app");
    assert_eq!(body["summary"], "Custom path app");
    assert_eq!(body["workspace_id"], "test-workspace");
    assert_eq!(body["custom_path"], "my-custom-app");

    // nonexistent custom path returns 404
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/apps_u/public_app_by_custom_path/nonexistent"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    Ok(())
}

/// A raw app's kind lives on its version row, so a value deployed through the
/// low-code endpoint used to convert the app in place and strand its bundle.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_raw_app_kind_is_not_flipped_by_update(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/apps");
    let raw_path = "u/test-user/raw_app";
    let low_code_path = "u/test-user/low_code_app";

    let raw_app_form = |path: &str| {
        reqwest::multipart::Form::new()
            .text(
                "app",
                json!({
                    "path": path,
                    "summary": "Raw app",
                    "value": { "files": { "index.ts": "export {}" }, "runnables": {} },
                    "policy": { "execution_mode": "publisher", "triggerables_v2": {} }
                })
                .to_string(),
            )
            .part(
                "js",
                reqwest::multipart::Part::bytes(b"console.log(1)".to_vec()),
            )
    };

    let resp = authed(client().post(format!("{base}/create_raw")))
        .multipart(raw_app_form(raw_path))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create_raw: {}", resp.text().await?);

    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_app(low_code_path, "Low code app"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    let versions_of = |path: &'static str| async move {
        let resp = authed_get(port, "get/p", path).await;
        let body = resp.json::<serde_json::Value>().await.unwrap();
        body["versions"].as_array().unwrap().len()
    };
    let raw_versions = versions_of(raw_path).await;

    // Neither endpoint may deploy a value onto an app of the other kind.
    let resp = authed(client().post(format!("{base}/update/{raw_path}")))
        .json(&json!({ "value": { "grid": [] } }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    assert!(
        resp.text().await?.contains("is a raw app"),
        "expected the low-code update of a raw app to be refused"
    );
    // The refusal has to land before the version insert, not roll one back.
    assert_eq!(versions_of(raw_path).await, raw_versions);

    let resp = authed(client().post(format!("{base}/update_raw/{low_code_path}")))
        .multipart(raw_app_form(low_code_path))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    assert!(
        resp.text().await?.contains("is a low-code app"),
        "expected the raw update of a low-code app to be refused"
    );

    // Nor may the source endpoints compile a value that isn't a raw app's: both
    // refuse up front, without queueing a bundle job no worker would pick up here.
    let resp = authed(client().post(format!("{base}/create_raw_source")))
        .json(&json!({
            "path": "u/test-user/from_source",
            "summary": "",
            "value": { "grid": [] },
            "policy": { "execution_mode": "publisher" }
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    assert!(
        resp.text().await?.contains("no `files` to bundle"),
        "expected a low-code value to be refused before bundling"
    );

    // The source endpoint refuses the same mismatch up front — it must not queue
    // a bundle job (which no worker would pick up here) to find that out.
    let resp = authed(client().post(format!("{base}/update_raw_source/{low_code_path}")))
        .json(&json!({ "value": { "files": { "/index.tsx": "export {}" } } }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    assert!(
        resp.text().await?.contains("is a low-code app"),
        "expected the source deploy of a low-code app to be refused"
    );

    // Metadata-only updates and same-kind deploys still go through.
    let resp = authed(client().post(format!("{base}/update/{raw_path}")))
        .json(&json!({ "summary": "Renamed" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update summary: {}", resp.text().await?);

    let resp = authed(client().post(format!("{base}/update_raw/{raw_path}")))
        .multipart(raw_app_form(raw_path))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update_raw: {}", resp.text().await?);

    let resp = authed_get(port, "get/p", raw_path).await;
    assert_eq!(resp.json::<serde_json::Value>().await?["raw_app"], true);

    // A caller that means to convert says so, which is how an app converted by
    // accident gets restored to what it was.
    let resp = authed(client().post(format!("{base}/update/{raw_path}")))
        .json(&json!({ "value": { "grid": [] }, "allow_kind_change": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "update: {}", resp.text().await?);

    let resp = authed_get(port, "get/p", raw_path).await;
    assert_eq!(resp.json::<serde_json::Value>().await?["raw_app"], false);

    Ok(())
}

/// Who may create at a path is the app table's answer, not a rule restated in the
/// handler: a non-admin member of `g/<group>` writes there, and nowhere a
/// non-member does. Both are decided before the sources are compiled, so an empty
/// `files` is enough to say which side of that check the request reached.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_raw_source_write_check_follows_app_rls(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::query(
        "INSERT INTO usr_to_group (workspace_id, usr, group_) VALUES
         ('test-workspace', 'test-user-2', 'all')",
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/apps");
    let create_as_member = |path: &'static str| {
        client()
            .post(format!("{base}/create_raw_source"))
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .json(&json!({
                "path": path,
                "summary": "",
                "value": { "files": {} },
                "policy": { "execution_mode": "publisher" }
            }))
            .send()
    };

    let resp = create_as_member("g/all/from_source").await.unwrap();
    assert_eq!(
        resp.status(),
        400,
        "a group member must reach the compile: {}",
        resp.text().await?
    );

    let resp = create_as_member("u/test-user/from_source").await.unwrap();
    assert_eq!(
        resp.status(),
        403,
        "expected another user's path to be refused"
    );

    Ok(())
}
