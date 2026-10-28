//! The error returned to a caller who cannot read an existing variable or resource is built
//! from a raw-pool read, so it must not carry the item's or its folder's `extra_perms`: those
//! maps tell any workspace member who holds access to something they cannot see.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const MARKER: &str = "acl_holder_marker";

#[sqlx::test(fixtures("base"))]
async fn denied_read_does_not_disclose_extra_perms(db: Pool<Postgres>) -> anyhow::Result<()> {
    let perms = serde_json::json!({ format!("u/{MARKER}"): true, format!("g/{MARKER}"): false });
    sqlx::query(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms, created_by)
         VALUES ('test-workspace', 'private', 'private', '{u/test-user}', $1, 'test-user')",
    )
    .bind(&perms)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO variable (workspace_id, path, value, is_secret, description, extra_perms)
         VALUES ('test-workspace', 'f/private/v', 'x', false, '', $1)",
    )
    .bind(&perms)
    .execute(&db)
    .await?;
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by)
         VALUES ('test-workspace', 'f/private/r', '{}', 'any', $1, 'test-user')",
    )
    .bind(&perms)
    .execute(&db)
    .await?;

    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    for url in [
        format!("{ws}/variables/get/f/private/v"),
        format!("{ws}/resources/get/f/private/r"),
    ] {
        let resp = client
            .get(&url)
            .bearer_auth("SECRET_TOKEN_3")
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        assert!(
            !status.is_success() && body.contains("don't have access"),
            "{url}: {status} {body}"
        );
        assert!(
            !body.contains(MARKER),
            "{url} discloses extra_perms: {body}"
        );
    }
    Ok(())
}
