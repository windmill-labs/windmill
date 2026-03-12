use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::jobs::{JobPayload, RawCode};
use windmill_common::scripts::ScriptLang;
use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_volume_insert(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
         VALUES ($1, $2, $3, $4)",
        "test-workspace",
        "test-volume",
        1024_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT workspace_id, name, size_bytes, created_by, last_used_at
         FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "test-volume"
    )
    .fetch_one(&db)
    .await?;

    assert_eq!(row.workspace_id, "test-workspace");
    assert_eq!(row.name, "test-volume");
    assert_eq!(row.size_bytes, 1024);
    assert_eq!(row.created_by, "test-user");
    assert!(row.last_used_at.is_none());

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_upsert_size(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by, last_used_at)
         VALUES ($1, $2, $3, $4, now())
         ON CONFLICT (workspace_id, name) DO UPDATE
         SET size_bytes = $3, last_used_at = now()",
        "test-workspace",
        "upsert-vol",
        500_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT size_bytes FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "upsert-vol"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(row.size_bytes, 500);

    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by, last_used_at)
         VALUES ($1, $2, $3, $4, now())
         ON CONFLICT (workspace_id, name) DO UPDATE
         SET size_bytes = $3, last_used_at = now()",
        "test-workspace",
        "upsert-vol",
        2048_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT size_bytes, last_used_at FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "upsert-vol"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(row.size_bytes, 2048);
    assert!(row.last_used_at.is_some());

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_update_last_used(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
         VALUES ($1, $2, $3, $4)",
        "test-workspace",
        "used-vol",
        100_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT last_used_at FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "used-vol"
    )
    .fetch_one(&db)
    .await?;
    assert!(row.last_used_at.is_none());

    sqlx::query!(
        "UPDATE volume SET last_used_at = now() WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "used-vol"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT last_used_at FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "used-vol"
    )
    .fetch_one(&db)
    .await?;
    assert!(row.last_used_at.is_some());

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_update_nonexistent_noop(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let result = sqlx::query!(
        "UPDATE volume SET last_used_at = now() WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "nonexistent-vol"
    )
    .execute(&db)
    .await?;

    assert_eq!(result.rows_affected(), 0);

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_list_multiple(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    for i in 0..5 {
        sqlx::query!(
            "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
             VALUES ($1, $2, $3, $4)",
            "test-workspace",
            format!("vol-{}", i),
            (i * 100) as i64,
            "test-user"
        )
        .execute(&db)
        .await?;
    }

    let rows = sqlx::query!(
        "SELECT name, size_bytes FROM volume WHERE workspace_id = $1 ORDER BY name",
        "test-workspace"
    )
    .fetch_all(&db)
    .await?;

    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].name, "vol-0");
    assert_eq!(rows[0].size_bytes, 0);
    assert_eq!(rows[4].name, "vol-4");
    assert_eq!(rows[4].size_bytes, 400);

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_delete(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
         VALUES ($1, $2, $3, $4)",
        "test-workspace",
        "deleteme",
        100_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "deleteme"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(count, Some(1));

    sqlx::query!(
        "DELETE FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "deleteme"
    )
    .execute(&db)
    .await?;

    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "deleteme"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(count, Some(0));

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_workspace_fk_constraint(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let result = sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
         VALUES ($1, $2, $3, $4)",
        "nonexistent-workspace",
        "vol",
        100_i64,
        "test-user"
    )
    .execute(&db)
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("foreign key"),
        "Expected foreign key violation, got: {}",
        err
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_primary_key_uniqueness(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
         VALUES ($1, $2, $3, $4)",
        "test-workspace",
        "unique-vol",
        100_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    let result = sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
         VALUES ($1, $2, $3, $4)",
        "test-workspace",
        "unique-vol",
        200_i64,
        "another-user"
    )
    .execute(&db)
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("duplicate key") || err.contains("unique"),
        "Expected unique violation, got: {}",
        err
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_extra_perms(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    // Insert volume with default (empty) extra_perms
    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
         VALUES ($1, $2, $3, $4)",
        "test-workspace",
        "perms-vol",
        100_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    // Default extra_perms should be empty object
    let row = sqlx::query!(
        "SELECT extra_perms FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "perms-vol"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(row.extra_perms, serde_json::json!({}));

    // Set extra_perms via jsonb_set (same pattern as granular_acls.rs)
    sqlx::query!(
        "UPDATE volume SET extra_perms = jsonb_set(extra_perms, $1, to_jsonb($2::bool), true)
         WHERE workspace_id = $3 AND name = $4",
        &vec!["u/alice".to_string()],
        true,
        "test-workspace",
        "perms-vol"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT extra_perms FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "perms-vol"
    )
    .fetch_one(&db)
    .await?;
    let perms = row.extra_perms.as_object().unwrap();
    assert_eq!(perms.get("u/alice").and_then(|v| v.as_bool()), Some(true));

    // Remove a permission entry
    sqlx::query!(
        "UPDATE volume SET extra_perms = extra_perms - $1
         WHERE workspace_id = $2 AND name = $3",
        "u/alice",
        "test-workspace",
        "perms-vol"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT extra_perms FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "perms-vol"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(row.extra_perms, serde_json::json!({}));

    Ok(())
}

#[test]
fn test_parse_volume_annotations_python() {
    use windmill_worker_volumes::parse_volume_annotations;

    let content = r#"# sandbox
# volume: training-data /tmp/training
# volume: models /opt/models

def main():
    pass
"#;
    let volumes = parse_volume_annotations(content, "#");
    assert_eq!(volumes.len(), 2);
    assert_eq!(volumes[0].name, "training-data");
    assert_eq!(volumes[0].target, "/tmp/training");
    assert_eq!(volumes[1].name, "models");
    assert_eq!(volumes[1].target, "/opt/models");
}

#[test]
fn test_parse_volume_annotations_typescript() {
    use windmill_worker_volumes::parse_volume_annotations;

    let content = r#"// sandbox
// volume: datasets /tmp/datasets

export async function main() {
    return "hello";
}
"#;
    let volumes = parse_volume_annotations(content, "//");
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes[0].name, "datasets");
    assert_eq!(volumes[0].target, "/tmp/datasets");
}

#[test]
fn test_parse_volume_annotations_no_prefix_match() {
    use windmill_worker_volumes::parse_volume_annotations;

    let content = "def main():\n    pass";
    let volumes = parse_volume_annotations(content, "#");
    assert!(volumes.is_empty());
}

#[test]
fn test_parse_volume_annotations_empty_script() {
    use windmill_worker_volumes::parse_volume_annotations;

    let volumes = parse_volume_annotations("", "#");
    assert!(volumes.is_empty());
}

#[test]
fn test_sandbox_annotation_python() {
    use windmill_common::worker::PythonAnnotations;

    let content = "# sandbox\n# volume: data /tmp/data\ndef main():\n    pass";
    let annotations = PythonAnnotations::parse(content);
    assert!(annotations.sandbox);
}

#[test]
fn test_sandbox_annotation_typescript() {
    use windmill_common::worker::TypeScriptAnnotations;

    let content = "// sandbox\n// volume: data /tmp/data\nexport function main() {}";
    let annotations = TypeScriptAnnotations::parse(content);
    assert!(annotations.sandbox);
}

#[test]
fn test_volume_comment_prefix_selection() {
    use windmill_common::scripts::ScriptLang;

    let get_prefix = |lang: &ScriptLang| -> &str {
        match lang {
            ScriptLang::Python3
            | ScriptLang::Bash
            | ScriptLang::Powershell
            | ScriptLang::Ansible
            | ScriptLang::Ruby => "#",
            ScriptLang::Deno
            | ScriptLang::Bun
            | ScriptLang::Bunnative
            | ScriptLang::Nativets
            | ScriptLang::Go => "//",
            _ => "",
        }
    };

    assert_eq!(get_prefix(&ScriptLang::Python3), "#");
    assert_eq!(get_prefix(&ScriptLang::Bash), "#");
    assert_eq!(get_prefix(&ScriptLang::Powershell), "#");
    assert_eq!(get_prefix(&ScriptLang::Ansible), "#");
    assert_eq!(get_prefix(&ScriptLang::Ruby), "#");
    assert_eq!(get_prefix(&ScriptLang::Deno), "//");
    assert_eq!(get_prefix(&ScriptLang::Bun), "//");
    assert_eq!(get_prefix(&ScriptLang::Bunnative), "//");
    assert_eq!(get_prefix(&ScriptLang::Nativets), "//");
    assert_eq!(get_prefix(&ScriptLang::Go), "//");
}

#[test]
fn test_volume_mount_struct() {
    use windmill_worker_volumes::VolumeMount;

    let mount = VolumeMount { name: "test-vol".to_string(), target: "/mnt/data".to_string() };
    assert_eq!(mount.name, "test-vol");
    assert_eq!(mount.target, "/mnt/data");
}

#[test]
fn test_parse_volume_relative_path() {
    use windmill_worker_volumes::parse_volume_annotations;

    let content = "// volume: agent-memory .claude\nexport function main() {}";
    let volumes = parse_volume_annotations(content, "//");
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes[0].name, "agent-memory");
    assert_eq!(volumes[0].target, ".claude");
}

#[test]
fn test_parse_volume_relative_nested_path() {
    use windmill_worker_volumes::parse_volume_annotations;

    let content = "# volume: data data/models\ndef main():\n    pass";
    let volumes = parse_volume_annotations(content, "#");
    assert_eq!(volumes.len(), 1);
    assert_eq!(volumes[0].name, "data");
    assert_eq!(volumes[0].target, "data/models");
}

#[cfg(feature = "private")]
#[test]
fn test_volume_nsjail_mount() {
    use std::path::Path;
    use windmill_worker_volumes::volume_nsjail_mount;

    let result = volume_nsjail_mount(Path::new("/tmp/volumes/data"), "/mnt/data");
    assert!(result.contains("src: \"/tmp/volumes/data\""));
    assert!(result.contains("dst: \"/mnt/data\""));
    assert!(result.contains("is_bind: true"));
    assert!(result.contains("rw: true"));
}

#[test]
fn test_sync_stats_default() {
    use windmill_worker_volumes::SyncStats;

    let stats = SyncStats { new_size_bytes: 0, file_count: 0, uploaded: 0, skipped: 0 };
    assert_eq!(stats.new_size_bytes, 0);
    assert_eq!(stats.file_count, 0);
    assert_eq!(stats.uploaded, 0);
    assert_eq!(stats.skipped, 0);
}

#[test]
fn test_asset_kind_volume_variant() {
    use windmill_types::assets::AssetKind;

    let kind = AssetKind::Volume;
    let serialized = serde_json::to_string(&kind).unwrap();
    assert_eq!(serialized, "\"volume\"");

    let deserialized: AssetKind = serde_json::from_str("\"volume\"").unwrap();
    assert!(matches!(deserialized, AssetKind::Volume));
}

/// E2E test: run a bun script with volume mount through a SQL-connected worker.
/// Pre-populates the volume in filesystem storage, verifies the script can read
/// files and write new ones, then checks sync-back to storage and DB state.
#[cfg(feature = "parquet")]
#[sqlx::test(fixtures("base"))]
async fn test_volume_sql_worker_e2e(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // 1. Set up filesystem-based object storage in a temp dir
    let storage_dir = tempfile::tempdir()?;
    let storage_root = storage_dir.path().to_string_lossy().to_string();

    let lfs_config = json!({
        "type": "FilesystemStorage",
        "root_path": storage_root,
        "public_resource": null,
        "advanced_permissions": null,
        "volume_storage": "primary"
    });

    sqlx::query!(
        "UPDATE workspace_settings SET large_file_storage = $1 WHERE workspace_id = $2",
        lfs_config,
        "test-workspace"
    )
    .execute(&db)
    .await?;

    // 2. Pre-populate the volume with a file (workspace-namespaced path)
    let vol_dir = storage_dir
        .path()
        .join("volumes")
        .join("test-workspace")
        .join("test-vol");
    std::fs::create_dir_all(&vol_dir)?;
    std::fs::write(vol_dir.join("hello.txt"), b"hello from volume")?;

    // 3. Push the job and run with SQL-connected worker
    let code = r#"// volume: test-vol /tmp/data

import { readFileSync, writeFileSync, existsSync } from "fs";

export function main() {
    const content = readFileSync("/tmp/data/hello.txt", "utf-8");
    writeFileSync("/tmp/data/output.txt", "written by sql worker");
    return {
        read_content: content,
        output_exists: existsSync("/tmp/data/output.txt"),
    };
}"#;

    let job = JobPayload::Code(RawCode {
        hash: None,
        content: code.to_string(),
        path: None,
        language: ScriptLang::Bun,
        lock: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
    });

    let result = run_job_in_new_worker_until_complete(&db, false, job, port).await;

    assert!(result.success, "job should succeed: {:?}", result.result);
    let json = result.json_result().expect("should have JSON result");
    assert_eq!(json["read_content"], json!("hello from volume"));
    assert_eq!(json["output_exists"], json!(true));

    // 4. Verify volume DB row was updated
    let vol_row = sqlx::query!(
        "SELECT size_bytes, file_count, leased_by, lease_until
         FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "test-vol"
    )
    .fetch_optional(&db)
    .await?;

    let vol_row = vol_row.expect("volume row should exist");
    assert!(
        vol_row.file_count >= 2,
        "should have at least 2 files (hello.txt + output.txt), got: {}",
        vol_row.file_count
    );
    assert!(vol_row.size_bytes > 0, "size_bytes should be > 0");
    assert!(vol_row.leased_by.is_none(), "lease should be released");

    // 5. Verify the new file was written back to storage
    let output_path = vol_dir.join("output.txt");
    assert!(
        output_path.exists(),
        "output.txt should be synced back to storage"
    );
    let output_content = std::fs::read_to_string(&output_path)?;
    assert_eq!(output_content, "written by sql worker");

    Ok(())
}
