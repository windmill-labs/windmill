use sqlx::{Pool, Postgres};
use windmill_common::jobs::{resolve_delete_after_secs, schedule_job_deletion};
use windmill_test_utils::*;

// ---------------------------------------------------------------------------
// Unit tests for resolve_delete_after_secs
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_no_deletion() {
    assert_eq!(resolve_delete_after_secs(None, None), None);
    assert_eq!(resolve_delete_after_secs(Some(false), None), None);
}

#[test]
fn test_resolve_immediate_backward_compat() {
    // delete_after_use=true with no secs → immediate (0)
    assert_eq!(resolve_delete_after_secs(Some(true), None), Some(0));
}

#[test]
fn test_resolve_explicit_secs() {
    assert_eq!(resolve_delete_after_secs(None, Some(0)), Some(0));
    assert_eq!(resolve_delete_after_secs(None, Some(3600)), Some(3600));
    assert_eq!(resolve_delete_after_secs(Some(true), Some(60)), Some(60));
    assert_eq!(resolve_delete_after_secs(Some(false), Some(120)), Some(120));
}

#[test]
fn test_resolve_rejects_negative() {
    assert_eq!(resolve_delete_after_secs(None, Some(-1)), None);
    assert_eq!(resolve_delete_after_secs(Some(true), Some(-100)), None);
}

// ---------------------------------------------------------------------------
// Integration: schedule_job_deletion inserts into job_delete_schedule
// ---------------------------------------------------------------------------

#[sqlx::test(fixtures("base"))]
async fn test_schedule_job_deletion_inserts_row(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = uuid::Uuid::new_v4();

    schedule_job_deletion(&db, job_id, "test-workspace", 3600).await?;

    let row = sqlx::query_as::<_, (uuid::Uuid, String)>(
        "SELECT job_id, workspace_id FROM job_delete_schedule WHERE job_id = $1",
    )
    .bind(job_id)
    .fetch_one(&db)
    .await?;

    assert_eq!(row.0, job_id);
    assert_eq!(row.1, "test-workspace");

    // Verify delete_at is approximately now + 3600s
    let delete_at: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT delete_at FROM job_delete_schedule WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&db)
            .await?;

    let expected_min = chrono::Utc::now() + chrono::Duration::seconds(3500);
    let expected_max = chrono::Utc::now() + chrono::Duration::seconds(3700);
    assert!(
        delete_at > expected_min && delete_at < expected_max,
        "delete_at should be ~1 hour from now, got {delete_at}"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn test_schedule_job_deletion_is_idempotent(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let job_id = uuid::Uuid::new_v4();

    schedule_job_deletion(&db, job_id, "test-workspace", 60).await?;
    // Second call should not error (ON CONFLICT DO NOTHING)
    schedule_job_deletion(&db, job_id, "test-workspace", 120).await?;

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM job_delete_schedule WHERE job_id = $1")
            .bind(job_id)
            .fetch_one(&db)
            .await?;
    assert_eq!(count, 1, "should have exactly one row (idempotent)");

    Ok(())
}

// ---------------------------------------------------------------------------
// Integration: a flow's own delete_after_secs = 0
// ---------------------------------------------------------------------------

#[cfg(feature = "enterprise")]
const FLOW_PATH: &str = "u/test-user/delete_after_use_flow";
#[cfg(feature = "enterprise")]
const FLOW_VERSION: i64 = 8_812_004;

/// A sequential loop: its iterations are stored with a NULL root_job, so only a walk down
/// parent_job reaches them.
#[cfg(feature = "enterprise")]
async fn deploy_loop_flow(db: &Pool<Postgres>) -> anyhow::Result<()> {
    let value = serde_json::json!({
        "delete_after_secs": 0,
        "modules": [{
            "id": "loop",
            "value": {
                "type": "forloopflow",
                "iterator": { "type": "static", "value": [1, 2] },
                "skip_failures": false,
                "parallel": false,
                "modules": [{ "id": "inner", "value": { "type": "identity" } }]
            }
        }]
    });
    sqlx::query(
        "INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at,
            schema, extra_perms, versions)
        VALUES ('test-workspace', $1, '', '', $2, 'test-user', now(), '{}', '{}', ARRAY[$3])",
    )
    .bind(FLOW_PATH)
    .bind(&value)
    .bind(FLOW_VERSION)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by)
        VALUES ($1, 'test-workspace', $2, $3, '{}', 'test-user')",
    )
    .bind(FLOW_VERSION)
    .bind(FLOW_PATH)
    .bind(&value)
    .execute(db)
    .await?;
    Ok(())
}

