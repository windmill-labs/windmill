//! A publisher-mode app run is allowed only while the caller can read the app,
//! and a grant or revocation applies to the very next run in the same process.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

#[sqlx::test(fixtures("base", "app_publisher_permit"))]
async fn test_publisher_app_read_access_is_rechecked(db: Pool<Postgres>) -> anyhow::Result<()> {
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
    let set_extra_perms = |perms: serde_json::Value| {
        sqlx::query("UPDATE app SET extra_perms = $1 WHERE id = 999101")
            .bind(perms)
            .execute(&db)
    };

    let (status, body) = run().await?;
    assert_eq!(status, 401, "unshared app must be refused: {body}");
    assert!(body.contains("Missing read permissions"), "{body}");

    set_extra_perms(json!({ "u/test-user-2": false })).await?;
    let (status, body) = run().await?;
    assert_eq!(status, 200, "a grant must apply to the next run: {body}");

    set_extra_perms(json!({})).await?;
    let (status, body) = run().await?;
    assert_eq!(
        status, 401,
        "a revocation must apply to the next run: {body}"
    );

    Ok(())
}
