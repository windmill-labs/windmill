use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn script_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/scripts/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(script_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

fn new_script(path: &str, summary: &str, content: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": summary,
        "description": "",
        "content": content,
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

/// A supplied lock queues no dependency job, so if the create does not record its hash nothing
/// ever will, and every importer of this script relocks on each of its deploys forever after.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_script_persists_supplied_lock_hash(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let path = "u/test-user/supplied_lock";
    let lock = r#"{"version":"4","remote":{}}"#;
    let mut script = new_script(
        path,
        "Supplied lock",
        "export async function main() { return 42; }",
    );
    script["lock"] = json!(lock);

    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/create"
    )))
    .json(&script)
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    let stored_hash = sqlx::query_scalar!(
        "SELECT lockfile_hash FROM lock_hash WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        path,
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(stored_hash, windmill_common::scripts::hash_script(lock));

    // A script deployed before the create recorded hashes has no row, and pushing it unchanged
    // creates no version to hang one off. Without the write on that path it would keep its
    // importers relocking until someone edited it.
    sqlx::query!(
        "DELETE FROM lock_hash WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        path,
    )
    .execute(&db)
    .await?;

    // The no-op comparison covers every field, so the push has to carry what the first deploy
    // filled in by itself; `auto_parent` both resolves the parent and keeps the hash distinct.
    script["auto_parent"] = json!(true);
    script["ws_error_handler_muted"] = json!(false);
    script["assets"] = json!([]);
    let resp = authed(client().post(format!(
        "http://localhost:{port}/api/w/test-workspace/scripts/create?skip_if_noop=true"
    )))
    .json(&script)
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "no-op push: {}", resp.text().await?);

    let versions: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM script WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        path,
    )
    .fetch_one(&db)
    .await?
    .unwrap_or_default();
    assert_eq!(versions, 1, "no-op push must not create a version");

    let repaired_hash = sqlx::query_scalar!(
        "SELECT lockfile_hash FROM lock_hash WHERE workspace_id = $1 AND path = $2",
        "test-workspace",
        path,
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(repaired_hash, windmill_common::scripts::hash_script(lock));

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_script_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/test_script",
            "Test script",
            "export async function main() { return 42; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create: {}", resp.text().await?);

    // create second script
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/another_script",
            "Another script",
            "export async function main() { return 'hello'; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create another: {}", resp.text().await?);

    // --- exists ---
    let resp = authed_get(port, "exists/p", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists/p", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get by path ---
    let resp = authed_get(port, "get/p", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_script");
    assert_eq!(body["summary"], "Test script");
    assert_eq!(body["language"], "deno");
    assert!(body["hash"].is_string(), "expected hash to be a hex string");
    let hash = body["hash"].as_str().unwrap().to_string();

    // get not found
    let resp = authed_get(port, "get/p", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- get by hash ---
    let resp = authed_get(port, "get/h", &hash).await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/test_script");

    // --- raw by path (requires language extension) ---
    let resp = authed_get(port, "raw/p", "u/test-user/test_script.ts").await;
    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(
        body.contains("return 42"),
        "expected script content, got: {body}"
    );

    // --- raw by hash (requires .ts suffix) ---
    let resp = authed_get(port, "raw/h", &format!("{hash}.ts")).await;
    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(body.contains("return 42"));

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 2,
        "expected at least 2 scripts, got {}",
        list.len()
    );
    assert!(list.iter().any(|s| s["path"] == "u/test-user/test_script"));

    // list with path_start filter
    let resp = authed(client().get(format!("{base}/list?path_start=u/test-user/another")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["path"], "u/test-user/another_script");

    // --- list_search ---
    let resp = authed(client().get(format!("{base}/list_search")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());

    // --- list_paths ---
    let resp = authed(client().get(format!("{base}/list_paths")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let paths = resp.json::<Vec<String>>().await?;
    assert!(paths.contains(&"u/test-user/test_script".to_string()));

    // --- history ---
    let resp = authed_get(port, "history/p", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!history.is_empty());

    // --- get_latest_version ---
    let resp = authed_get(port, "get_latest_version", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);

    // --- deployment_status ---
    let resp = authed_get(port, "deployment_status/h", &hash).await;
    assert_eq!(resp.status(), 200);

    // --- raw_unpinned by path ---
    let resp = authed_get(port, "raw_unpinned/p", "u/test-user/test_script.ts").await;
    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(body.contains("return 42"));

    // --- list_tokens ---
    let resp = authed_get(port, "list_tokens", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- list_paths_from_workspace_runnable ---
    let resp = authed_get(
        port,
        "list_paths_from_workspace_runnable",
        "u/test-user/test_script",
    )
    .await;
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<String>>().await?;

    // --- update script (create new version) ---
    let mut updated = new_script(
        "u/test-user/test_script",
        "Updated test script",
        "export async function main() { return 99; }",
    );
    updated["parent_hash"] = json!(&hash);
    let resp = authed(client().post(format!("{base}/create")))
        .json(&updated)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "update: {}", resp.text().await?);

    // verify new version
    let resp = authed_get(port, "get/p", "u/test-user/test_script").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "Updated test script");
    let new_hash = body["hash"].as_str().unwrap();
    assert_ne!(new_hash, hash, "hash should change on update");

    // history should have 2 entries now
    let resp = authed_get(port, "history/p", "u/test-user/test_script").await;
    let history = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        history.len() >= 2,
        "expected at least 2 history entries, got {}",
        history.len()
    );

    // --- history_update ---
    let resp = authed(client().post(format!(
        "{base}/history_update/h/{new_hash}/p/u/test-user/test_script"
    )))
    .json(&json!({"deployment_msg": "deployed v2"}))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "history_update: {}", resp.text().await?);

    // --- toggle_workspace_error_handler (EE-gated, expect 400 in OSS) ---
    let resp = authed(client().post(script_url(
        port,
        "toggle_workspace_error_handler/p",
        "u/test-user/test_script",
    )))
    .json(&json!({}))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 400,
        "toggle_workspace_error_handler: unexpected status {}",
        resp.status()
    );

    // --- get_triggers_count ---
    let resp = authed_get(port, "get_triggers_count", "u/test-user/test_script").await;
    assert_eq!(resp.status(), 200);

    // --- tokened_raw (global unauthed, token in URL) ---
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/scripts_u/tokened_raw/test-workspace/SECRET_TOKEN/u/test-user/test_script.ts"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "tokened_raw: {}", resp.text().await?);

    // --- archive by path ---
    let resp = authed(client().post(script_url(port, "archive/p", "u/test-user/another_script")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // archived script should still be gettable
    let resp = authed_get(port, "get/p", "u/test-user/another_script").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["archived"], true);
    let another_hash = body["hash"].as_str().unwrap().to_string();

    // --- archive by hash ---
    let resp = authed(client().post(script_url(port, "archive/h", &another_hash)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- delete by hash ---
    let resp = authed(client().post(script_url(port, "delete/h", &another_hash)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- delete_bulk ---
    let resp = authed(client().delete(format!("{base}/delete_bulk")))
        .json(&json!({"paths": ["u/test-user/test_script"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "delete_bulk: {}", resp.text().await?);

    let resp = authed_get(port, "exists/p", "u/test-user/test_script").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // --- empty_ts (global unauthed) ---
    let resp = client()
        .get(format!(
            "http://localhost:{port}/api/scripts_u/empty_ts/u/test-user/any_script"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(body.is_empty(), "expected empty string, got: {body}");

    // ===== Hub endpoints (require external network, expect 500 or 200) =====

    // --- hub/top ---
    let resp = authed(client().get(format!("http://localhost:{port}/api/scripts/hub/top")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/top: unexpected status {}",
        resp.status()
    );

    // --- hub/get (raw script by path, needs hub/ prefix in path) ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/scripts/hub/get/hub/1/hello"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get: unexpected status {}",
        resp.status()
    );

    // --- hub/get_full (full script by path, needs hub/ prefix in path) ---
    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/scripts/hub/get_full/hub/1/hello"
    )))
    .send()
    .await
    .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "hub/get_full: unexpected status {}",
        resp.status()
    );

    // --- integrations hub/list ---
    let resp = authed(client().get(format!("http://localhost:{port}/api/integrations/hub/list")))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status() == 200 || resp.status() == 500,
        "integrations hub/list: unexpected status {}",
        resp.status()
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_auto_parent_resolves_parent_hash(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");

    // Create v1
    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            "u/test-user/auto_parent_test",
            "v1",
            "export async function main() { return 1; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create v1: {}", resp.text().await?);

    // Get the hash of v1
    let resp = authed_get(port, "get/p", "u/test-user/auto_parent_test").await;
    let body = resp.json::<serde_json::Value>().await?;
    let v1_hash = body["hash"].as_str().unwrap().to_string();

    // Create v2 using auto_parent (no parent_hash provided)
    let mut v2 = new_script(
        "u/test-user/auto_parent_test",
        "v2",
        "export async function main() { return 2; }",
    );
    v2["auto_parent"] = json!(true);
    let resp = authed(client().post(format!("{base}/create")))
        .json(&v2)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "create v2 with auto_parent: {}",
        resp.text().await?
    );

    // Get v2 and verify its parent_hash points to v1
    let resp = authed_get(port, "get/p", "u/test-user/auto_parent_test").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "v2");
    let v2_hash = body["hash"].as_str().unwrap().to_string();
    assert_ne!(v2_hash, v1_hash);

    // v2's parent_hashes should contain v1
    let parent_hashes = body["parent_hashes"].as_array().unwrap();
    assert!(
        parent_hashes
            .iter()
            .any(|h| h.as_str() == Some(v1_hash.as_str())),
        "v2 parent_hashes should contain v1 hash {v1_hash}, got: {parent_hashes:?}"
    );

    // Create v3 with auto_parent to confirm it chains correctly
    let mut v3 = new_script(
        "u/test-user/auto_parent_test",
        "v3",
        "export async function main() { return 3; }",
    );
    v3["auto_parent"] = json!(true);
    let resp = authed(client().post(format!("{base}/create")))
        .json(&v3)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "create v3 with auto_parent: {}",
        resp.text().await?
    );

    let resp = authed_get(port, "get/p", "u/test-user/auto_parent_test").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["summary"], "v3");

    // v3's parent_hashes should contain v2 (and transitively v1)
    let parent_hashes = body["parent_hashes"].as_array().unwrap();
    assert!(
        parent_hashes
            .iter()
            .any(|h| h.as_str() == Some(v2_hash.as_str())),
        "v3 parent_hashes should contain v2 hash {v2_hash}, got: {parent_hashes:?}"
    );

    // Redeploy v1's exact body. The version hash covers the parent, so this is a
    // distinct version of the lineage rather than a repeat of the archived v1 —
    // which it is not if the hash is taken before auto_parent resolves the parent.
    let mut revert = new_script(
        "u/test-user/auto_parent_test",
        "v1",
        "export async function main() { return 1; }",
    );
    revert["auto_parent"] = json!(true);
    let resp = authed(client().post(format!("{base}/create")))
        .json(&revert)
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "reverting to v1's content with auto_parent: {}",
        resp.text().await?
    );

    Ok(())
}

/// The update route carries the version being superseded in its URL, so a caller that
/// cannot read a `parent_hash` still chains onto the history instead of forking it.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_script_chains_moves_and_refuses_a_free_path(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");
    let path = "u/test-user/update_test";

    // Nothing deployed there yet: an update has no version to supersede.
    let resp = authed(client().post(format!("{base}/update/{path}")))
        .json(&new_script(
            path,
            "v1",
            "export async function main() { return 1; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404, "update of a free path must be refused");

    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            path,
            "v1",
            "export async function main() { return 1; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create v1: {}", resp.text().await?);
    let v1_hash = authed_get(port, "get/p", path)
        .await
        .json::<serde_json::Value>()
        .await?["hash"]
        .as_str()
        .unwrap()
        .to_string();

    // The body repeats the path, so the script stays where it is, chained onto v1.
    let resp = authed(client().post(format!("{base}/update/{path}")))
        .json(&new_script(
            path,
            "v2",
            "export async function main() { return 2; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        201,
        "update in place: {}",
        resp.text().await?
    );

    let body = authed_get(port, "get/p", path)
        .await
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(body["summary"], "v2");
    let v2_hash = body["hash"].as_str().unwrap().to_string();
    let parent_hashes = body["parent_hashes"].as_array().unwrap();
    assert!(
        parent_hashes.iter().any(|h| h.as_str() == Some(&v1_hash)),
        "v2 must descend from v1 {v1_hash}, got: {parent_hashes:?}"
    );

    // A body path that differs moves the script, taking the history with it.
    let moved_path = "u/test-user/update_test_moved";
    let resp = authed(client().post(format!("{base}/update/{path}")))
        .json(&new_script(
            moved_path,
            "v3",
            "export async function main() { return 3; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "move: {}", resp.text().await?);

    let body = authed_get(port, "get/p", moved_path)
        .await
        .json::<serde_json::Value>()
        .await?;
    assert_eq!(body["summary"], "v3");
    let parent_hashes = body["parent_hashes"].as_array().unwrap();
    assert!(
        parent_hashes.iter().any(|h| h.as_str() == Some(&v2_hash)),
        "the moved script must descend from v2 {v2_hash}, got: {parent_hashes:?}"
    );
    assert_eq!(
        authed_get(port, "get/p", path)
            .await
            .json::<serde_json::Value>()
            .await?["archived"],
        json!(true),
        "the vacated path must be left archived"
    );

    Ok(())
}

/// An archived path holds no version to supersede, so an update must not revive it. The
/// resolution that decides this runs in the deploying transaction rather than ahead of
/// it, which is what also covers an archive landing mid-deploy — a race this sequential
/// test cannot stage, so it pins the reachable half.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_script_does_not_revive_an_archived_path(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");
    let path = "u/test-user/archived_update_test";

    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            path,
            "v1",
            "export async function main() { return 1; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create v1: {}", resp.text().await?);

    let resp = authed(client().post(format!("{base}/archive/p/{path}")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "archive: {}", resp.text().await?);

    let resp = authed(client().post(format!("{base}/update/{path}")))
        .json(&new_script(
            path,
            "v2",
            "export async function main() { return 2; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        404,
        "updating an archived path must not revive it"
    );

    assert_eq!(
        authed_get(port, "get/p", path)
            .await
            .json::<serde_json::Value>()
            .await?["archived"],
        json!(true),
        "the path must still be archived"
    );

    Ok(())
}

/// Stages the interleaving the sequential test above cannot: the update resolves its
/// parent while an archive is mid-flight. Holding the head row locked from another
/// connection parks the update on that row, so the archive lands first by construction
/// — the ordering that, unlocked, hands the deploy an archived hash to chain onto.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_script_loses_a_race_with_archive(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");
    let path = "u/test-user/raced_update_test";

    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            path,
            "v1",
            "export async function main() { return 1; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create v1: {}", resp.text().await?);

    // Take the head row before the update can, so it blocks where it resolves.
    let mut blocker = db.begin().await?;
    let head: i64 = sqlx::query_scalar(
        "SELECT hash FROM script WHERE path = $1 AND archived = false AND workspace_id = $2 \
         FOR UPDATE",
    )
    .bind(path)
    .bind("test-workspace")
    .fetch_one(&mut *blocker)
    .await?;

    let update = tokio::spawn({
        let base = base.clone();
        let body = new_script(path, "v2", "export async function main() { return 2; }");
        async move {
            authed(client().post(format!("{base}/update/{path}")))
                .json(&body)
                .send()
                .await
                .unwrap()
        }
    });

    // Order the archive after whatever the update has already read: wait until it is
    // parked on the row lock. Without this the update can lose to a local UPDATE and
    // never reach its resolution, which is the sequential case the test above covers.
    let mut parked = false;
    for _ in 0..400 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity WHERE wait_event_type = 'Lock' \
             AND datname = current_database() AND pid <> pg_backend_pid()",
        )
        .fetch_one(&db)
        .await?;
        if waiting > 0 {
            parked = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert!(parked, "the update never parked on the head row lock");

    // Archive under the lock the update is waiting on, then release it.
    sqlx::query("UPDATE script SET archived = true WHERE hash = $1 AND workspace_id = $2")
        .bind(head)
        .bind("test-workspace")
        .execute(&mut *blocker)
        .await?;
    blocker.commit().await?;

    let resp = tokio::time::timeout(std::time::Duration::from_secs(20), update).await??;
    assert_eq!(
        resp.status(),
        404,
        "an update that lost the race must not revive the archived script"
    );
    assert_eq!(
        authed_get(port, "get/p", path)
            .await
            .json::<serde_json::Value>()
            .await?["archived"],
        json!(true),
        "the path must be left archived"
    );

    Ok(())
}

/// The same interleaving, except the winner leaves a live head behind. The loser must
/// say so rather than "not found" of a path the caller can see holds a script.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_script_reports_losing_to_a_concurrent_deploy(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");
    let path = "u/test-user/superseded_update_test";

    let resp = authed(client().post(format!("{base}/create")))
        .json(&new_script(
            path,
            "v1",
            "export async function main() { return 1; }",
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "create v1: {}", resp.text().await?);

    let mut winner = db.begin().await?;
    let head: i64 = sqlx::query_scalar(
        "SELECT hash FROM script WHERE path = $1 AND archived = false AND workspace_id = $2 \
         FOR UPDATE",
    )
    .bind(path)
    .bind("test-workspace")
    .fetch_one(&mut *winner)
    .await?;

    let update = tokio::spawn({
        let base = base.clone();
        let body = new_script(path, "v2", "export async function main() { return 2; }");
        async move {
            authed(client().post(format!("{base}/update/{path}")))
                .json(&body)
                .send()
                .await
                .unwrap()
        }
    });

    let mut parked = false;
    for _ in 0..400 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity WHERE wait_event_type = 'Lock' \
             AND datname = current_database() AND pid <> pg_backend_pid()",
        )
        .fetch_one(&db)
        .await?;
        if waiting > 0 {
            parked = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert!(parked, "the update never parked on the head row lock");

    // What a deploy leaves behind: the old head archived, a new one live at the path.
    // Copied through a temp table so this does not have to restate every column.
    sqlx::query(
        "CREATE TEMP TABLE superseding ON COMMIT DROP AS SELECT * FROM script WHERE hash = $1",
    )
    .bind(head)
    .execute(&mut *winner)
    .await?;
    sqlx::query("UPDATE superseding SET hash = $1, archived = false, parent_hashes = ARRAY[$2]")
        .bind(head + 1)
        .bind(head)
        .execute(&mut *winner)
        .await?;
    sqlx::query("UPDATE script SET archived = true WHERE hash = $1")
        .bind(head)
        .execute(&mut *winner)
        .await?;
    sqlx::query("INSERT INTO script SELECT * FROM superseding")
        .execute(&mut *winner)
        .await?;
    winner.commit().await?;

    let resp = tokio::time::timeout(std::time::Duration::from_secs(20), update).await??;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "losing the race should not read as success: {body}"
    );
    assert!(
        body.contains("deployed to concurrently"),
        "the loser must say it was superseded, not that the script is missing: {body}"
    );

    Ok(())
}

/// Regression test for GHSA-2ppx-66jv-wpw5: a path-scoped token must only see
/// the scripts within its scope when listing, even though the route-level scope
/// check only validates `domain:action`. Before the fix, `list_search` (and
/// `list`) returned `path` + full `content` for every script the underlying
/// user could see, leaking out-of-scope script source to narrowly-scoped tokens.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_search_scope_filtering(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/scripts");

    // Create two folders and one script in each, as the (super-admin) test user.
    for folder in ["allowed", "private"] {
        let resp = authed(client().post(format!(
            "http://localhost:{port}/api/w/test-workspace/folders/create"
        )))
        .json(&json!({ "name": folder }))
        .send()
        .await
        .unwrap();
        assert_eq!(resp.status(), 200, "create folder: {}", resp.text().await?);
    }

    for (path, content) in [
        (
            "f/allowed/foo",
            "export async function main() { return 'allowed'; }",
        ),
        (
            "f/private/bar",
            "export async function main() { return 'secret'; }",
        ),
    ] {
        let resp = authed(client().post(format!("{base}/create")))
            .json(&new_script(path, "summary", content))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201, "create {path}: {}", resp.text().await?);
    }

    // Helper: GET /list_search with an arbitrary bearer token, returning the set
    // of script paths visible to that token.
    async fn list_search_paths(port: u16, token: &str) -> Vec<String> {
        let resp = client()
            .get(format!(
                "http://localhost:{port}/api/w/test-workspace/scripts/list_search"
            ))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        resp.json::<Vec<serde_json::Value>>()
            .await
            .unwrap()
            .into_iter()
            .map(|s| s["path"].as_str().unwrap().to_string())
            .collect()
    }

    // Insert three tokens for the same super-admin user, differing only by scope.
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes) VALUES
         (encode(sha256('SCOPED_TOKEN'::bytea), 'hex'), 'SCOPED_TOK', 'SCOPED_TOKEN', 'test@windmill.dev', 'scoped', true, ARRAY['scripts:read:f/allowed/*']),
         (encode(sha256('BROAD_TOKEN'::bytea), 'hex'), 'BROAD_TOK', 'BROAD_TOKEN', 'test@windmill.dev', 'broad', true, ARRAY['scripts:read']),
         (encode(sha256('TAG_TOKEN'::bytea), 'hex'), 'TAG_TOK', 'TAG_TOKEN', 'test@windmill.dev', 'tag-only', true, ARRAY['if_jobs:filter_tags:default'])",
    )
    .execute(&db)
    .await?;

    // Path-scoped token: only sees scripts within `f/allowed/*`.
    let scoped = list_search_paths(port, "SCOPED_TOKEN").await;
    assert!(
        scoped.contains(&"f/allowed/foo".to_string()),
        "scoped token should see f/allowed/foo, got: {scoped:?}"
    );
    assert!(
        !scoped.contains(&"f/private/bar".to_string()),
        "scoped token must NOT see f/private/bar, got: {scoped:?}"
    );

    // Broad `scripts:read` token: still sees every RLS-visible script.
    let broad = list_search_paths(port, "BROAD_TOKEN").await;
    assert!(broad.contains(&"f/allowed/foo".to_string()));
    assert!(
        broad.contains(&"f/private/bar".to_string()),
        "broad scripts:read token should see all scripts, got: {broad:?}"
    );

    // Tag-filter-only token is not scope-restricted: unchanged, sees all.
    let tag_only = list_search_paths(port, "TAG_TOKEN").await;
    assert!(tag_only.contains(&"f/allowed/foo".to_string()));
    assert!(tag_only.contains(&"f/private/bar".to_string()));

    // Unscoped token (no scopes column set): unchanged, sees all.
    let unscoped = list_search_paths(port, "SECRET_TOKEN").await;
    assert!(unscoped.contains(&"f/allowed/foo".to_string()));
    assert!(unscoped.contains(&"f/private/bar".to_string()));

    Ok(())
}
