//! Another member's access lists paths the caller may not be able to read, so only a workspace
//! admin may ask for it.

use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_member_cannot_read_another_members_access(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let url = format!(
        "http://localhost:{}/api/w/test-workspace/access/get",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    let resp = client
        .get(&url)
        .query(&[("principal", "u/test-user")])
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 403, "{}", resp.text().await?);

    let resp = client
        .get(&url)
        .query(&[("principal", "u/test-user-2")])
        .header("Authorization", "Bearer SECRET_TOKEN_2")
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    Ok(())
}
