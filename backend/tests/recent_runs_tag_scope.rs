//! `jobs/recent_runs` aggregates over the workspace through `user_db`, which applies RLS but not a
//! token's `if_jobs:filter_tags` scope: the handler has to filter on it itself.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

// From the `jobs_read_auth` fixture, both runs of test-user-2: one tagged `deno`, one `flow`.
const DENO_JOB: &str = "14141414-1414-1414-1414-141414141414";
const FLOW_JOB: &str = "15151515-1515-1515-1515-151515151515";

async fn recent_runs(port: u16, token: &str) -> String {
    let resp = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/jobs/recent_runs"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("request");
    assert!(resp.status().is_success(), "{}", resp.status());
    resp.text().await.expect("body")
}

#[sqlx::test(fixtures("base", "jobs_read_auth"))]
async fn test_recent_runs_honors_tag_scope(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let unscoped = recent_runs(port, "SECRET_TOKEN_2").await;
    assert!(unscoped.contains(DENO_JOB), "{unscoped}");
    assert!(unscoped.contains(FLOW_JOB), "{unscoped}");

    // SCOPED_DENO_TOKEN is test-user-2 restricted to `if_jobs:filter_tags:deno`.
    let scoped = recent_runs(port, "SCOPED_DENO_TOKEN").await;
    assert!(scoped.contains(DENO_JOB), "{scoped}");
    assert!(!scoped.contains(FLOW_JOB), "{scoped}");
    Ok(())
}
