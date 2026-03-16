//! Tests for the token hash migration.
//!
//! Verifies that:
//! - Rust hash_token() matches PostgreSQL's encode(sha256(...),'hex')
//! - Newly created tokens can authenticate immediately
//! - Token list/delete-by-prefix works with the new token_prefix column
//! - Logout invalidates tokens via hash-based deletion
//! - Backward compat: plaintext column is populated when old workers exist
//! - rotate_webhook_token produces valid tokens and defers old token deletion

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::auth::{hash_token, TOKEN_PREFIX_LEN};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn authed_with(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Test 1: Verify that Rust's hash_token() produces the same hash as PostgreSQL's
/// encode(sha256(token::bytea), 'hex'). This is the foundational invariant.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_hash_consistency(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Compute hash in Rust
    let rust_hash = hash_token("SECRET_TOKEN");

    // Compute hash in PostgreSQL
    let pg_hash: String =
        sqlx::query_scalar!("SELECT encode(sha256('SECRET_TOKEN'::bytea), 'hex') AS hash")
            .fetch_one(&db)
            .await?
            .unwrap();

    assert_eq!(
        rust_hash, pg_hash,
        "Rust hash_token() must match PostgreSQL sha256()"
    );

    // Also verify it matches what's stored in the fixture
    let stored_hash: String = sqlx::query_scalar!(
        "SELECT token_hash FROM token WHERE email = 'test@windmill.dev' AND label = 'test token'"
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(
        rust_hash, stored_hash,
        "hash_token() must match the fixture's pre-computed hash"
    );

    Ok(())
}

/// Test 2: Create a token via API, then immediately use it to authenticate.
/// Verifies create_token_internal stores the hash correctly and auth lookups work.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_create_token_and_auth(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/users");

    // Create a new token
    let resp = authed(client().post(format!("{base}/tokens/create")))
        .json(&json!({"label": "test-hash-token"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    let new_token = resp.text().await?;
    assert!(!new_token.is_empty());

    // Use the new token to call whoami
    let resp = authed_with(client().get(format!("{base}/whoami")), &new_token)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "newly created token must authenticate");
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["email"], "test@windmill.dev");

    // Verify the hash is stored correctly in DB
    let expected_hash = hash_token(&new_token);
    let db_hash: Option<String> = sqlx::query_scalar!(
        "SELECT token_hash FROM token WHERE token_hash = $1",
        expected_hash
    )
    .fetch_optional(&db)
    .await?;
    assert!(db_hash.is_some(), "token_hash must be stored in DB");

    Ok(())
}

/// Test 3: Create a token, list tokens (verify prefix), delete by prefix, confirm invalid.
/// Covers the change from WHERE token LIKE to WHERE token_prefix = $1.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_token_list_and_delete_by_prefix(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/users");

    // Create a token
    let resp = authed(client().post(format!("{base}/tokens/create")))
        .json(&json!({"label": "prefix-test-token"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    let new_token = resp.text().await?;
    let prefix = &new_token[..TOKEN_PREFIX_LEN];

    // List tokens and find our token by prefix
    let resp = authed(client().get(format!("{base}/tokens/list")))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let tokens = resp.json::<Vec<serde_json::Value>>().await?;
    let found = tokens
        .iter()
        .any(|t| t["token_prefix"].as_str() == Some(prefix));
    assert!(found, "token with prefix {prefix} must appear in list");

    // Verify the new token works
    let resp = authed_with(client().get(format!("{base}/whoami")), &new_token)
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    // Delete by prefix
    let resp = authed(client().delete(format!("{base}/tokens/delete/{prefix}")))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "delete token: {}", resp.text().await?);

    // Confirm the token is gone from the DB (auth cache may still serve 200 briefly)
    let token_hash = hash_token(&new_token);
    let deleted: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM token WHERE token_hash = $1) AS exists",
        token_hash
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(true);
    assert!(
        !deleted,
        "token must be deleted from DB after delete-by-prefix"
    );

    Ok(())
}

/// Test 4: Logout invalidates a token via hash-based deletion.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_logout_invalidates_token(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/users");
    let auth_base = format!("http://localhost:{port}/api/auth");

    // Create a fresh token (don't burn the fixture token)
    let resp = authed(client().post(format!("{base}/tokens/create")))
        .json(&json!({"label": "logout-test-token"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    let token = resp.text().await?;

    // Verify it works
    let resp = authed_with(client().get(format!("{base}/whoami")), &token)
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    // Logout with the token
    let resp = authed_with(client().post(format!("{auth_base}/logout")), &token)
        .send()
        .await?;
    assert!(
        resp.status() == 200 || resp.status() == 303,
        "logout: unexpected status {}",
        resp.status()
    );

    // Confirm the token is gone from the DB (auth cache may still serve 200 briefly)
    let token_hash = hash_token(&token);
    let exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM token WHERE token_hash = $1) AS exists",
        token_hash
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(true);
    assert!(!exists, "token must be deleted from DB after logout");

    Ok(())
}

/// Test 5: Backward compatibility — plaintext column behavior based on MIN_VERSION.
/// When old workers exist (MIN_VERSION < 1.650.0), plaintext must be written so
/// old workers running WHERE token = $1 can still authenticate.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_plaintext_backward_compat(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    use windmill_common::min_version::{MIN_VERSION, MIN_VERSION_SUPPORTS_TOKEN_HASH};

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/users");

    // --- Phase 1: Simulate old workers present (version < 1.650.0) ---
    // Set MIN_VERSION to one minor below the token hash feature version
    let mut old_version = MIN_VERSION_SUPPORTS_TOKEN_HASH.version().clone();
    old_version.minor -= 1;
    *MIN_VERSION.write().await = old_version;

    let resp = authed(client().post(format!("{base}/tokens/create")))
        .json(&json!({"label": "old-worker-compat-token"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    let old_compat_token = resp.text().await?;
    let old_compat_hash = hash_token(&old_compat_token);

    // Plaintext should be stored (for old workers)
    let plaintext: Option<String> = sqlx::query_scalar!(
        "SELECT token FROM token WHERE token_hash = $1",
        old_compat_hash
    )
    .fetch_one(&db)
    .await?;
    assert!(
        plaintext.is_some(),
        "plaintext must be stored when old workers exist"
    );
    assert_eq!(plaintext.unwrap(), old_compat_token);

    // Old-style query (what old workers run) must find the token
    let old_style_email: Option<String> = sqlx::query_scalar!(
        "SELECT email FROM token WHERE token = $1 AND (expiration > NOW() OR expiration IS NULL)",
        &old_compat_token
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    assert_eq!(
        old_style_email.as_deref(),
        Some("test@windmill.dev"),
        "old-style WHERE token = $1 must find the token"
    );

    // New-style query must also work
    let new_style_email: Option<String> = sqlx::query_scalar!(
        "SELECT email FROM token WHERE token_hash = $1 AND (expiration > NOW() OR expiration IS NULL)",
        old_compat_hash
    )
    .fetch_optional(&db)
    .await?
    .flatten();
    assert_eq!(
        new_style_email.as_deref(),
        Some("test@windmill.dev"),
        "new-style WHERE token_hash = $1 must also work"
    );

    // --- Phase 2: All workers upgraded (version >= 1.650.0) ---
    *MIN_VERSION.write().await = MIN_VERSION_SUPPORTS_TOKEN_HASH.version().clone();

    let resp = authed(client().post(format!("{base}/tokens/create")))
        .json(&json!({"label": "new-worker-token"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    let new_token = resp.text().await?;
    let new_hash = hash_token(&new_token);

    // Plaintext should NOT be stored
    let plaintext: Option<String> =
        sqlx::query_scalar!("SELECT token FROM token WHERE token_hash = $1", new_hash)
            .fetch_one(&db)
            .await?;
    assert!(
        plaintext.is_none(),
        "plaintext must be NULL when all workers support hash"
    );

    // Old-style query should NOT find this token
    let old_style_result: Option<String> =
        sqlx::query_scalar!("SELECT email FROM token WHERE token = $1", &new_token)
            .fetch_optional(&db)
            .await?
            .flatten();
    assert!(
        old_style_result.is_none(),
        "old-style query must not find token when plaintext is NULL"
    );

    // New-style query must still work
    let resp = authed_with(client().get(format!("{base}/whoami")), &new_token)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        200,
        "new token must authenticate via hash lookup"
    );

    Ok(())
}

/// Test 6: rotate_webhook_token atomically creates a new token and deletes the old one.
/// Returns new_token_hash so callers can roll back on subsequent failure.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_rotate_webhook_token(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    use windmill_native_triggers::rotate_webhook_token;

    // Insert a token directly with known values
    let original_token = "test-webhook-token-original-1234";
    let original_hash = hash_token(original_token);
    let original_prefix = &original_token[..TOKEN_PREFIX_LEN];

    sqlx::query!(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin)
         VALUES ($1, $2, $3, 'test@windmill.dev', 'webhook-test', false)",
        original_hash,
        original_prefix,
        original_token,
    )
    .execute(&db)
    .await?;

    // Rotate the token
    let rotated = rotate_webhook_token(&db, &original_hash)
        .await?
        .expect("rotate must return Some for existing token");

    // New token should be different
    assert_ne!(rotated.new_token, original_token);

    // new_token_hash should match the hash of the new token
    let expected_new_hash = hash_token(&rotated.new_token);
    assert_eq!(rotated.new_token_hash, expected_new_hash);

    // New token's hash should exist in DB
    let exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM token WHERE token_hash = $1) AS exists",
        rotated.new_token_hash
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(false);
    assert!(exists, "new token hash must exist in DB after rotation");

    // Old token should be gone (deleted atomically during rotation)
    let old_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM token WHERE token_hash = $1) AS exists",
        original_hash
    )
    .fetch_one(&db)
    .await?
    .unwrap_or(true);
    assert!(
        !old_exists,
        "old token must be deleted atomically during rotation"
    );

    // Rotating a non-existent hash should return None
    let result = rotate_webhook_token(&db, &original_hash).await?;
    assert!(
        result.is_none(),
        "rotating a deleted token must return None"
    );

    Ok(())
}
