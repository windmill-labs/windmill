//! Regression tests for GHSA-vm75-gmpw-rvp9: the unauthenticated `/api/slack` callback must
//! not be drivable into decrypting arbitrary workspace variables.
//!
//! The OpenModal branch reaches `get_slack_token` (a privileged, RLS-bypassing variable
//! decryption). It is now gated by a per-workspace HMAC over (w_id, job_id, path) — the same
//! workspace key used to sign resume URLs. Without a valid signature the request is rejected
//! with 401 before any decryption, even when `SLACK_SIGNING_SECRET` is unset (the default).

use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

/// Re-implementation of the server's `sign_slack_payload` for the positive-control test.
/// The fixture sets `workspace_key.key = 'test-key'` for `test-workspace`.
fn sign(w_id: &str, parts: &[&[u8]]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(b"test-key").unwrap();
    mac.update(b"slack_payload_v1\0"); // SLACK_PAYLOAD_HMAC_DOMAIN
    mac.update(w_id.as_bytes());
    for p in parts {
        mac.update(b"\0");
        mac.update(p);
    }
    hex::encode(mac.finalize().into_bytes())
}

/// POST an `open_modal` block action to the unauthenticated `/api/slack` callback.
async fn post_open_modal(port: u16, value: serde_json::Value) -> reqwest::Response {
    let payload = json!({
        "type": "block_actions",
        "trigger_id": "trigger-123",
        "container": { "message_ts": "0", "channel_id": "C1" },
        "actions": [ { "action_id": "open_modal", "value": value.to_string() } ],
    });
    client()
        .post(format!("http://localhost:{port}/api/slack"))
        .form(&[("payload", payload.to_string())])
        .send()
        .await
        .unwrap()
}

/// POST a `view_submission` to the unauthenticated `/api/slack` callback with the given
/// private_metadata.
async fn post_view_submission(port: u16, private_metadata: serde_json::Value) -> reqwest::Response {
    let payload = json!({
        "type": "view_submission",
        "view": {
            "state": { "values": {} },
            "private_metadata": private_metadata.to_string(),
        },
    });
    client()
        .post(format!("http://localhost:{port}/api/slack"))
        .form(&[("payload", payload.to_string())])
        .send()
        .await
        .unwrap()
}

/// A submission with an unsigned (or tampered) `private_metadata` must be rejected with 401
/// BEFORE the resume/cancel action runs — the signature gate is checked first. The resume_url
/// here is well-formed (so it parses) but never acted upon.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_view_submission_without_signature_is_rejected(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let job_id = Uuid::new_v4();
    let resume_url = format!("/api/w/test-workspace/jobs_u/resume/{job_id}/1/deadbeef");

    let resp = post_view_submission(
        port,
        json!({
            "resume_url": resume_url,
            "resource_path": "u/admin/secret",
            "container": { "message_ts": "0", "channel_id": "C1" },
            "hide_cancel": false,
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        401,
        "unsigned submission must be rejected before the resume action"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_open_modal_without_signature_is_rejected(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let job_id = Uuid::new_v4();

    // No signature → must be rejected with 401 before any variable lookup. Before the fix
    // this reached `get_slack_token` and forced decryption of `u/admin/secret`.
    let resp = post_open_modal(
        port,
        json!({
            "w_id": "test-workspace",
            "job_id": job_id.to_string(),
            "path": "u/admin/secret",
            "flow_step_id": "a",
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        401,
        "unsigned OpenModal callback must be rejected"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_open_modal_with_wrong_signature_is_rejected(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let job_id = Uuid::new_v4();

    let resp = post_open_modal(
        port,
        json!({
            "w_id": "test-workspace",
            "job_id": job_id.to_string(),
            "path": "u/admin/secret",
            "flow_step_id": "a",
            "signature": "deadbeef",
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        401,
        "OpenModal callback with an invalid signature must be rejected"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_open_modal_with_tampered_path_is_rejected(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let job_id = Uuid::new_v4();

    // A signature legitimately minted for one path cannot be reused to decrypt another: the
    // path is bound into the HMAC.
    let signature = sign(
        "test-workspace",
        &[job_id.to_string().as_bytes(), b"u/admin/legit_resource"],
    );
    let resp = post_open_modal(
        port,
        json!({
            "w_id": "test-workspace",
            "job_id": job_id.to_string(),
            "path": "u/admin/some_other_secret",
            "flow_step_id": "a",
            "signature": signature,
        }),
    )
    .await;
    assert_eq!(
        resp.status(),
        401,
        "a signature bound to a different path must not authorize decryption"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_open_modal_with_valid_signature_passes_the_gate(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let job_id = Uuid::new_v4();
    let path = "u/admin/nonexistent_resource";

    // A correctly signed payload passes the authorization gate and proceeds to resolve the
    // slack resource. The resource does not exist, so the handler returns a generic 400
    // ("Invalid Slack callback request") rather than 401 — proving the gate accepted the
    // signature (so the fix does not simply reject everything) without echoing the path.
    let signature = sign(
        "test-workspace",
        &[job_id.to_string().as_bytes(), path.as_bytes()],
    );
    let resp = post_open_modal(
        port,
        json!({
            "w_id": "test-workspace",
            "job_id": job_id.to_string(),
            "path": path,
            "flow_step_id": "a",
            "signature": signature,
        }),
    )
    .await;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "validly signed callback should pass the gate and 400 on the missing resource, got {status}: {body}"
    );
    assert!(
        !body.contains("nonexistent_resource"),
        "error must not echo the probed path: {body}"
    );

    Ok(())
}
