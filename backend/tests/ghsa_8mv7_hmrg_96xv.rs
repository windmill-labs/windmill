//! Regression test for GHSA-8mv7-hmrg-96xv.
//!
//! The version-keyed flow run routes (`/jobs/run/fv/{version}`,
//! `/jobs/run_wait_result/fv/{version}` GET+POST, and the streaming variant)
//! resolved `flow_version.id -> path` on the raw DB pool and then only ran
//! `check_scopes` against the resolved path. `check_scopes` gates the API
//! token's scopes, not the caller's folder-level RBAC, so a low-privilege
//! workspace member with a normal (unscoped) token could execute an
//! admin-owned, folder-scoped flow they had zero permission on — while the
//! path-keyed sibling `/jobs/run/f/{path}` correctly rejected them via the
//! RLS-aware flow lookup.
//!
//! The fix resolves the version's path through an RLS-aware (`user_db`) query,
//! so the folder ACL is enforced identically on both route families. This test
//! pins down:
//!   - the folder owner (non-admin) can still run the flow by version (201),
//!   - a member with no folder grant is rejected by version (401), and
//!   - that rejection now matches the path-keyed sibling (401).

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

#[sqlx::test(fixtures("base", "ghsa_8mv7_hmrg_96xv"))]
async fn test_fv_version_route_enforces_folder_acl(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace");

    // base.sql tokens:
    //   SECRET_TOKEN_2 -> test-user-2 (non-admin, owns folder `secret`)
    //   SECRET_TOKEN_3 -> test-user-3 (non-admin, NO grant on folder `secret`)
    let owner_token = "SECRET_TOKEN_2";
    let member_token = "SECRET_TOKEN_3";
    let version = 9000001_i64;
    let flow_path = "f/secret/admin_flow";

    // 1. The folder owner (a plain non-admin member, authorized via the folder
    //    ACL) can run the flow by version id. This must not regress.
    let resp = authed(
        client().post(format!("{base}/jobs/run/fv/{version}")),
        owner_token,
    )
    .json(&json!({}))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        201,
        "folder owner must still be able to run the flow by version: {}",
        resp.text().await?
    );

    // 2. A member with no folder grant must be rejected on the path-keyed
    //    route (the always-correct control sibling).
    let resp = authed(
        client().post(format!("{base}/jobs/run/f/{flow_path}")),
        member_token,
    )
    .json(&json!({}))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "control: member must be rejected by the path-keyed route"
    );

    // 3. The vulnerability: the same member must now ALSO be rejected on the
    //    version-keyed route, instead of bypassing the folder ACL.
    let resp = authed(
        client().post(format!("{base}/jobs/run/fv/{version}")),
        member_token,
    )
    .json(&json!({}))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 401,
        "member must NOT be able to run a folder-scoped flow by version id: {body}"
    );
    // The caller supplied only an opaque version id; the rejection must not
    // disclose the resolved flow path back to them.
    assert!(
        !body.contains(flow_path),
        "rejection must not leak the resolved flow path: {body}"
    );

    // 4. Same gate on the run_wait_result version routes (POST and GET).
    let resp = authed(
        client().post(format!("{base}/jobs/run_wait_result/fv/{version}")),
        member_token,
    )
    .json(&json!({}))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "member must be rejected on run_wait_result/fv POST"
    );

    let resp = authed(
        client().get(format!("{base}/jobs/run_wait_result/fv/{version}")),
        member_token,
    )
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        401,
        "member must be rejected on run_wait_result/fv GET"
    );

    Ok(())
}
