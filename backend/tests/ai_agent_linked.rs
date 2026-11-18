use serde_json::json;
use sqlx::{Pool, Postgres};
use std::time::Duration;
use windmill_common::{flows::FlowValue, jobs::JobPayload};
use windmill_test_utils::{
    completed_job, in_test_worker, listen_for_completed_jobs, ApiServer, RunJob, StreamFind,
};

/// Only writes of an agent are checked for operators, so a linked step must refuse a path whose
/// resource is no longer an agent rather than run the `tools` it now holds.
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn linked_agent_step_refuses_a_resource_of_another_type(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type)
         VALUES ('test-workspace', 'u/test-user/agent', $1, 'object')",
    )
    .bind(
        json!({"tools": [{"id": "t", "value": {"tool_type": "flowmodule",
        "type": "rawscript", "language": "bun", "input_transforms": {},
        "content": "export async function main() { return 1 }"}}]}),
    )
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let flow: FlowValue = serde_json::from_value(json!({"modules": [{"id": "agent", "value": {
        "type": "aiagent", "tools": [], "agent": "$res:u/test-user/agent",
        "input_transforms": {"user_message": {"type": "static", "value": "hi"}}
    }}]}))?;
    let id = RunJob::from(JobPayload::RawFlow {
        value: flow,
        path: Some("u/test-user/linked".into()),
        restarted_from: None,
    })
    .push(&db)
    .await;
    let notifications = listen_for_completed_jobs(&db).await;
    tokio::time::timeout(
        Duration::from_secs(45),
        in_test_worker(&db, notifications.find(&id), server.addr.port()),
    )
    .await?;
    let job = completed_job(id, &db).await;
    server.close().await?;
    assert!(!job.success);
    assert!(
        format!("{:?}", job.result).contains("not an AI agent"),
        "{:?}",
        job.result
    );
    Ok(())
}
