//! Regression test: the object-storage connectivity test's SSRF / local-filesystem hardening
//! applies to every non-super-admin caller, not only on Cloud.
//!
//! `POST /api/settings/test_object_storage_config` -> `test_s3_bucket` runs the probe on the API
//! server itself and reflects the upstream response into its error, so an authenticated user could
//! otherwise reach internal endpoints, port-scan the server's network, or write to its local disk
//! through the Filesystem backend. `CLOUD_HOSTED` is unset in this harness, so the test exercises
//! a self-hosted deployment and pins:
//!   - a non-super-admin is rejected for a loopback S3 endpoint without any connection being made,
//!     and for a Filesystem backend, and
//!   - a super admin keeps the unrestricted path (a Filesystem backend round-trips successfully).
//!
//! Requires the `parquet` feature — the route is gated on it.
//! `base` fixture: test@windmill.dev (super admin, SECRET_TOKEN); test2@windmill.dev (SECRET_TOKEN_2).
#![cfg(feature = "parquet")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use windmill_test_utils::*;

const SUPER_ADMIN_TOKEN: &str = "SECRET_TOKEN";
const USER_TOKEN: &str = "SECRET_TOKEN_2";

async fn test_object_storage(
    url: &str,
    token: &str,
    body: serde_json::Value,
) -> anyhow::Result<(u16, String)> {
    let resp = reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await?;
    Ok((resp.status().as_u16(), resp.text().await?))
}

#[sqlx::test(fixtures("base"))]
async fn object_storage_test_is_restricted_for_non_super_admins_off_cloud(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let url = format!(
        "http://localhost:{}/api/settings/test_object_storage_config",
        server.addr.port()
    );

    // A loopback "S3 endpoint" standing in for an internal service: the probe must be rejected
    // before the server opens a connection to it.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let internal_port = listener.local_addr()?.port();
    let connected = Arc::new(AtomicBool::new(false));
    tokio::spawn({
        let connected = connected.clone();
        async move {
            while listener.accept().await.is_ok() {
                connected.store(true, Ordering::SeqCst);
            }
        }
    });
    let internal_s3 = json!({
        "type": "S3",
        "bucket": "bucket",
        "region": "us-east-1",
        "access_key": "key",
        "secret_key": "secret",
        "endpoint": format!("http://127.0.0.1:{internal_port}"),
        "allow_http": true,
        "path_style": true,
    });
    let (status, body) = test_object_storage(&url, USER_TOKEN, internal_s3).await?;
    assert_eq!(
        status, 401,
        "non-super-admin must be rejected for a loopback endpoint (got {status}): {body}"
    );
    assert!(
        body.contains("requires a super admin"),
        "unexpected rejection: {body}"
    );
    assert!(
        !connected.load(Ordering::SeqCst),
        "the server must not connect to the rejected endpoint"
    );

    let tmp = tempfile::tempdir()?;
    let filesystem = json!({ "type": "Filesystem", "root_path": tmp.path().to_str().unwrap() });
    let (status, body) = test_object_storage(&url, USER_TOKEN, filesystem.clone()).await?;
    assert_eq!(
        status, 401,
        "non-super-admin must be rejected for a Filesystem backend (got {status}): {body}"
    );
    assert!(
        body.contains("requires a super admin"),
        "unexpected rejection: {body}"
    );

    // Super admins keep the unrestricted path.
    let (status, body) = test_object_storage(&url, SUPER_ADMIN_TOKEN, filesystem).await?;
    assert_eq!(
        status, 200,
        "super admin must be able to test a Filesystem backend (got {status}): {body}"
    );
    Ok(())
}
