//! Regression test for resolving a workspace file storage by name: `_default_` resolves to the
//! primary storage (see `get_large_file_storage`), and a name with no storage behind it is
//! reported with the name rather than as a workspace with no storage at all.
//!
//! Pinned against a FilesystemStorage LFS so the test needs no object store.
#![cfg(all(feature = "private", feature = "parquet"))]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

/// Configure the workspace's primary storage as a filesystem store, with no secondary
/// storage at all — so `_default_` can only work by naming the primary one.
async fn configure_primary_lfs(db: &Pool<Postgres>, root_path: &str) -> anyhow::Result<()> {
    let lfs_config = json!({
        "type": "FilesystemStorage",
        "root_path": root_path,
        "public_resource": null,
        "advanced_permissions": null
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
async fn test_workspace_storage_resolves_by_name(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );

    let storage_dir = tempfile::tempdir()?;
    configure_primary_lfs(&db, &storage_dir.path().to_string_lossy()).await?;
    std::fs::write(storage_dir.path().join("file.txt"), b"primary payload")?;

    let resp = authed(
        client().get(format!(
            "{base}/job_helpers/download_s3_file?file_key=file.txt&storage=_default_"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    let status = resp.status();
    let body = resp.bytes().await?;
    assert!(
        status.is_success(),
        "`_default_` must resolve to the primary storage: {status} {}",
        String::from_utf8_lossy(&body)
    );
    assert_eq!(body.as_ref(), b"primary payload");

    let resp = authed(
        client().get(format!(
            "{base}/job_helpers/download_s3_file?file_key=file.txt&storage=nope"
        )),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert!(
        body.contains("nope"),
        "the error for an unknown storage must name it: {body}"
    );
    // Must stay a 400: the message echoes back a caller-supplied name, so one like
    // `archive not found` reaches the asset previewer's "not found" substring test, and only
    // the 400 stops it rendering as the "object not written yet" empty state instead
    // (`S3FilePreview.svelte`, `isNotFoundError`).
    assert_eq!(status, 400, "got {status}: {body}");

    Ok(())
}

/// `_default_` resolving to the primary storage is only safe because no secondary storage can
/// carry that name: one that did would be shadowed, silently moving the workspace's reads and
/// writes to another bucket. The reservation is what makes the resolution above sound.
#[sqlx::test(fixtures("base"))]
async fn test_default_is_reserved_as_a_secondary_storage_name(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let storage_dir = tempfile::tempdir()?;
    let root = storage_dir.path().to_string_lossy().to_string();

    let secondary = |name: &str| {
        json!({
            "large_file_storage": {
                "type": "FilesystemStorage",
                "root_path": root,
                "advanced_permissions": null,
                "secondary_storage": {
                    name: {
                        "type": "FilesystemStorage",
                        "root_path": root,
                        "advanced_permissions": null
                    }
                }
            }
        })
    };

    let resp = authed(
        client().post(format!("{base}/workspaces/edit_large_file_storage_config")),
        "SECRET_TOKEN",
    )
    .json(&secondary("_default_"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "`_default_` must be refused as a secondary storage name: {}",
        resp.text().await?
    );

    // Any other name still saves, so the check is not blanket-rejecting secondary storages.
    let resp = authed(
        client().post(format!("{base}/workspaces/edit_large_file_storage_config")),
        "SECRET_TOKEN",
    )
    .json(&secondary("archive"))
    .send()
    .await?;
    assert!(resp.status().is_success(), "{}", resp.text().await?);

    Ok(())
}
