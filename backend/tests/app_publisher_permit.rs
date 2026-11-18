//! A refused publisher-mode app run is not remembered: once the caller is granted
//! read access to the app, the very next run in the same process is allowed.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

#[sqlx::test(fixtures("base", "app_publisher_permit"))]
async fn test_publisher_app_refusal_is_not_cached(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let url = format!(
        "http://localhost:{port}/api/w/test-workspace/apps_u/execute_component/u/test-user/papp"
    );

    let run = || async {
        let resp = reqwest::Client::new()
            .post(&url)
            .header("Authorization", "Bearer SECRET_TOKEN_2")
            .json(&json!({ "component": "c", "path": "script/u/test-user/hello", "args": {} }))
            .send()
            .await?;
        let status = resp.status().as_u16();
        anyhow::Ok((status, resp.text().await?))
    };

    let (status, body) = run().await?;
    assert_eq!(status, 401, "unshared app must be refused: {body}");
    assert!(body.contains("Missing read permissions"), "{body}");

    sqlx::query("UPDATE app SET extra_perms = '{\"u/test-user-2\": false}' WHERE id = 999101")
        .execute(&db)
        .await?;
    let (status, body) = run().await?;
    assert_eq!(status, 200, "a grant must apply to the next run: {body}");

    Ok(())
}
