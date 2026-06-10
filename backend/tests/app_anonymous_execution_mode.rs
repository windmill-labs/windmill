//! Regression test for the admin gate on anonymous app execution mode.
//!
//! Any user with write access to an app could set its policy
//! `execution_mode` to `anonymous` (no login required), exposing the app
//! publicly without authentication or admin oversight. The fix mirrors the
//! `custom_path` admin gate: only workspace admins can create an app as
//! anonymous or flip an existing app to anonymous. To avoid breaking
//! existing workflows, non-admins can still redeploy an app that is
//! already anonymous (no exposure change) and can downgrade it back to
//! `publisher` (exposure reduction).
//!
//! Users from the `base` fixture:
//!   test-user   (admin,    token SECRET_TOKEN)
//!   test-user-2 (non-admin, token SECRET_TOKEN_2)

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const USER_TOKEN: &str = "SECRET_TOKEN_2";

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {}", token))
}

fn app_payload(path: &str, execution_mode: &str) -> serde_json::Value {
    json!({
        "path": path,
        "summary": "Test app",
        "value": {},
        "policy": { "execution_mode": execution_mode, "triggerables": {} }
    })
}

fn policy_update(execution_mode: &str) -> serde_json::Value {
    json!({
        "policy": { "execution_mode": execution_mode, "triggerables": {} }
    })
}

#[sqlx::test(fixtures("base"))]
async fn test_anonymous_execution_mode_requires_admin(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let app_path = "u/test-user-2/test_app";

    // 1. A non-admin cannot create an app with anonymous execution mode.
    let resp = authed(client().post(format!("{ws}/apps/create")), USER_TOKEN)
        .json(&app_payload(app_path, "anonymous"))
        .send()
        .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 403,
        "non-admin creating an anonymous app must be rejected: {body}"
    );
    assert!(
        body.contains("Admin"),
        "error should mention the admin requirement, got: {body}"
    );

    // 2. The same create with publisher mode succeeds.
    let resp = authed(client().post(format!("{ws}/apps/create")), USER_TOKEN)
        .json(&app_payload(app_path, "publisher"))
        .send()
        .await?;
    assert_eq!(
        resp.status(),
        201,
        "non-admin creating a publisher app must succeed: {}",
        resp.text().await?
    );

    // 3. A non-admin cannot flip an existing app to anonymous.
    let resp = authed(
        client().post(format!("{ws}/apps/update/{app_path}")),
        USER_TOKEN,
    )
    .json(&policy_update("anonymous"))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 403,
        "non-admin flipping an app to anonymous must be rejected: {body}"
    );

    // 4. An admin can flip the app to anonymous.
    let resp = authed(
        client().post(format!("{ws}/apps/update/{app_path}")),
        ADMIN_TOKEN,
    )
    .json(&policy_update("anonymous"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "admin flipping an app to anonymous must succeed: {}",
        resp.text().await?
    );

    // 5. A non-admin can still redeploy an app that is already anonymous —
    //    keeping it anonymous does not change its exposure, and blocking it
    //    would prevent non-admin editors from deploying public apps at all
    //    (the frontend always sends the full policy on deploy).
    let resp = authed(
        client().post(format!("{ws}/apps/update/{app_path}")),
        USER_TOKEN,
    )
    .json(&json!({
        "value": { "edited": true },
        "policy": { "execution_mode": "anonymous", "triggerables": {} }
    }))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "non-admin redeploying an already-anonymous app must succeed: {}",
        resp.text().await?
    );

    // 6. A non-admin can downgrade the app back to publisher (reduces exposure).
    let resp = authed(
        client().post(format!("{ws}/apps/update/{app_path}")),
        USER_TOKEN,
    )
    .json(&policy_update("publisher"))
    .send()
    .await?;
    assert_eq!(
        resp.status(),
        200,
        "non-admin downgrading anonymous to publisher must succeed: {}",
        resp.text().await?
    );

    // 7. After the downgrade, flipping back to anonymous requires admin again.
    let resp = authed(
        client().post(format!("{ws}/apps/update/{app_path}")),
        USER_TOKEN,
    )
    .json(&policy_update("anonymous"))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 403,
        "non-admin re-flipping to anonymous must be rejected: {body}"
    );

    Ok(())
}
