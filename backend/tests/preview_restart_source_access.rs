//! A flow preview restarted from an earlier run copies that run's step results, so it must be
//! refused unless the caller can see every run the request names.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_common::jobs::JobPayload;
use windmill_test_utils::*;

fn identity_flow() -> serde_json::Value {
    json!({ "modules": [
        { "id": "a", "value": { "type": "identity" } },
        { "id": "b", "value": { "type": "identity" } },
    ]})
}

async fn completed_run(db: &Pool<Postgres>, port: u16, username: &str, email: &str) -> String {
    RunJob::from(JobPayload::RawFlow {
        value: serde_json::from_value(identity_flow()).unwrap(),
        path: None,
        restarted_from: None,
    })
    .as_user(username, email)
    .run_until_complete(db, false, port)
    .await
    .id
    .to_string()
}

#[sqlx::test(fixtures("base"))]
async fn test_preview_restart_requires_source_read_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let admin_run = completed_run(&db, port, "test-user", "test@windmill.dev").await;
    let own_run = completed_run(&db, port, "test-user-2", "test2@windmill.dev").await;

    let restart = |restarted_from: serde_json::Value| {
        reqwest::Client::new()
            .post(format!(
                "http://localhost:{port}/api/w/test-workspace/jobs/run/preview_flow"
            ))
            .bearer_auth("SECRET_TOKEN_2")
            .json(
                &json!({ "args": {}, "value": identity_flow(), "restarted_from": restarted_from }),
            )
            .send()
    };

    let resp = restart(json!({ "flow_job_id": admin_run, "step_id": "b" })).await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);

    let resp = restart(json!({
        "flow_job_id": own_run, "step_id": "b",
        "nested": { "flow_job_id": admin_run, "step_id": "b" },
    }))
    .await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);

    let resp = restart(json!({ "flow_job_id": own_run, "step_id": "b" })).await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);

    // A run hidden on its own but inside a flow the caller can read is readable, as on the run page.
    let child_run = completed_run(&db, port, "test-user", "test@windmill.dev").await;
    sqlx::query("UPDATE v2_job SET parent_job = $1::uuid WHERE id = $2::uuid")
        .bind(&own_run)
        .bind(&child_run)
        .execute(&db)
        .await?;
    let resp = restart(json!({
        "flow_job_id": own_run, "step_id": "b",
        "nested": { "flow_job_id": child_run, "step_id": "b" },
    }))
    .await?;
    assert_eq!(resp.status(), 201, "{}", resp.text().await?);
    Ok(())
}
