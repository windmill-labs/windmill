//! `DELETE /w/{workspace}/capture/{id}` must stay inside the workspace in the URL.
//!
//! Capture ids come from one instance-wide sequence and the capture RLS policies
//! key on the path segment only, never on `workspace_id` — so an id alone is not
//! an authorization boundary. A member of one workspace can name any id and, if
//! the row's path happens to sit inside their grants (`u/<their username>/…`,
//! `g/<their group>/…`, a same-named folder), reach a row belonging to a
//! workspace they are not a member of.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

async fn capture_workspace(db: &Pool<Postgres>, id: i64) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT workspace_id FROM capture WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
        .unwrap()
}

#[sqlx::test(fixtures("base", "capture_cross_workspace"))]
async fn delete_capture_is_confined_to_the_url_workspace(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    // Capture 1 lives in test-workspace-2, which this member has no access to.
    let resp = client
        .delete(format!("{base}/capture/1"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .send()
        .await?;
    let status = resp.status();
    assert_eq!(
        capture_workspace(&db, 1).await.as_deref(),
        Some("test-workspace-2"),
        "capture of another workspace was deleted (status {status})"
    );

    // The member's own capture in test-workspace still deletes.
    let resp = client
        .delete(format!("{base}/capture/2"))
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    assert_eq!(capture_workspace(&db, 2).await, None);

    Ok(())
}
