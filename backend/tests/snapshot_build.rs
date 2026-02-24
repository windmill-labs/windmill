/*!
 * Integration test for sandbox snapshot building.
 *
 * Requires:
 * - `crane` CLI in PATH (for Docker image export)
 * - A running PostgreSQL database (via DATABASE_URL or sqlx test infrastructure)
 *
 * Skips gracefully if `crane` is not available.
 */

#[cfg(feature = "parquet")]
mod tests {
    use sqlx::{Pool, Postgres};

    /// Check if `crane` CLI is available in PATH.
    fn crane_available() -> bool {
        std::process::Command::new("crane")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Build a minimal snapshot from busybox (no setup script).
    /// Verifies the full pipeline: crane export -> tar.gz -> upload to filesystem store -> DB status update.
    #[sqlx::test(fixtures("base"))]
    async fn test_build_snapshot_minimal(db: Pool<Postgres>) {
        if !crane_available() {
            eprintln!("SKIPPED: crane not found in PATH");
            return;
        }

        let w_id = "test-workspace";
        let store_dir = tempfile::tempdir().unwrap();

        // Configure filesystem-backed object store for the test workspace
        let lfs = serde_json::json!({
            "type": "FilesystemStorage",
            "root_path": store_dir.path().to_string_lossy(),
        });
        sqlx::query!(
            "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
            lfs,
            w_id,
        )
        .execute(&db)
        .await
        .unwrap();

        // Insert a pending snapshot row (mirrors what the API create_snapshot does)
        sqlx::query!(
            "INSERT INTO sandbox_snapshot \
             (workspace_id, name, tag, s3_key, docker_image, status, created_by) \
             VALUES ($1, $2, $3, '', $4, 'pending', 'test-user')",
            w_id,
            "integ-test",
            "latest",
            "busybox:latest",
        )
        .execute(&db)
        .await
        .unwrap();

        // Run the build
        windmill_sandbox::build_snapshot(w_id, "integ-test", "latest", "busybox:latest", None, &db)
            .await
            .unwrap();

        // Verify snapshot is ready with valid metadata
        let row = sqlx::query!(
            "SELECT status, s3_key, size_bytes, content_hash \
             FROM sandbox_snapshot \
             WHERE workspace_id = $1 AND name = $2 AND tag = $3",
            w_id,
            "integ-test",
            "latest",
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert_eq!(row.status, "ready", "snapshot status should be 'ready'");
        assert!(
            row.size_bytes.unwrap_or(0) > 0,
            "snapshot should have non-zero size"
        );
        assert!(
            !row.content_hash.is_empty(),
            "content_hash should be populated"
        );
        assert!(
            row.s3_key.starts_with("sandbox/snapshots/"),
            "s3_key should have correct prefix, got: {}",
            row.s3_key
        );

        // Verify the tar.gz file was written to the filesystem store
        let file_path = store_dir.path().join(&row.s3_key);
        assert!(
            file_path.exists(),
            "snapshot tar.gz should exist at {}",
            file_path.display()
        );

        // Verify the tar.gz is valid
        let bytes = std::fs::read(&file_path).unwrap();
        let dest = tempfile::tempdir().unwrap();
        windmill_sandbox::untar_gz(&bytes, dest.path()).unwrap();

        // busybox should have /bin/busybox
        assert!(
            dest.path().join("bin/busybox").exists(),
            "unpacked snapshot should contain /bin/busybox"
        );
    }

    /// Build a snapshot with a setup script that creates a file.
    /// Verifies the setup script ran inside the rootfs.
    #[sqlx::test(fixtures("base"))]
    async fn test_build_snapshot_with_setup_script(db: Pool<Postgres>) {
        if !crane_available() {
            eprintln!("SKIPPED: crane not found in PATH");
            return;
        }

        // nsjail is needed for setup scripts
        let nsjail_available = std::process::Command::new(
            std::env::var("NSJAIL_PATH").unwrap_or_else(|_| "nsjail".to_string()),
        )
        .arg("--help")
        .output()
        .is_ok();

        if !nsjail_available {
            eprintln!("SKIPPED: nsjail not found in PATH");
            return;
        }

        let w_id = "test-workspace";
        let store_dir = tempfile::tempdir().unwrap();

        let lfs = serde_json::json!({
            "type": "FilesystemStorage",
            "root_path": store_dir.path().to_string_lossy(),
        });
        sqlx::query!(
            "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
            lfs,
            w_id,
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query!(
            "INSERT INTO sandbox_snapshot \
             (workspace_id, name, tag, s3_key, docker_image, setup_script, status, created_by) \
             VALUES ($1, $2, $3, '', $4, $5, 'pending', 'test-user')",
            w_id,
            "setup-test",
            "latest",
            "busybox:latest",
            "echo 'hello from setup' > /tmp/setup_marker",
        )
        .execute(&db)
        .await
        .unwrap();

        windmill_sandbox::build_snapshot(
            w_id,
            "setup-test",
            "latest",
            "busybox:latest",
            Some("echo 'hello from setup' > /tmp/setup_marker"),
            &db,
        )
        .await
        .unwrap();

        // Verify status
        let row = sqlx::query!(
            "SELECT status, s3_key FROM sandbox_snapshot \
             WHERE workspace_id = $1 AND name = $2 AND tag = $3",
            w_id,
            "setup-test",
            "latest",
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(row.status, "ready");

        // Unpack and verify the setup script's marker file exists
        let bytes = std::fs::read(store_dir.path().join(&row.s3_key)).unwrap();
        let dest = tempfile::tempdir().unwrap();
        windmill_sandbox::untar_gz(&bytes, dest.path()).unwrap();

        let marker = dest.path().join("tmp/setup_marker");
        assert!(
            marker.exists(),
            "setup script marker file should exist in the snapshot"
        );
        let content = std::fs::read_to_string(&marker).unwrap();
        assert_eq!(content.trim(), "hello from setup");
    }

    /// Verify that build_snapshot correctly sets status to 'failed' and records
    /// the error when given an invalid docker image.
    #[sqlx::test(fixtures("base"))]
    async fn test_build_snapshot_invalid_image(db: Pool<Postgres>) {
        if !crane_available() {
            eprintln!("SKIPPED: crane not found in PATH");
            return;
        }

        let w_id = "test-workspace";
        let store_dir = tempfile::tempdir().unwrap();

        let lfs = serde_json::json!({
            "type": "FilesystemStorage",
            "root_path": store_dir.path().to_string_lossy(),
        });
        sqlx::query!(
            "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
            lfs,
            w_id,
        )
        .execute(&db)
        .await
        .unwrap();

        sqlx::query!(
            "INSERT INTO sandbox_snapshot \
             (workspace_id, name, tag, s3_key, docker_image, status, created_by) \
             VALUES ($1, $2, $3, '', $4, 'pending', 'test-user')",
            w_id,
            "bad-image",
            "latest",
            "nonexistent-registry.invalid/no-such-image:v999",
        )
        .execute(&db)
        .await
        .unwrap();

        let result = windmill_sandbox::build_snapshot(
            w_id,
            "bad-image",
            "latest",
            "nonexistent-registry.invalid/no-such-image:v999",
            None,
            &db,
        )
        .await;

        assert!(result.is_err(), "build should fail for invalid image");

        // Verify the DB status was set to 'failed' with an error message
        let row = sqlx::query!(
            "SELECT status, build_error FROM sandbox_snapshot \
             WHERE workspace_id = $1 AND name = $2 AND tag = $3",
            w_id,
            "bad-image",
            "latest",
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert_eq!(row.status, "failed");
        assert!(
            row.build_error.is_some(),
            "build_error should be populated on failure"
        );
    }
}
