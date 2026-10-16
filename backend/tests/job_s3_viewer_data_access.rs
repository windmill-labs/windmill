//! A job whose `job_perms.viewer_data_access` is set reads workspace object storage as its viewer:
//! the advanced permission rules are checked against the viewer, not the job's own identity, and
//! such a job with no viewer is refused rather than checked as itself.
//!
//! Advanced S3 permissions are an enterprise feature.
#![cfg(all(feature = "enterprise", feature = "private", feature = "parquet"))]

use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

/// A job of `test-user-3`, run for `viewer`, reading data as that viewer or not.
async fn job_token(
    db: &Pool<Postgres>,
    viewer: Option<&str>,
    viewer_data_access: bool,
) -> anyhow::Result<String> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, permissioned_as_email,
            kind, script_lang, runnable_path, tag)
        VALUES ($1, 'test-workspace', 'test-user-3', 'u/test-user-3', 'test3@windmill.dev',
            'script', 'deno', 'u/test-user-3/app_runnable', 'deno')",
    )
    .bind(id)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO job_perms (job_id, email, username, is_admin, is_operator, folders, groups,
            workspace_id, end_user_email, viewer_data_access)
        VALUES ($1, 'test3@windmill.dev', 'test-user-3', false, false, '{}', '{}',
            'test-workspace', $2, $3)",
    )
    .bind(id)
    .bind(viewer)
    .bind(viewer_data_access)
    .execute(db)
    .await?;
    Ok(windmill_common::auth::create_token_for_owner(
        db,
        "test-workspace",
        "u/test-user-3",
        "ephemeral-script",
        300,
        "test3@windmill.dev",
        &id,
        None,
        None,
    )
    .await?)
}

#[sqlx::test(fixtures("base"))]
async fn a_job_reading_data_as_its_viewer_gets_the_viewers_storage_paths(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let storage_dir = tempfile::tempdir()?;
    for user in ["test-user-2", "test-user-3"] {
        let dir = storage_dir.path().join("u").join(user);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("file.txt"), user)?;
    }
    sqlx::query("UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2")
        .bind(json!({
            "type": "FilesystemStorage",
            "root_path": storage_dir.path().to_string_lossy(),
            "public_resource": null,
            "advanced_permissions": [{ "pattern": "u/{username}/**", "allow": "read" }]
        }))
        .bind("test-workspace")
        .execute(&db)
        .await?;

    let read = |token: String, user: &'static str| async move {
        let resp = reqwest::Client::new()
            .get(format!(
                "http://localhost:{port}/api/w/test-workspace/job_helpers/download_s3_file"
            ))
            .query(&[("file_key", format!("u/{user}/file.txt"))])
            .bearer_auth(token)
            .send()
            .await?;
        Ok::<_, anyhow::Error>(resp.status().is_success())
    };

    let own = job_token(&db, Some("test2@windmill.dev"), false).await?;
    assert!(read(own.clone(), "test-user-3").await?);
    assert!(
        !read(own, "test-user-2").await?,
        "a job not reading as its viewer reached the viewer's paths"
    );

    let as_viewer = job_token(&db, Some("test2@windmill.dev"), true).await?;
    assert!(read(as_viewer.clone(), "test-user-2").await?);
    assert!(
        !read(as_viewer, "test-user-3").await?,
        "a job reading as its viewer kept its own identity's paths"
    );

    let no_viewer = job_token(&db, None, true).await?;
    assert!(
        !read(no_viewer, "test-user-3").await?,
        "a job reading as a viewer it does not have was checked as itself"
    );
    Ok(())
}
