//! Shared AI session artifacts: one link per author and artifact, readable by any workspace
//! member until its retention window passes, and removable only by its author or an admin.
//!
//! Expiry is enforced on read as well as by the monitor's sweep, so a share past its window must
//! not be served in the gap before the sweep reaches it.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN: &str = "Bearer SECRET_TOKEN";
const MEMBER: &str = "Bearer SECRET_TOKEN_2";

async fn share(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    content: &str,
) -> anyhow::Result<Value> {
    let resp = client
        .post(format!("{base}/share"))
        .header("Authorization", token)
        .json(&json!({
            "artifact_id": "plan:session-1",
            "name": "Plan",
            "kind": "md",
            "version": 1,
            "content": content,
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);
    Ok(resp.json().await?)
}

#[sqlx::test(fixtures("base", "ai_shared_artifacts"))]
async fn shared_artifact_is_served_to_members_until_it_expires(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace/ai/shared_artifacts",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    let first = share(&client, &base, MEMBER, "draft").await?;
    let second = share(&client, &base, MEMBER, "final").await?;
    assert_eq!(first["id"], second["id"], "re-sharing minted a second link");
    let id = second["id"].as_str().unwrap();

    let resp = client
        .get(format!("{base}/get/{id}"))
        .header("Authorization", ADMIN)
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await?;
    assert_eq!(body["content"], "final");

    // The handlers read through the raw pool, so the workspace in the URL is the only thing
    // scoping a share: a member of another workspace must not reach it by id through theirs.
    let resp = client
        .get(format!(
            "http://localhost:{}/api/w/test-workspace-2/ai/shared_artifacts/get/{id}",
            server.addr.port()
        ))
        .header("Authorization", ADMIN)
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        404,
        "a share was served through another workspace's path"
    );

    sqlx::query(
        "UPDATE ai_shared_artifact SET shared_at = now() - ($1::bigint + 60) * interval '1 second'",
    )
    .bind(windmill_common::ai_shared_artifact_retention_secs())
    .execute(&db)
    .await?;

    let resp = client
        .get(format!("{base}/get/{id}"))
        .header("Authorization", ADMIN)
        .send()
        .await?;
    assert_eq!(resp.status(), 404, "an expired share was served");

    let status: Value = client
        .get(format!("{base}/status?artifact_id=plan:session-1"))
        .header("Authorization", MEMBER)
        .send()
        .await?
        .json()
        .await?;
    assert!(
        status.get("share").is_none(),
        "an expired share was reported live: {status}"
    );

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn only_the_author_or_an_admin_can_unshare(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace/ai/shared_artifacts",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    let shared = share(&client, &base, ADMIN, "admin's plan").await?;
    let id = shared["id"].as_str().unwrap();

    let resp = client
        .delete(format!("{base}/delete/{id}"))
        .header("Authorization", MEMBER)
        .send()
        .await?;
    assert_eq!(resp.status(), 404);
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM ai_shared_artifact")
        .fetch_one(&db)
        .await?;
    assert_eq!(remaining, 1, "a member deleted someone else's share");

    let member_share = share(&client, &base, MEMBER, "member's plan").await?;
    let resp = client
        .delete(format!(
            "{base}/delete/{}",
            member_share["id"].as_str().unwrap()
        ))
        .header("Authorization", ADMIN)
        .send()
        .await?;
    assert_eq!(resp.status(), 200, "{}", resp.text().await?);

    Ok(())
}

/// The id is compared against a `VARCHAR(255)` column on every route that takes one, and a
/// NUL in it would otherwise reach Postgres and come back as a 500.
#[sqlx::test(fixtures("base"))]
async fn a_malformed_artifact_id_is_refused_on_every_route(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace/ai/shared_artifacts",
        server.addr.port()
    );
    let client = reqwest::Client::new();

    for bad_id in ["", "a\0b", &"x".repeat(256)] {
        let resp = client
            .get(format!("{base}/status"))
            .query(&[("artifact_id", bad_id)])
            .header("Authorization", MEMBER)
            .send()
            .await?;
        assert_eq!(resp.status(), 400, "status accepted {bad_id:?}");

        let resp = client
            .post(format!("{base}/share"))
            .header("Authorization", MEMBER)
            .json(&json!({
                "artifact_id": bad_id,
                "name": "Plan",
                "kind": "md",
                "version": 1,
                "content": "x",
            }))
            .send()
            .await?;
        assert_eq!(resp.status(), 400, "share accepted {bad_id:?}");
    }

    Ok(())
}
