use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const WS: &str = "test-workspace";

/// A reqwest client that sends `Authorization: Bearer <token>`. The base
/// fixture seeds: SECRET_TOKEN (test-user, admin), SECRET_TOKEN_2
/// (test-user-2, non-admin), SECRET_TOKEN_3 (test-user-3, non-admin).
fn client_for(token: &str) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    reqwest::ClientBuilder::new()
        .default_headers(headers)
        .build()
        .unwrap()
}

fn save_url(port: u16, kind: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/{WS}/drafts/update/{kind}/{path}")
}

async fn draft_count(db: &Pool<Postgres>, path: &str, kind: &str, email: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM draft WHERE workspace_id = $1 AND path = $2 \
         AND typ = $3::text::DRAFT_KIND AND email = $4",
    )
    .bind(WS)
    .bind(path)
    .bind(kind)
    .bind(email)
    .fetch_one(db)
    .await
    .unwrap()
}

/// Upsert → conflict (stale last_sync) → force-overwrite → delete, the
/// optimistic-concurrency contract `update_draft` exists to enforce.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_draft_conflict_lifecycle(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let c = client_for("SECRET_TOKEN");
    let path = "u/test-user/draft_x";
    let url = save_url(port, "script", path);

    // First save: no last_sync ("treat as fresh") → saved.
    let r = c
        .post(&url)
        .json(&json!({ "value": { "n": 1 } }))
        .send()
        .await?;
    assert_eq!(r.status(), 200, "first save");
    let body: serde_json::Value = r.json().await?;
    assert_eq!(body["status"], "saved");
    let ts1 = body["current_timestamp"].as_str().unwrap().to_string();
    assert_eq!(
        draft_count(&db, path, "script", "test@windmill.dev").await,
        1
    );

    // A tiny gap so the next now() is strictly greater than ts1.
    tokio::time::sleep(std::time::Duration::from_millis(15)).await;

    // Save with the matching last_sync → not stale → saved, newer ts.
    let r = c
        .post(&url)
        .json(&json!({ "value": { "n": 2 }, "last_sync": ts1 }))
        .send()
        .await?;
    let body: serde_json::Value = r.json().await?;
    assert_eq!(body["status"], "saved", "in-order save");
    let ts2 = body["current_timestamp"].as_str().unwrap().to_string();
    assert_ne!(ts1, ts2, "timestamp should advance");

    // Save with the now-stale ts1 → conflict, server reports its current ts.
    let r = c
        .post(&url)
        .json(&json!({ "value": { "n": 3 }, "last_sync": ts1 }))
        .send()
        .await?;
    let body: serde_json::Value = r.json().await?;
    assert_eq!(body["status"], "conflict", "stale save must conflict");
    assert_eq!(body["current_timestamp"].as_str().unwrap(), ts2);

    // The conflicting write must NOT have landed — value is still {n:2}.
    let stored: serde_json::Value = sqlx::query_scalar::<_, sqlx::types::Json<serde_json::Value>>(
        "SELECT value FROM draft WHERE workspace_id = $1 AND path = $2 \
         AND typ = 'script' AND email = 'test@windmill.dev'",
    )
    .bind(WS)
    .bind(path)
    .fetch_one(&db)
    .await?
    .0;
    assert_eq!(stored["n"], 2, "conflicting write must be rejected");

    // force = true overrides the conflict check.
    let r = c
        .post(&url)
        .json(&json!({ "value": { "n": 3 }, "last_sync": ts1, "force": true }))
        .send()
        .await?;
    assert_eq!(
        r.json::<serde_json::Value>().await?["status"],
        "saved",
        "force"
    );

    // Delete (value: null) → saved, row gone.
    let r = c.post(&url).json(&json!({ "value": null })).send().await?;
    assert_eq!(
        r.json::<serde_json::Value>().await?["status"],
        "saved",
        "delete"
    );
    assert_eq!(
        draft_count(&db, path, "script", "test@windmill.dev").await,
        0,
        "row removed after delete"
    );

    Ok(())
}

/// require_can_write_path: own namespace allowed, another user's namespace
/// rejected, operators rejected outright.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_draft_write_authorization(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let user2 = client_for("SECRET_TOKEN_2"); // test-user-2, non-admin

    // Own namespace → allowed.
    let r = user2
        .post(save_url(port, "script", "u/test-user-2/own"))
        .json(&json!({ "value": { "a": 1 } }))
        .send()
        .await?;
    assert_eq!(r.status(), 200, "own namespace allowed");

    // Another user's namespace, no grant → rejected.
    let r = user2
        .post(save_url(port, "script", "u/test-user/theirs"))
        .json(&json!({ "value": { "a": 1 } }))
        .send()
        .await?;
    assert_eq!(r.status(), 401, "other user's namespace rejected");

    // Operators can't save drafts at all.
    sqlx::query(
        "UPDATE usr SET operator = true WHERE workspace_id = $1 AND username = 'test-user-3'",
    )
    .bind(WS)
    .execute(&db)
    .await?;
    let op = client_for("SECRET_TOKEN_3");
    let r = op
        .post(save_url(port, "script", "u/test-user-3/own"))
        .json(&json!({ "value": { "a": 1 } }))
        .send()
        .await?;
    assert_eq!(r.status(), 401, "operator rejected");

    Ok(())
}

