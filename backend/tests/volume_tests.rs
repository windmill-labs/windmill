use sqlx::{Pool, Postgres};
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
        "SELECT workspace_id, name, size_bytes, created_by, last_read_at, last_write_at
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
    assert!(row.last_read_at.is_none());
    assert!(row.last_write_at.is_none());

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_upsert_size(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by, last_write_at)
         VALUES ($1, $2, $3, $4, now())
         ON CONFLICT (workspace_id, name) DO UPDATE
         SET size_bytes = $3, last_write_at = now()",
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
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by, last_write_at)
         VALUES ($1, $2, $3, $4, now())
         ON CONFLICT (workspace_id, name) DO UPDATE
         SET size_bytes = $3, last_write_at = now()",
        "test-workspace",
        "upsert-vol",
        2048_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT size_bytes, last_write_at FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "upsert-vol"
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(row.size_bytes, 2048);
    assert!(row.last_write_at.is_some());

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_update_last_read(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by)
         VALUES ($1, $2, $3, $4)",
        "test-workspace",
        "read-vol",
        100_i64,
        "test-user"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT last_read_at FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "read-vol"
    )
    .fetch_one(&db)
    .await?;
    assert!(row.last_read_at.is_none());

    sqlx::query!(
        "UPDATE volume SET last_read_at = now() WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "read-vol"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT last_read_at FROM volume WHERE workspace_id = $1 AND name = $2",
        "test-workspace",
        "read-vol"
    )
    .fetch_one(&db)
    .await?;
    assert!(row.last_read_at.is_some());

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_volume_update_nonexistent_noop(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let result = sqlx::query!(
        "UPDATE volume SET last_read_at = now() WHERE workspace_id = $1 AND name = $2",
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
async fn test_volume_extra_perms(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let perms_val = serde_json::json!({"u/test-user": true, "g/all": false});
    sqlx::query!(
        "INSERT INTO volume (workspace_id, name, size_bytes, created_by, extra_perms)
         VALUES ($1, $2, $3, $4, $5)",
        "test-workspace",
        "perms-vol",
        100_i64,
        "test-user",
        perms_val
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

    let perms: serde_json::Value = row.extra_perms;
    assert_eq!(perms["u/test-user"], serde_json::json!(true));
    assert_eq!(perms["g/all"], serde_json::json!(false));

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
async fn test_volume_settings_column(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let settings = serde_json::json!({
        "admin_only_creation": true,
        "max_volume_size_bytes": 1073741824_i64
    });

    sqlx::query!(
        "UPDATE workspace_settings SET volume_settings = $1::jsonb WHERE workspace_id = $2",
        settings,
        "test-workspace"
    )
    .execute(&db)
    .await?;

    let row = sqlx::query!(
        "SELECT volume_settings FROM workspace_settings WHERE workspace_id = $1",
        "test-workspace"
    )
    .fetch_one(&db)
    .await?;

    let settings = row.volume_settings.unwrap();
    assert_eq!(settings["admin_only_creation"], serde_json::json!(true));
    assert_eq!(
        settings["max_volume_size_bytes"],
        serde_json::json!(1073741824_i64)
    );

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

#[test]
fn test_volume_nsjail_mount_oss() {
    use std::path::Path;
    use windmill_worker_volumes::volume_nsjail_mount;

    let result = volume_nsjail_mount(Path::new("/tmp/volumes/data"), "/mnt/data");
    assert_eq!(result, "");
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
