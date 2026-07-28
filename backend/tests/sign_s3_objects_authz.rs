//! Regression test for the `sign_s3_objects` permission bypass.
//!
//! Invariant: minting an S3 read signature (`apps/sign_s3_objects`) requires the
//! CALLER to hold `S3Permission::READ` on the key. The signature is a transferable
//! bearer capability (`validate_s3_signature` only checks HMAC + expiry), so a
//! caller must not be able to sign a key they cannot themselves read — otherwise
//! any workspace member could bypass the advanced S3 permission rules.
//!
//! Pinned against a FilesystemStorage LFS whose advanced permissions grant a
//! non-admin READ on `allowed/*` but nothing on `secret/*`:
//!   - the non-admin CAN sign `allowed/*` (authorized), and the minted signature
//!     validates end-to-end through the presigned s3_proxy fetch route;
//!   - the non-admin CANNOT sign `secret/*` (bypass closed);
//! Advanced S3 permissions are an enterprise feature, so this test requires the
//! `enterprise` + `private` + `parquet` features.
#![cfg(all(feature = "enterprise", feature = "private", feature = "parquet"))]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Configure the workspace LFS as a filesystem store rooted at `root_path`, with
/// an advanced permission rule granting READ on `allowed/*` to everyone the glob
/// matches (non-admins included). No rule covers `secret/*`, so it is denied.
async fn configure_lfs(db: &Pool<Postgres>, root_path: &str) -> anyhow::Result<()> {
    let lfs_config = json!({
        "type": "FilesystemStorage",
        "root_path": root_path,
        "public_resource": null,
        "advanced_permissions": [
            { "pattern": "allowed/*", "allow": "read" }
        ]
    });
    sqlx::query!(
        "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
        lfs_config,
        "test-workspace"
    )
    .execute(db)
    .await?;
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_sign_s3_objects_enforces_read_authz(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    let storage_dir = tempfile::tempdir()?;
    let storage_root = storage_dir.path().to_string_lossy().to_string();
    configure_lfs(&db, &storage_root).await?;

    // A real object so the signed fetch can stream bytes end-to-end.
    let allowed_dir = storage_dir.path().join("allowed");
    std::fs::create_dir_all(&allowed_dir)?;
    std::fs::write(allowed_dir.join("file.txt"), b"authorized payload")?;

    // ---- CORE REGRESSION: a non-admin (test-user-2) may NOT sign a key they have
    //      no READ permission on. Before the fix this returned a valid signature.
    let resp = authed(
        client().post(format!("{base}/apps/sign_s3_objects")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({ "s3_objects": [{ "s3": "secret/file.txt" }] }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert!(
        !status.is_success(),
        "non-admin must NOT be able to sign a key they cannot read (bypass): {status} {body}"
    );

    // ---- NO OVER-BLOCKING: the same non-admin CAN sign a key their advanced
    //      permissions allow them to read.
    let resp = authed(
        client().post(format!("{base}/apps/sign_s3_objects")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({ "s3_objects": [{ "s3": "allowed/file.txt" }] }))
    .send()
    .await?;
    let status = resp.status();
    let signed: serde_json::Value = resp.json().await?;
    assert!(
        status.is_success(),
        "non-admin must be able to sign a key they can read: {status} {signed}"
    );
    let presigned = signed[0]["presigned"]
        .as_str()
        .expect("authorized sign must return a presigned string")
        .to_string();

    // ---- END-TO-END: the minted signature is accepted by the fetch-side gate.
    //      Hit the presigned s3_proxy route (default storage) and confirm it
    //      streams the object rather than rejecting the signature.
    let fetch_url = format!("{base}/s3_proxy/_default_/allowed/file.txt?{presigned}");
    let resp = client().get(&fetch_url).send().await?;
    let status = resp.status();
    let body = resp.bytes().await?;
    assert!(
        status.is_success(),
        "signed fetch of an authorized key must succeed end-to-end: {status} {:?}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(
        body.as_ref(),
        b"authorized payload",
        "signed fetch must stream the authorized object's bytes"
    );

    Ok(())
}
