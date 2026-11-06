use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::{ApiServer, RunJob};

async fn run_step(
    db: &Pool<Postgres>,
    port: u16,
    kind: &str,
    path: &str,
) -> anyhow::Result<(bool, String, Vec<String>)> {
    run_step_value(
        db,
        port,
        json!({"type": kind, "path": path, "input_transforms": {}}),
    )
    .await
}

async fn run_step_value(
    db: &Pool<Postgres>,
    port: u16,
    step: serde_json::Value,
) -> anyhow::Result<(bool, String, Vec<String>)> {
    let value: FlowValue =
        serde_json::from_value(json!({"modules": [{"id": "a", "value": step}]}))?;
    let job = RunJob::from(JobPayload::RawFlow { value, path: None, restarted_from: None })
        .run_until_complete(db, false, port)
        .await;
    let children: Vec<String> = sqlx::query_scalar(
        "SELECT runnable_path FROM v2_job WHERE parent_job = $1 AND kind IN ('script', 'flow')",
    )
    .bind(job.id)
    .fetch_all(db)
    .await?;
    Ok((
        job.success,
        job.json_result().unwrap().to_string(),
        children,
    ))
}

/// A stored step path reaches the worker unvalidated from a raw preview or an older flow, and must
/// mean what the run routes make of it.
#[sqlx::test(fixtures("base"))]
async fn flow_steps_resolve_their_parsed_path(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO script (workspace_id, created_by, content, schema, summary, description, path, hash, language, lock)
         VALUES ('test-workspace', 'test-user', 'echo ran', '{}', '', '', 'f/system/step', 2762, 'bash', '')",
    )
    .execute(&db)
    .await?;
    let sub_flow = json!({"modules": [{"id": "s", "value": {"type": "identity"}}]}).to_string();
    sqlx::query(
        "INSERT INTO flow (workspace_id, summary, description, path, versions, schema, value, edited_by)
         VALUES ('test-workspace', '', '', 'f/system/sub', '{}', '{}', $1::text::jsonb, 'test-user')",
    )
    .bind(&sub_flow)
    .execute(&db)
    .await?;
    sqlx::query(
        "WITH v AS (
            INSERT INTO flow_version (workspace_id, path, value, schema, created_by)
            VALUES ('test-workspace', 'f/system/sub', $1::text::jsonb, '{}', 'test-user') RETURNING id
        )
        UPDATE flow SET versions = ARRAY[(SELECT id FROM v)]
        WHERE workspace_id = 'test-workspace' AND path = 'f/system/sub'",
    )
    .bind(&sub_flow)
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    for (kind, path, canonical) in [
        ("script", "/f/system/step", "f/system/step"),
        ("flow", "/f/system/sub", "f/system/sub"),
    ] {
        let (success, result, children) = run_step(&db, port, kind, path).await?;
        assert!(success, "{path}: {result}");
        assert_eq!(children, [canonical], "{path}");

        let twice = format!("/{path}");
        let (success, result, _) = run_step(&db, port, kind, &twice).await?;
        assert!(!success, "{twice}: {result}");
        assert!(
            result.contains(&format!("invalid {kind} path")),
            "{twice}: {result}"
        );
    }

    // A pinned step runs its hash whatever the path beside it says.
    for path in ["", "//f/system/step", "hub/abc/x"] {
        let step = json!({
            "type": "script", "path": path, "hash": format!("{:016x}", 2762), "input_transforms": {}
        });
        let (success, result, children) = run_step_value(&db, port, step).await?;
        assert!(success, "{path}: {result}");
        assert_eq!(children, [path], "{path}");
    }
    Ok(())
}
