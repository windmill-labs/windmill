//! E2E regression tests for OAuth token refresh persistence through the
//! configured secret backend.
//!
//! Regression for windmill#9471 / windmill-ee-private#607: the lazy on-fetch
//! OAuth refresh used to persist the freshly minted token with a raw
//! `UPDATE variable SET value = <db-encrypted>`, bypassing the secret-backend
//! abstraction. With an external backend (AWS Secrets Manager / Azure Key
//! Vault / Vault) reads resolve through the backend and ignore `variable.value`
//! entirely, so the external store stayed frozen at its connect-time token and
//! every read that did not itself trigger a mint served a stale/expired token.
//!
//! These tests exercise the persistence step (`store_oauth_token_value`) — the
//! exact code path that was fixed — against both the database backend and an
//! external (AWS Secrets Manager via LocalStack) backend, plus the self-healing
//! reset on a failed persist. They are opt-in (they mutate the shared
//! `global_settings.secret_backend` row and workspace/variable/account rows on a
//! real DB) and skip unless `RUN_SECRET_BACKEND_E2E=1` is set.
//!
//! ## Run
//!
//! Database-backend case (needs a migrated DB). Note `RUN_SECRET_BACKEND_E2E=1`
//! is required or every test skips:
//!
//! ```bash
//! RUN_SECRET_BACKEND_E2E=1 \
//! DATABASE_URL=postgres://postgres:changeme@127.0.0.1:5432/windmill \
//!   cargo test -p windmill-store --features private,enterprise,oauth2 \
//!   oauth_refresh_secret_backend_tests -- --nocapture --test-threads=1
//! ```
//!
//! External-backend cases additionally need LocalStack `secretsmanager` and
//! `RUN_AWS_SM_TESTS=1`:
//!
//! ```bash
//! docker run -d -e SERVICES=secretsmanager localstack/localstack:3.8
//! RUN_SECRET_BACKEND_E2E=1 RUN_AWS_SM_TESTS=1 AWS_SM_ENDPOINT=http://<localstack-ip>:4566 \
//! DATABASE_URL=postgres://postgres:changeme@127.0.0.1:5432/windmill \
//!   cargo test -p windmill-store --features private,enterprise,oauth2 \
//!   oauth_refresh_secret_backend_tests -- --nocapture --test-threads=1
//! ```

use crate::secret_backend_ext::{get_secret_value, store_oauth_token_value, store_secret_value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

// global_settings holds a single `secret_backend` row shared across tests;
// serialize the test bodies so concurrent runs don't clobber each other's
// configured backend. (Also run with --test-threads=1 for good measure.)
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

// Opt-in gate so the suite never runs (and mutates shared DB state) as part of a
// normal `cargo test` invocation.
fn run_e2e() -> bool {
    env_flag("RUN_SECRET_BACKEND_E2E")
}

fn run_aws_sm() -> bool {
    env_flag("RUN_AWS_SM_TESTS")
}

/// Restore the default (database) backend so we don't leave the instance
/// pointed at a test backend for any concurrently-running suite.
async fn reset_backend(db: &Pool<Postgres>) {
    set_backend(db, serde_json::json!({ "type": "Database" })).await;
}

fn aws_sm_endpoint() -> String {
    std::env::var("AWS_SM_ENDPOINT").unwrap_or_else(|_| "http://localhost:4566".to_string())
}

async fn db() -> Pool<Postgres> {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point at a migrated windmill database");
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("connect to DATABASE_URL")
}

/// Fresh workspace + key + clean variable/account rows for `w_id`.
async fn setup_workspace(db: &Pool<Postgres>, w_id: &str) {
    sqlx::query("DELETE FROM variable WHERE workspace_id = $1")
        .bind(w_id)
        .execute(db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM account WHERE workspace_id = $1")
        .bind(w_id)
        .execute(db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM workspace_key WHERE workspace_id = $1")
        .bind(w_id)
        .execute(db)
        .await
        .unwrap();
    sqlx::query("DELETE FROM workspace WHERE id = $1")
        .bind(w_id)
        .execute(db)
        .await
        .unwrap();

    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ($1, $1, 'admin@windmill.dev')")
        .bind(w_id)
        .execute(db)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO workspace_key (workspace_id, kind, key) VALUES ($1, 'cloud', 'e2ekey')",
    )
    .bind(w_id)
    .execute(db)
    .await
    .unwrap();
}

async fn set_backend(db: &Pool<Postgres>, config: serde_json::Value) {
    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ('secret_backend', $1) \
         ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(config)
    .execute(db)
    .await
    .unwrap();
}

fn aws_sm_config(endpoint: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "AwsSecretsManager",
        "region": "us-east-1",
        "access_key_id": "test",
        "secret_access_key": "test",
        "endpoint_url": endpoint,
        "prefix": "windmill-e2e/"
    })
}

