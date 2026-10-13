//! Every s3_proxy response carries `nosniff` and the sandbox CSP, error responses included,
//! since the presigned route serves workspace objects from the app origin without a login.
#![cfg(all(feature = "enterprise", feature = "private", feature = "parquet"))]

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn test_s3_proxy_sets_content_security_headers(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/api/w/test-workspace/s3_proxy/_default_/x.html?exp=9999999999&sig=00"
        ))
        .send()
        .await?;

    assert!(resp.status().is_client_error(), "status: {}", resp.status());
    assert_eq!(resp.headers()["x-content-type-options"], "nosniff");
    assert_eq!(
        resp.headers()["content-security-policy"],
        "sandbox allow-scripts allow-forms allow-popups allow-modals allow-downloads"
    );
    Ok(())
}
