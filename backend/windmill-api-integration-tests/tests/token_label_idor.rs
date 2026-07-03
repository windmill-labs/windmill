//! Regression tests for GHSA-8x8x-88qc-qp4r: token label collision bypassing job read
//! access control (IDOR).
//!
//! `username_override` is derived from a fully user-controlled token label, so a bare
//! `username_override == created_by` match in `require_job_read_access` is forgeable. The fix
//! binds that fast path to a non-forgeable attribute — the job's `permissioned_as_email` (the
//! token owner's email) must equal the caller's email. This:
//! - denies a colliding-label token created by a different principal, while
//! - still allowing a principal to re-read its own labeled-token jobs (incl. when RLS would
//!   otherwise hide them), and
//! - leaving user-facing webhook/http/email trigger token creation untouched (those labels
//!   are created through the public token API by design).

use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn bearer(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

async fn create_token_with_label(port: u16, caller_token: &str, label: &str) -> reqwest::Response {
    bearer(
        client().post(format!("http://localhost:{port}/api/users/tokens/create")),
        caller_token,
    )
    .json(&json!({ "label": label }))
    .send()
    .await
    .unwrap()
}

/// Insert a completed job with a labeled-token `created_by`, running as `permissioned_as`
/// (email `permissioned_as_email`) with the given `runnable_path` (which governs RLS).
async fn insert_labeled_job(
    db: &Pool<Postgres>,
    created_by: &str,
    runnable_path: &str,
    permissioned_as: &str,
    permissioned_as_email: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email, runnable_path, kind, tag, args, visible_to_owner)
         VALUES ($1, 'test-workspace', $2, $3, $4, $5, 'script', 'deno', '{}'::jsonb, true)",
    )
    .bind(id)
    .bind(created_by)
    .bind(permissioned_as)
    .bind(permissioned_as_email)
    .bind(runnable_path)
    .execute(db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, result, status)
         VALUES ($1, 'test-workspace', 100, '{\"secret\":\"super-secret-value\"}'::jsonb, 'success')",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();
    id
}

/// The core IDOR: an operator who mints a token whose label collides with another
/// principal's labeled-token identity must NOT be able to read that principal's job — the
/// `permissioned_as_email` of that job is the victim's, not the attacker's.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_label_collision_does_not_grant_job_read(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // A job submitted with a token labeled "collide", running as the admin (test-user).
    let job_id = insert_labeled_job(
        &db,
        "label-collide",
        "u/test-user/secret_script",
        "u/test-user",
        "test@windmill.dev",
    )
    .await;

    // Sanity: the admin can read it, so the job exists and is otherwise readable.
    let resp = bearer(
        client().get(format!("{base}/completed/get/{job_id}")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "admin must still read the job");

    // The attacker (a different member, test-user-2) mints a colliding-label token.
    let resp = create_token_with_label(port, "SECRET_TOKEN_2", "collide").await;
    assert_eq!(resp.status(), 201);
    let attacker_token = resp.text().await?;

    // Reading the admin's job with the colliding token must be denied. Before the fix the
    // `username_override == created_by` fast path returned the full result here.
    let resp = bearer(
        client().get(format!("{base}/completed/get/{job_id}")),
        &attacker_token,
    )
    .send()
    .await?;
    assert!(
        !resp.status().is_success(),
        "colliding-label token must not read another principal's job (got {})",
        resp.status()
    );
    let body = resp.text().await?;
    assert!(
        !body.contains("super-secret-value"),
        "job result must not leak to the colliding-label token"
    );

    Ok(())
}

/// The fix must not regress the legitimate case: a principal re-reading its own
/// labeled-token job is granted via the email-bound fast path, even when RLS would hide the
/// job (the runnable lives in another user's space the caller has no RLS path to).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_legit_labeled_self_read_still_works(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // Created by test-user-2's labeled token, running as test-user-2, but the runnable lives
    // under u/test-user so RLS alone would not reveal it to test-user-2 — the grant must come
    // from the email-bound fast path.
    let job_id = insert_labeled_job(
        &db,
        "label-mine",
        "u/test-user/shared_script",
        "u/test-user-2",
        "test2@windmill.dev",
    )
    .await;

    let resp = create_token_with_label(port, "SECRET_TOKEN_2", "mine").await;
    assert_eq!(resp.status(), 201);
    let token = resp.text().await?;

    let resp = bearer(
        client().get(format!("{base}/completed/get/{job_id}")),
        &token,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "owner must still read their own labeled-token job via the email-bound fast path"
    );

    Ok(())
}

/// P1 regression guard: the user-facing token API must keep accepting the labels that the
/// webhook / http-route / email trigger panels mint (e.g. `webhook-<user>-<rand>`). The fix
/// must not reserve those prefixes.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_trigger_token_labels_still_creatable(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    for label in [
        "webhook-test-user-2-ab12",
        "http-test-user-2-cd34",
        "email-test-user-2-ef56",
        "my-ci-token",
    ] {
        let resp = create_token_with_label(port, "SECRET_TOKEN_2", label).await;
        assert_eq!(
            resp.status(),
            201,
            "creating a token with label {label:?} must succeed"
        );
    }

    Ok(())
}
