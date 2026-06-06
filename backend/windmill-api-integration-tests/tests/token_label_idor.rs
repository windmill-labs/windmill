//! Regression tests for GHSA-8x8x-88qc-qp4r: token label collision bypassing job read
//! access control (IDOR).
//!
//! Two layers are covered:
//! - System-minted token labels (webhook-/http-/email-/ws-/ephemeral-*) must not be
//!   creatable through the user-facing token API (`is_reserved_token_label`).
//! - A user-forgeable `label-*` override must not satisfy the `created_by` fast path in
//!   `require_job_read_access`, while a legitimate self-read still resolves via RLS.

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

/// Insert a completed job whose `created_by` is a labeled-token identity, running in some
/// principal's space (`runnable_path`/`permissioned_as`), so RLS visibility is governed by
/// that path rather than by `created_by`.
async fn insert_labeled_job(
    db: &Pool<Postgres>,
    created_by: &str,
    runnable_path: &str,
    permissioned_as: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, runnable_path, kind, tag, args, visible_to_owner)
         VALUES ($1, 'test-workspace', $2, $3, $4, 'script', 'deno', '{}'::jsonb, true)",
    )
    .bind(id)
    .bind(created_by)
    .bind(permissioned_as)
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

/// Pure check of the reserved-label classifier (no DB / server needed).
#[test]
fn reserved_token_labels_are_recognized() {
    // System-minted prefixes that map to a trusted username_override must be reserved.
    for reserved in [
        "webhook-foo",
        "ephemeral-webhook-foo",
        "http-bar",
        "email-baz",
        "ws-qux",
        "ephemeral-script-end-user-admin",
        "ephemeral-script",
        "session",
        "Ephemeral lsp token",
    ] {
        assert!(
            windmill_api_auth::is_reserved_token_label(reserved),
            "{reserved:?} must be treated as a reserved system label"
        );
    }

    // Ordinary user labels (and the generic `label-*` form, which is not a reserved
    // *prefix* — it is produced internally, never accepted verbatim) must be allowed.
    for ok in ["myci", "deploy", "label-foo", "webhookish", "", "http"] {
        assert!(
            !windmill_api_auth::is_reserved_token_label(ok),
            "{ok:?} must not be treated as reserved"
        );
    }
}

/// The user-facing token creation API must reject system-minted labels but accept ordinary
/// ones.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_reserved_label_rejected_by_create_api(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // A non-admin member trying to forge a system identity is rejected.
    for reserved in [
        "webhook-evil",
        "http-evil",
        "ephemeral-script-end-user-admin",
    ] {
        let resp = create_token_with_label(port, "SECRET_TOKEN_2", reserved).await;
        assert_eq!(
            resp.status(),
            400,
            "creating a token with reserved label {reserved:?} must be rejected"
        );
    }

    // An ordinary label still works.
    let resp = create_token_with_label(port, "SECRET_TOKEN_2", "my-ci-token").await;
    assert_eq!(
        resp.status(),
        201,
        "creating a token with an ordinary label must succeed"
    );

    Ok(())
}

/// The core IDOR: an operator who creates a token whose label collides with another
/// principal's labeled-token identity must NOT be able to read that principal's job.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_label_collision_does_not_grant_job_read(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // A job submitted with a token labeled "collide", running in the admin's space.
    let job_id = insert_labeled_job(
        &db,
        "label-collide",
        "u/test-user/secret_script",
        "u/test-user",
    )
    .await;

    // Sanity: the admin can read it, so the job exists and is otherwise readable.
    let resp = bearer(
        client().get(format!("{base}/completed/get/{job_id}")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "admin must still be able to read the job"
    );

    // The attacker (non-admin member) mints a colliding-label token.
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

/// The fix must not regress the legitimate case: a member reading back their own
/// labeled-token job in their own space still works via RLS (no fast path needed).
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_legit_labeled_self_read_still_works(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/jobs");

    // A job created by test-user-2's labeled token, in test-user-2's own space.
    let job_id = insert_labeled_job(
        &db,
        "label-mine",
        "u/test-user-2/my_script",
        "u/test-user-2",
    )
    .await;

    // test-user-2 mints a token with that same label and reads its own job.
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
        "owner must still read their own labeled-token job via RLS"
    );

    Ok(())
}
