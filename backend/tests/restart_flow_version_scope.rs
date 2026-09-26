//! A restart's `flow_version` is caller-supplied while the restarted job keeps the original
//! job's path, so only versions of that same flow may be restarted on.

use sqlx::{Pool, Postgres};
use windmill_common::cache;
use windmill_common::error::Error;
use windmill_common::flow_status::FlowStatus;
use windmill_common::jobs::JobPayload;
use windmill_queue::PushIsolationLevel;

const WS: &str = "test-workspace";

async fn push_restart(
    db: &Pool<Postgres>,
    completed_job_id: uuid::Uuid,
    flow_version: i64,
) -> Result<uuid::Uuid, Error> {
    let args = std::collections::HashMap::new();
    let (uuid, tx) = windmill_queue::push(
        db,
        PushIsolationLevel::IsolatedRoot(db.clone()),
        WS,
        JobPayload::RestartedFlow {
            completed_job_id,
            step_id: "a".into(),
            branch_or_iteration_n: None,
            flow_version: Some(flow_version),
            branch_chosen: None,
            nested: None,
        },
        windmill_queue::PushArgs::from(&args),
        "test-user",
        "test@windmill.dev",
        "u/test-user".to_string(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;
    Ok(uuid)
}

#[sqlx::test(fixtures("base", "hello"))]
async fn test_restart_rejects_flow_version_of_another_path(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    // A completed run of f/system/hello_flow (version 1443253234253453).
    let version = 1443253234253453;
    let flow = cache::flow::fetch_version(&db, version).await?;
    let completed_job_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id, created_by,
                             permissioned_as, permissioned_as_email, tag)
         VALUES ($1, $2, 'flow', 'f/system/hello_flow', $3, 'test-user', 'u/test-user',
                 'test@windmill.dev', 'flow')",
    )
    .bind(completed_job_id)
    .bind(WS)
    .bind(version)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, flow_status)
         VALUES ($1, $2, 0, 'success', $3)",
    )
    .bind(completed_job_id)
    .bind(WS)
    .bind(serde_json::to_value(FlowStatus::new(flow.value()))?)
    .execute(&db)
    .await?;

    // f/system/hello_with_nodes_flow's version, in the same workspace.
    let err = push_restart(&db, completed_job_id, 1443253234253454)
        .await
        .expect_err("a version of another flow must be rejected");
    assert!(matches!(err, Error::BadRequest(_)), "{err:?}");

    // Another version of the restarted flow itself stays allowed.
    let same_path_version: i64 = sqlx::query_scalar(
        "INSERT INTO flow_version (workspace_id, path, schema, value, created_by)
         SELECT workspace_id, path, schema, value, created_by FROM flow_version WHERE id = $1
         RETURNING id",
    )
    .bind(version)
    .fetch_one(&db)
    .await?;
    push_restart(&db, completed_job_id, same_path_version).await?;
    Ok(())
}