/// `(flow_step_id, args, result)` of every job of the run, the flow job included as "flow".
#[cfg(feature = "enterprise")]
async fn run_data(
    db: &Pool<Postgres>,
    flow_id: uuid::Uuid,
) -> anyhow::Result<Vec<(String, serde_json::Value, serde_json::Value)>> {
    Ok(sqlx::query_as(
        "WITH RECURSIVE run AS (
            SELECT id FROM v2_job WHERE id = $1
            UNION ALL
            SELECT j.id FROM v2_job j JOIN run r ON j.parent_job = r.id
        )
        SELECT COALESCE(j.flow_step_id, 'flow'), j.args, c.result
        FROM run JOIN v2_job j USING (id) JOIN v2_job_completed c USING (id)",
    )
    .bind(flow_id)
    .fetch_all(db)
    .await?)
}

#[cfg(feature = "enterprise")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_level_delete_clears_steps_and_keeps_flow_result(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    use windmill_common::jobs::JobPayload;

    initialize_tracing().await;
    deploy_loop_flow(&db).await?;
    let server = ApiServer::start(db.clone()).await?;
    let flow_job = RunJob::from(JobPayload::Flow {
        path: FLOW_PATH.to_string(),
        dedicated_worker: None,
        apply_preprocessor: false,
        version: FLOW_VERSION,
        labels: None,
        job_token_scopes: None,
    })
    .arg("x", serde_json::json!("secret"))
    .run_until_complete(&db, false, server.addr.port())
    .await;

    let jobs = run_data(&db, flow_job.id).await?;
    assert!(jobs.len() > 1, "the flow should have run steps: {jobs:?}");
    for (step, args, result) in &jobs {
        assert_eq!(args, &serde_json::json!({}), "{step} kept its args");
        if step == "flow" {
            // An async run has no caller to read the result, so the flow keeps it.
            assert_ne!(
                result,
                &serde_json::json!({}),
                "the flow's result was cleared"
            );
        } else {
            assert_eq!(result, &serde_json::json!({}), "{step} kept its result");
        }
    }
    Ok(())
}

#[cfg(feature = "enterprise")]
#[sqlx::test(fixtures("base"))]
async fn test_flow_level_delete_clears_flow_result_after_sync_read(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    deploy_loop_flow(&db).await?;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let response = in_test_worker(
        windmill_common::worker::Connection::Sql(db.clone()),
        reqwest::Client::new()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/jobs/run_wait_result/f/{FLOW_PATH}"
            ))
            .bearer_auth("SECRET_TOKEN")
            .json(&serde_json::json!({ "x": "secret" }))
            .send(),
        port,
    )
    .await?
    .error_for_status()?
    .json::<serde_json::Value>()
    .await?;

    assert_eq!(response.as_array().map(Vec::len), Some(2), "{response}");
    let flow_id: uuid::Uuid =
        sqlx::query_scalar("SELECT id FROM v2_job WHERE runnable_path = $1 AND parent_job IS NULL")
            .bind(FLOW_PATH)
            .fetch_one(&db)
            .await?;
    let kept: Vec<_> = run_data(&db, flow_id)
        .await?
        .into_iter()
        .filter(|(_, args, result)| {
            args != &serde_json::json!({}) || result != &serde_json::json!({})
        })
        .collect();
    assert!(kept.is_empty(), "jobs that kept args or result: {kept:?}");
    Ok(())
}
