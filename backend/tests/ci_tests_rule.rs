//! `RequirePassingCiTests`: an item a parent's CI tests cover, and each of those tests, reaches
//! the parent only as a promotion from a fork where they passed, and a promotion writes the
//! fork's copy rather than the request's.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const TOKEN: &str = "FDR_RANDOM_TOKEN";
const TARGET: &str = "u/fdr-random/target";
const TEST: &str = "u/fdr-random/target_test";

async fn deploy(port: u16, ws: &str, path: &str, content: &str, query: &str) -> (u16, String) {
    let base = format!("http://localhost:{port}/api/w/{ws}/scripts");
    let http = reqwest::Client::new();
    let parent_hash = http
        .get(format!("{base}/get/p/{path}"))
        .bearer_auth(TOKEN)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .ok()
        .and_then(|s| s.get("hash").cloned());
    let resp = http
        .post(format!("{base}/create{query}"))
        .bearer_auth(TOKEN)
        .json(
            &json!({"path": path, "summary": "", "description": "", "content": content,
            "language": "bash", "parent_hash": parent_hash}),
        )
        .send()
        .await
        .unwrap();
    (resp.status().as_u16(), resp.text().await.unwrap())
}

#[sqlx::test(fixtures("fork_deployment_requests"))]
async fn test_require_passing_ci_tests(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    sqlx::query(
        "INSERT INTO workspace_protection_rule (workspace_id, name, rules, bypass_groups, \
         bypass_users) VALUES ('parent-ws', 'ci', 128, '{}', '{}')",
    )
    .execute(&db)
    .await?;
    // The same test in both, deployed before it covers anything.
    for ws in ["parent-ws", "fork-ws"] {
        assert_eq!(deploy(port, ws, TEST, "exit 0", "").await.0, 201);
    }
    sqlx::query(
        "INSERT INTO ci_test_reference (workspace_id, test_script_path, test_script_hash, \
         tested_item_path, tested_item_kind) VALUES ('parent-ws', $1, 0, $2, 'script')",
    )
    .bind(TEST)
    .bind(TARGET)
    .execute(&db)
    .await?;
    assert_eq!(
        deploy(port, "fork-ws", TARGET, "echo tested", "").await.0,
        201
    );

    let (status, body) = deploy(port, "parent-ws", TARGET, "echo tested", "").await;
    assert_eq!(status, 403, "{body}");
    let promote = "?promote_from=fork-ws";
    let (status, body) = deploy(port, "parent-ws", TARGET, "echo forged", promote).await;
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("has not run"), "{body}");

    sqlx::query(
        "WITH j AS ( \
           INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id, created_by, \
                               permissioned_as, trigger_kind, trigger) \
           SELECT gen_random_uuid(), 'fork-ws', 'script', $1, hash, 'fdr-random', \
                  'u/fdr-random', 'ci_test', 'script/' || $2 \
           FROM script WHERE workspace_id = 'fork-ws' AND path = $1 \
           ORDER BY created_at DESC LIMIT 1 RETURNING id) \
         INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, started_at) \
         SELECT id, 'fork-ws', 1, 'success', now() FROM j",
    )
    .bind(TEST)
    .bind(TARGET)
    .execute(&db)
    .await?;
    let (status, body) = deploy(port, "parent-ws", TARGET, "echo forged", promote).await;
    assert_eq!(status, 201, "{body}");
    let deployed: String = sqlx::query_scalar(
        "SELECT content FROM script WHERE workspace_id = 'parent-ws' AND path = $1 \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(TARGET)
    .fetch_one(&db)
    .await?;
    assert_eq!(deployed, "echo tested");

    // Rewriting the test in place would switch the rule off.
    let (status, body) = deploy(port, "parent-ws", TEST, "exit 0 # weakened", "").await;
    assert_eq!(status, 403, "{body}");
    Ok(())
}
