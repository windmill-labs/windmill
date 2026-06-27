//! Regression test for `remove_granular_acl` on scripts with multiple versions.
//!
//! The `script` table is keyed on `(workspace_id, hash)`, so several rows can
//! share the same `(workspace_id, path)` across versions. `add_granular_acl`
//! writes `extra_perms` to *every* row matching the path, so when ≥2 versions
//! carry the same permission key, the CTE in `remove_granular_acl` returns more
//! than one row. Before the fix (`LIMIT 1` on the `RETURNING` scalar subquery)
//! PostgreSQL rejected this with "more than one row returned by a subquery used
//! as an expression" and the request 500'd.

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn new_script(path: &str, content: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "",
        "description": "",
        "content": content,
        "language": "deno",
        "schema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

#[sqlx::test(fixtures("base"))]
async fn test_remove_granular_acl_on_multi_version_script(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    let path = "u/test-user/multi_version_acl";
    let acl_owner = "u/other-user";

    // 1. Create version 1 of the script.
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&new_script(
        path,
        "export async function main() { return 1; }",
    ))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create v1: {}", resp.text().await?);
    let v1_hash: String = resp.text().await?;

    // 2. Create version 2 at the same path. This archives v1 but keeps both
    //    rows sharing `(workspace_id, path)`.
    let mut v2 = new_script(path, "export async function main() { return 2; }");
    v2["parent_hash"] = json!(v1_hash);
    let resp = authed(
        client().post(format!("{base}/scripts/create")),
        "SECRET_TOKEN",
    )
    .json(&v2)
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "create v2: {}", resp.text().await?);

    // Sanity: two rows now share the same `(workspace_id, path)`.
    let version_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM script WHERE path = $1 AND workspace_id = $2")
            .bind(path)
            .bind("test-workspace")
            .fetch_one(&db)
            .await?;
    assert_eq!(version_count, 2, "expected two script versions at the path");

    // 3. Grant a granular ACL. `add_granular_acl` updates *all* rows matching
    //    the path, so the permission key lands in every version's extra_perms.
    let resp = authed(
        client().post(format!("{base}/acls/add/script/{path}")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "owner": acl_owner, "write": true }))
    .send()
    .await?;
    assert_eq!(resp.status(), 200, "add acl: {}", resp.text().await?);

    // Both versions carry the permission key — this is what makes the CTE in
    // `remove_granular_acl` return more than one row.
    let rows_with_perm: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM script WHERE path = $1 AND workspace_id = $2 AND extra_perms ? $3",
    )
    .bind(path)
    .bind("test-workspace")
    .bind(acl_owner)
    .fetch_one(&db)
    .await?;
    assert_eq!(rows_with_perm, 2, "both versions should carry the acl key");

    // 4. Remove the granular ACL. Before the fix this 500'd with "more than one
    //    row returned by a subquery used as an expression".
    let resp = authed(
        client().post(format!("{base}/acls/remove/script/{path}")),
        "SECRET_TOKEN",
    )
    .json(&json!({ "owner": acl_owner }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 200,
        "remove acl on a multi-version script should succeed, got {status}: {body}"
    );

    // 5. The permission key is gone from every version.
    let rows_with_perm: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM script WHERE path = $1 AND workspace_id = $2 AND extra_perms ? $3",
    )
    .bind(path)
    .bind("test-workspace")
    .bind(acl_owner)
    .fetch_one(&db)
    .await?;
    assert_eq!(
        rows_with_perm, 0,
        "the acl key should be removed from all versions"
    );

    Ok(())
}
