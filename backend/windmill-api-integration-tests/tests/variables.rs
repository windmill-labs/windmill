use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

fn variable_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/variables/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(variable_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "variables_test"))]
async fn test_variable_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/variables");

    // --- exists ---
    let resp = authed_get(port, "exists", "u/test-user/plain_var").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get (plain) ---
    let resp = authed_get(port, "get", "u/test-user/plain_var").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/plain_var");
    assert_eq!(body["value"], "hello world");
    assert_eq!(body["is_secret"], false);
    assert_eq!(body["description"], "A plain variable");

    // --- get (secret, decrypt_secret=false) ---
    // fixture secrets are stored as plaintext, so skip decryption
    let resp = authed(client().get(format!(
        "{}/get/u/test-user/secret_var?decrypt_secret=false",
        base
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/secret_var");
    assert_eq!(body["is_secret"], true);
    assert_eq!(body["value"], serde_json::Value::Null);

    // get (secret, include_encrypted=true returns raw stored value)
    let resp = authed(client().get(format!(
        "{}/get/u/test-user/secret_var?decrypt_secret=false&include_encrypted=true",
        base
    )))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["is_secret"], true);
    assert!(body["value"].is_string(), "expected encrypted value string");

    // --- get not found ---
    let resp = authed_get(port, "get", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- get_value (plain) ---
    let resp = authed_get(port, "get_value", "u/test-user/plain_var").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<String>().await?, "hello world");

    let resp = authed_get(port, "get_value", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 3,
        "expected at least 3 variables from fixture, got {}",
        list.len()
    );
    assert!(list.iter().any(|v| v["path"] == "u/test-user/plain_var"));
    // secrets should have null value in list
    let secret = list
        .iter()
        .find(|v| v["path"] == "u/test-user/secret_var")
        .expect("secret_var missing from list");
    assert_eq!(secret["value"], serde_json::Value::Null);

    // list with path_start filter
    let resp = authed(client().get(format!("{base}/list?path_start=u/test-user/plain")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["path"], "u/test-user/plain_var");

    // --- list_contextual ---
    let resp = authed(client().get(format!("{base}/list_contextual")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());
    // should contain reserved variable names like WM_TOKEN, WM_WORKSPACE
    assert!(
        list.iter().any(|v| v["name"] == "WM_WORKSPACE"),
        "expected WM_WORKSPACE in contextual variables"
    );

    // --- create (plain) ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/new_var",
            "value": "new_value",
            "is_secret": false,
            "description": "Created in test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = authed_get(port, "exists", "u/test-user/new_var").await;
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "get_value", "u/test-user/new_var").await;
    assert_eq!(resp.json::<String>().await?, "new_value");

    // create duplicate -> 400
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/new_var",
            "value": "dup",
            "is_secret": false,
            "description": "Duplicate"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // --- create (secret) ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/new_secret",
            "value": "my_secret_val",
            "is_secret": true,
            "description": "A new secret"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // secret is stored encrypted, get_value should decrypt it
    let resp = authed_get(port, "get_value", "u/test-user/new_secret").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<String>().await?, "my_secret_val");

    // --- update (value) ---
    let resp = authed(client().post(variable_url(port, "update", "u/test-user/new_var")))
        .json(&json!({"value": "updated_value"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "get_value", "u/test-user/new_var").await;
    assert_eq!(resp.json::<String>().await?, "updated_value");

    // --- update (description) ---
    let resp = authed(client().post(variable_url(port, "update", "u/test-user/new_var")))
        .json(&json!({"description": "Updated desc"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "get", "u/test-user/new_var").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["description"], "Updated desc");

    // --- delete ---
    let resp = authed(client().delete(variable_url(port, "delete", "u/test-user/new_var")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "exists", "u/test-user/new_var").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // delete nonexistent -> 200 (no-op, doesn't error)
    let resp = authed(client().delete(variable_url(port, "delete", "u/test-user/new_var")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // --- delete_bulk ---
    let resp = authed(client().delete(format!("{base}/delete_bulk")))
        .json(&json!({"paths": ["u/test-user/new_secret", "u/test-user/another_var"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let deleted = resp.json::<Vec<String>>().await?;
    assert_eq!(deleted.len(), 2);

    let resp = authed_get(port, "exists", "u/test-user/new_secret").await;
    assert_eq!(resp.json::<bool>().await?, false);

    let resp = authed_get(port, "exists", "u/test-user/another_var").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // --- encrypt ---
    let resp = authed(client().post(format!("{base}/encrypt")))
        .json(&"test_plaintext".to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let encrypted = resp.text().await?;
    assert!(!encrypted.is_empty());
    assert_ne!(encrypted, "test_plaintext");

    Ok(())
}

/// Regression test: the variable-value cache (`get_value?allow_cache=true`) must be
/// identity-scoped. test-user-2 (folder access) warms the cache; test-user-3 (no access)
/// must then be denied rather than served the cached value.
#[sqlx::test(migrations = "../migrations", fixtures("base", "variable_cache_rls"))]
async fn test_variable_value_cache_is_identity_scoped(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let url = format!(
        "{}?allow_cache=true",
        variable_url(port, "get_value", "f/secret/cache_target_var")
    );
    let get = |token: &str| {
        client()
            .get(url.as_str())
            .header("Authorization", format!("Bearer {token}"))
    };

    // test-user-2 has folder access and WARMS the cache.
    let resp = get("SECRET_TOKEN_2").send().await?;
    assert_eq!(resp.status(), 200);
    assert!(resp.text().await?.contains("LEAKED_VAR_SECRET"));

    // test-user-3 has no folder access: must miss the cache and be denied (401), not leak.
    let resp = get("SECRET_TOKEN_3").send().await?;
    assert_eq!(resp.status(), 401);
    assert!(!resp.text().await?.contains("LEAKED_VAR_SECRET"));

    Ok(())
}

/// Secret variables ARE cached (with their per-read side effects — the EE
/// `variables.decrypt_secret` audit and running-job secret registration — re-run on every
/// hit; that re-emission is not observable in the OSS build since `audit_log` is a no-op).
/// We assert the caching itself: warm the cache, delete the row directly (no API/NOTIFY, so
/// the in-memory cache survives), and re-read with `allow_cache=true` — the value is still
/// returned from cache. A non-secret variable behaves identically (control).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_variables_are_cached(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/variables");

    let plain = "u/test-user/cache_plain_probe";
    let secret = "u/test-user/cache_secret_probe";

    // Create one non-secret and one secret variable (the secret is stored encrypted).
    for (path, value, is_secret) in [
        (plain, "PLAIN_PROBE", false),
        (secret, "SECRET_PROBE", true),
    ] {
        let resp = authed(client().post(format!("{base}/create")))
            .json(
                &json!({ "path": path, "value": value, "is_secret": is_secret, "description": "" }),
            )
            .send()
            .await?;
        assert_eq!(resp.status(), 201);
    }

    let read = |path: &str| {
        let url = format!("{base}/get_value/{path}?allow_cache=true");
        async move { authed(client().get(url)).send().await.unwrap() }
    };

    // Warm the cache for both.
    assert_eq!(read(plain).await.json::<String>().await?, "PLAIN_PROBE");
    assert_eq!(read(secret).await.json::<String>().await?, "SECRET_PROBE");

    // Delete both rows directly — bypasses the API and its NOTIFY-based invalidation, so
    // the in-memory cache survives. A subsequent read can only succeed from cache.
    for path in [plain, secret] {
        sqlx::query("DELETE FROM variable WHERE workspace_id = 'test-workspace' AND path = $1")
            .bind(path)
            .execute(&db)
            .await?;
    }

    // Both (secret included) are still served from the cache.
    assert_eq!(read(plain).await.json::<String>().await?, "PLAIN_PROBE");
    let resp = read(secret).await;
    assert_eq!(resp.status(), 200, "secret must still be served from cache");
    assert_eq!(resp.json::<String>().await?, "SECRET_PROBE");

    Ok(())
}
