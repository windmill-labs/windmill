use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

async fn wait_for_fork(client: &reqwest::Client, base_url: &str, fork_id: &str) -> Value {
    for _ in 0..100 {
        let status: Value = client
            .get(format!("{base_url}/fork_creation_status/{fork_id}"))
            .header("Authorization", "Bearer SECRET_TOKEN")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if status["status"] != "running" {
            return status;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("fork {fork_id} still running");
}

/// A fork created in the background is reported by `fork_creation_status`, including a failure
/// that happens after the request has already returned.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_fork_created_in_background(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base_url = format!(
        "http://localhost:{}/api/w/test-workspace/workspaces",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let create = |id: &str| {
        client
            .post(format!("{base_url}/create_fork?background=true"))
            .header("Authorization", "Bearer SECRET_TOKEN")
            .json(&json!({ "id": id, "name": id }))
            .send()
    };

    let resp = create("wm-fork-bg-ok").await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    let status = wait_for_fork(&client, &base_url, "wm-fork-bg-ok").await;
    assert_eq!(status, json!({ "status": "completed" }));
    // A member who neither started the fork nor administers the workspace cannot read its outcome.
    let resp = client
        .get(format!("{base_url}/fork_creation_status/wm-fork-bg-ok"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 404);
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workspace WHERE id = 'wm-fork-bg-ok')")
            .fetch_one(&db)
            .await?;
    assert!(exists);
    // Under CE's workspace cap.
    sqlx::query("UPDATE workspace SET deleted = true WHERE id = 'wm-fork-bg-ok'")
        .execute(&db)
        .await?;

    // Hold the fork's write lock so the request is validated and returns, then commit a workspace
    // with the same id: the background write fails on it.
    let mut blocker = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('fork:wm-fork-bg-err'))")
        .execute(&mut *blocker)
        .await?;
    let resp = create("wm-fork-bg-err").await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    // A second request is refused while the first is in flight, with the wording the AI-session
    // fork matches to wait for its own earlier request.
    let resp = create("wm-fork-bg-err").await?;
    assert_eq!(resp.status(), 400);
    assert!(resp.text().await?.contains("is already being created"));
    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ('wm-fork-bg-err', 'x', 'x')")
        .execute(&mut *blocker)
        .await?;
    blocker.commit().await?;
    let status = wait_for_fork(&client, &base_url, "wm-fork-bg-err").await;
    assert_eq!(status["status"], "failed");
    assert!(
        status["error"].as_str().unwrap().contains("workspace_pkey"),
        "{status}"
    );

    Ok(())
}
