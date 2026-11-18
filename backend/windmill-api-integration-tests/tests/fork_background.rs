use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

async fn wait_for_fork(client: &reqwest::Client, base_url: &str, creation_id: &str) -> Value {
    for _ in 0..100 {
        let status: Value = client
            .get(format!("{base_url}/fork_creation_status/{creation_id}"))
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
    panic!("fork creation {creation_id} still running");
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
    assert_eq!(resp.status(), 200);
    let ok_creation = resp.text().await?;
    let status = wait_for_fork(&client, &base_url, &ok_creation).await;
    assert_eq!(status, json!({ "status": "completed" }));
    // Nobody but the user who started a creation reads it.
    let resp = client
        .get(format!("{base_url}/fork_creation_status/{ok_creation}"))
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
    assert_eq!(resp.status(), 200);
    let err_creation = resp.text().await?;
    let resp = create("wm-fork-bg-err").await?;
    assert_eq!(resp.status(), 400, "a second creation while the first is in flight");
    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ('wm-fork-bg-err', 'x', 'x')")
        .execute(&mut *blocker)
        .await?;
    blocker.commit().await?;
    let status = wait_for_fork(&client, &base_url, &err_creation).await;
    assert_eq!(status["status"], "failed");
    assert!(
        status["error"].as_str().unwrap().contains("workspace_pkey"),
        "{status}"
    );

    // A retry of the same id is a new attempt: the failed one does not read its outcome.
    sqlx::query("DELETE FROM workspace WHERE id = 'wm-fork-bg-err'")
        .execute(&db)
        .await?;
    let resp = create("wm-fork-bg-err").await?;
    assert_eq!(resp.status(), 200);
    let retry_creation = resp.text().await?;
    assert_eq!(
        wait_for_fork(&client, &base_url, &retry_creation).await,
        json!({ "status": "completed" })
    );
    let resp = client
        .get(format!("{base_url}/fork_creation_status/{err_creation}"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .send()
        .await?;
    assert_eq!(resp.status(), 404);

    // An attempt whose server stopped heartbeating before its fork committed reads as failed.
    let abandoned = "5c3e1b1e-0000-4000-8000-000000000000";
    sqlx::query(
        "INSERT INTO workspace_fork_creation
             (fork_workspace_id, parent_workspace_id, created_by, creation_id, heartbeat_at)
         VALUES ('wm-fork-gone', 'test-workspace', 'test@windmill.dev', $1::uuid,
                 now() - interval '2 minutes')",
    )
    .bind(abandoned)
    .execute(&db)
    .await?;
    assert_eq!(
        wait_for_fork(&client, &base_url, abandoned).await["status"],
        "failed"
    );

    // Without the flag the fork is created before the answer, which clients asking for a
    // background fork recognise a server without it by.
    sqlx::query("UPDATE workspace SET deleted = true WHERE id = 'wm-fork-bg-err'")
        .execute(&db)
        .await?;
    let resp = client
        .post(format!("{base_url}/create_fork"))
        .header("Authorization", "Bearer SECRET_TOKEN")
        .json(&json!({ "id": "wm-fork-sync", "name": "wm-fork-sync" }))
        .send()
        .await?;
    assert_eq!(resp.text().await?, "Created forked workspace wm-fork-sync");

    Ok(())
}