/// Simulate `Connect`: store the initial token through the backend and create
/// the linked secret variable + account (expired, with a refresh token).
async fn simulate_connect(db: &Pool<Postgres>, w_id: &str, path: &str, initial_token: &str) -> i32 {
    let stored = store_secret_value(db, w_id, path, initial_token)
        .await
        .expect("store initial token");

    let account_id: i32 = sqlx::query_scalar(
        "INSERT INTO account (workspace_id, expires_at, refresh_token, client, grant_type) \
         VALUES ($1, now() - interval '1 hour', 'rt_dummy', 'gdrive', 'authorization_code') \
         RETURNING id",
    )
    .bind(w_id)
    .fetch_one(db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO variable (workspace_id, path, value, is_secret, is_oauth, account, expires_at) \
         VALUES ($1, $2, $3, true, true, $4, now() - interval '1 hour')",
    )
    .bind(w_id)
    .bind(path)
    .bind(&stored)
    .bind(account_id)
    .execute(db)
    .await
    .unwrap();

    account_id
}

async fn variable_value(db: &Pool<Postgres>, w_id: &str, path: &str) -> String {
    sqlx::query_scalar("SELECT value FROM variable WHERE workspace_id = $1 AND path = $2")
        .bind(w_id)
        .bind(path)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn account_expires_in_past(db: &Pool<Postgres>, w_id: &str, account_id: i32) -> bool {
    sqlx::query_scalar("SELECT expires_at < now() FROM account WHERE workspace_id = $1 AND id = $2")
        .bind(w_id)
        .bind(account_id)
        .fetch_one(db)
        .await
        .unwrap()
}

/// Database backend (the "without external storage" case): refresh must
/// re-encrypt the new token into `variable.value`; reads serve the new token.
#[tokio::test]
async fn database_backend_persists_refreshed_token() {
    if !run_e2e() {
        println!(
            "Skipping database_backend_persists_refreshed_token: set RUN_SECRET_BACKEND_E2E=1"
        );
        return;
    }
    let _guard = SERIAL.lock().await;
    let db = db().await;
    let w_id = "wm_e2e_db";
    let path = "f/google/gdrive";

    set_backend(&db, serde_json::json!({ "type": "Database" })).await;
    setup_workspace(&db, w_id).await;
    let _ = simulate_connect(&db, w_id, path, "OLD_TOKEN").await;

    // Connect-time token is served.
    let v = variable_value(&db, w_id, path).await;
    assert_eq!(
        get_secret_value(&db, w_id, path, &v).await.unwrap(),
        "OLD_TOKEN"
    );

    // Refresh persists the new token.
    store_oauth_token_value(&db, w_id, path, "NEW_TOKEN")
        .await
        .unwrap();

    let v = variable_value(&db, w_id, path).await;
    assert_eq!(
        get_secret_value(&db, w_id, path, &v).await.unwrap(),
        "NEW_TOKEN",
        "database backend should serve the refreshed token"
    );
    println!("  ✓ database backend serves refreshed token");
    reset_backend(&db).await;
}

/// External backend (the "with external storage" case): refresh must write
/// the new token to AWS Secrets Manager. Before the fix the external store
/// stayed frozen and reads served the stale connect-time token.
#[tokio::test]
async fn external_backend_persists_refreshed_token() {
    if !run_e2e() || !run_aws_sm() {
        println!("Skipping external_backend_persists_refreshed_token: set RUN_SECRET_BACKEND_E2E=1 and RUN_AWS_SM_TESTS=1");
        return;
    }
    let _guard = SERIAL.lock().await;
    let db = db().await;
    let w_id = "wm_e2e_awssm";
    let path = "f/google/gsheets";

    set_backend(&db, aws_sm_config(&aws_sm_endpoint())).await;
    setup_workspace(&db, w_id).await;
    let _ = simulate_connect(&db, w_id, path, "OLD_TOKEN").await;

    // Connect-time token is served from the external store.
    let marker = variable_value(&db, w_id, path).await;
    assert!(
        marker.starts_with("$aws_sm:"),
        "external backend should store a marker in variable.value, got {marker}"
    );
    assert_eq!(
        get_secret_value(&db, w_id, path, &marker).await.unwrap(),
        "OLD_TOKEN"
    );

    // Demonstrate the original bug shape: a raw DB write to variable.value is
    // futile because reads resolve through the backend and ignore it.
    sqlx::query(
        "UPDATE variable SET value = 'ignored_db_blob' WHERE workspace_id = $1 AND path = $2",
    )
    .bind(w_id)
    .bind(path)
    .execute(&db)
    .await
    .unwrap();
    assert_eq!(
        get_secret_value(&db, w_id, path, "ignored_db_blob")
            .await
            .unwrap(),
        "OLD_TOKEN",
        "reads ignore variable.value for external backends — a raw UPDATE can't refresh the served token"
    );

    // The fix: persist through the backend.
    store_oauth_token_value(&db, w_id, path, "NEW_TOKEN")
        .await
        .unwrap();

    let marker = variable_value(&db, w_id, path).await;
    assert_eq!(
        get_secret_value(&db, w_id, path, &marker).await.unwrap(),
        "NEW_TOKEN",
        "external backend should serve the refreshed token written back to AWS SM"
    );
    println!("  ✓ external (AWS SM) backend serves refreshed token written back to the store");
    reset_backend(&db).await;
}

/// If persisting the refreshed token fails (e.g. transient external-backend
/// error) after the account was committed fresh, the account expiry must be
/// reset to the past so the next fetch retries instead of serving a stale
/// token for the whole token lifetime.
#[tokio::test]
async fn failed_persist_resets_account_expiry() {
    if !run_e2e() || !run_aws_sm() {
        println!("Skipping failed_persist_resets_account_expiry: set RUN_SECRET_BACKEND_E2E=1 and RUN_AWS_SM_TESTS=1");
        return;
    }
    let _guard = SERIAL.lock().await;
    let db = db().await;
    let w_id = "wm_e2e_selfheal";
    let path = "f/google/gdrive";

    // Working backend first to seed the variable + a *fresh* account.
    set_backend(&db, aws_sm_config(&aws_sm_endpoint())).await;
    setup_workspace(&db, w_id).await;
    let account_id = simulate_connect(&db, w_id, path, "OLD_TOKEN").await;
    sqlx::query("UPDATE account SET expires_at = now() + interval '1 hour' WHERE workspace_id = $1 AND id = $2")
        .bind(w_id)
        .bind(account_id)
        .execute(&db)
        .await
        .unwrap();
    assert!(!account_expires_in_past(&db, w_id, account_id).await);

    // Point the backend at an unreachable endpoint so the persist fails.
    set_backend(&db, aws_sm_config("http://127.0.0.1:1")).await;

    let res = store_oauth_token_value(&db, w_id, path, "NEW_TOKEN").await;
    assert!(res.is_err(), "persist to unreachable backend should fail");
    assert!(
        account_expires_in_past(&db, w_id, account_id).await,
        "a failed persist must reset account.expires_at to the past so refresh retries"
    );
    println!("  ✓ failed persist reset account expiry (self-healing)");
    reset_backend(&db).await;
}
