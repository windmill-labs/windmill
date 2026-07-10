//! Deployed-app S3 reads authorize on-behalf of the app author, gated by app
//! provenance — for a logged-in viewer just as for an anonymous one. A viewer
//! must not be able to launder the author's (often privileged) S3 permissions
//! by passing an arbitrary `file_key` to the app's on-behalf endpoint. That is a
//! confused-deputy: before this change a logged-in, non-embed session hit an
//! unconditional `Ok(())` bypass in `check_if_allowed_to_access_s3_file_from_app`
//! and could read ANY key the author could reach.
//!
//! Requires the `parquet` feature — the real `apps_u/*` S3 handlers are gated on
//! it (without it the route returns a "requires parquet" stub).
//!
//! Users from the `base` fixture:
//!   test-user   (admin,     token SECRET_TOKEN)
//!   test-user-2 (non-admin, token SECRET_TOKEN_2) — no S3 folder permissions
#![cfg(feature = "parquet")]

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

#[sqlx::test(fixtures("base"))]
async fn test_deployed_app_s3_onbehalf_provenance(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // Admin deploys an `anonymous` execution-mode app that declares exactly one
    // provenance key in its policy allowlist. `on_behalf_of` is auto-set to the
    // creator (the admin) by the create handler, so the app reads S3 as that
    // author — the whole point of the on-behalf path.
    let provenance_key = "provenance/allowed.parquet";
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": "u/test-user/s3onbehalf",
            "summary": "s3 onbehalf test",
            "value": {},
            "policy": {
                "execution_mode": "anonymous",
                "triggerables": {},
                "allowed_s3_keys": [{ "s3_path": provenance_key }]
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "app create: {}", resp.text().await?);

    let download = |key: &'static str, token: &'static str| {
        let url = format!("{ws}/apps_u/download_s3_file/u/test-user/s3onbehalf?s3={key}");
        authed(client().get(url), token).send()
    };

    // A logged-in NON-admin viewer requesting the app's DECLARED key clears the
    // provenance gate: the request fails only later at the workspace S3-storage
    // lookup (none is configured in the test workspace), NOT with "File
    // restricted". Getting that far proves the read authorized on-behalf of the
    // author rather than against the viewer's own (absent) S3 permissions.
    let body = download(provenance_key, USER_TOKEN).await?.text().await?;
    assert!(
        !body.contains("File restricted"),
        "declared provenance key must pass the app provenance gate for a logged-in viewer, got: {body}"
    );

    // The same viewer requesting a key the app never declared nor produced is
    // denied — this is the confused-deputy guard. Without the fix the logged-in
    // bypass would have let this through and read the key as the admin author.
    let resp = download("evil/secret.parquet", USER_TOKEN).await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert!(
        body.contains("File restricted"),
        "non-provenance key must be denied for a logged-in deployed viewer (confused-deputy guard), got {status}: {body}"
    );

    Ok(())
}
