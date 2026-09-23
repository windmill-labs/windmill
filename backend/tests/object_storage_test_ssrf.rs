//! `POST /api/settings/test_object_storage_config` runs the probe on the API server and reflects the
//! upstream response, so every non-super-admin must be rejected for private/loopback endpoints and
//! the Filesystem backend on every deployment (`CLOUD_HOSTED` is unset here), while a super admin's
//! Filesystem probe still round-trips. Requires the `parquet` feature, like the route.
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