/// The item-level extra_perms fallback: a user granted write on a deployed
/// item (via the Share dialog) can save a draft on it even though it's
/// outside their namespace. Regression test for the authz drop.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_update_draft_extra_perms_writer(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let path = "u/test-user/shared";

    // A deployed script owned by test-user, shared with test-user-2 (write).
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, \
            language, schema, extra_perms, created_by) \
         VALUES ($1, 1, $2, '', '', 'x', 'deno', '{}'::jsonb, \
            '{\"u/test-user-2\": true}'::jsonb, 'test-user')",
    )
    .bind(WS)
    .bind(path)
    .execute(&db)
    .await?;

    let user2 = client_for("SECRET_TOKEN_2");
    let r = user2
        .post(save_url(port, "script", path))
        .json(&json!({ "value": { "a": 1 } }))
        .send()
        .await?;
    assert_eq!(
        r.status(),
        200,
        "extra_perms writer can save a draft: {}",
        r.text().await?
    );

    // Without a grant on a different shared item → still rejected.
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, \
            language, schema, extra_perms, created_by) \
         VALUES ($1, 2, 'u/test-user/private', '', '', 'x', 'deno', '{}'::jsonb, \
            '{}'::jsonb, 'test-user')",
    )
    .bind(WS)
    .execute(&db)
    .await?;
    let r = user2
        .post(save_url(port, "script", "u/test-user/private"))
        .json(&json!({ "value": { "a": 1 } }))
        .send()
        .await?;
    assert_eq!(r.status(), 401, "no grant → rejected");

    // A READ-ONLY grant (`extra_perms` value false) must not allow draft
    // saves: the write check defers to RLS via `SELECT ... FOR UPDATE`,
    // and locking applies the UPDATE policies — visibility under the
    // SELECT policy alone isn't enough. Pins the FOR UPDATE semantics the
    // probe relies on.
    sqlx::query(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, \
            language, schema, extra_perms, created_by) \
         VALUES ($1, 3, 'u/test-user/readonly', '', '', 'x', 'deno', '{}'::jsonb, \
            '{\"u/test-user-2\": false}'::jsonb, 'test-user')",
    )
    .bind(WS)
    .execute(&db)
    .await?;
    let r = user2
        .post(save_url(port, "script", "u/test-user/readonly"))
        .json(&json!({ "value": { "a": 1 } }))
        .send()
        .await?;
    assert_eq!(r.status(), 401, "read-only grant → rejected");

    Ok(())
}

/// Cross-user draft viewing (`GET /drafts/get/{kind}/{path}`) is disabled
/// for the drawer kinds (resource/variable/triggers) so a viewer can't
/// read another user's draft; it stays available for script/flow/app.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_cross_user_draft_privacy(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // test-user saves a variable draft and a script draft in a shared folder.
    let admin = client_for("SECRET_TOKEN");
    admin
        .post(save_url(port, "variable", "f/shared/v"))
        .json(&json!({ "value": { "variable": { "value": "x", "is_secret": false } } }))
        .send()
        .await?;

    let user2 = client_for("SECRET_TOKEN_2");
    // Variable is a drawer kind → cross-user view is forbidden regardless of
    // path access (the kind gate fires first).
    let r = user2
        .get(format!(
            "http://localhost:{port}/api/w/{WS}/drafts/get/variable/f/shared/v?username=test-user"
        ))
        .send()
        .await?;
    assert_eq!(
        r.status(),
        404,
        "variable drafts are private to their owner"
    );

    // Sharing kinds (script) are NOT gated by the kind check — a missing
    // draft / no access yields 404 too, but the "private to their owner"
    // wording is specific to the drawer kinds, so assert it's absent here.
    let r = user2
        .get(format!(
            "http://localhost:{port}/api/w/{WS}/drafts/get/script/f/shared/s?username=test-user"
        ))
        .send()
        .await?;
    let body = r.text().await?;
    assert!(
        !body.contains("private to their owner"),
        "script kind must not be blocked by the cross-user privacy gate: {body}"
    );

    Ok(())
}
