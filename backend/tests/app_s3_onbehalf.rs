//! Deployed-app S3 reads authorize on-behalf of the app author and are confined
//! to app provenance (declared keys or recent job outputs): a viewer cannot read
//! an arbitrary `file_key` as the author. Requires the `parquet` feature — the
//! real `apps_u/*` S3 handlers are gated on it.
//!
//! `base` fixture: test-user (admin, SECRET_TOKEN); test-user-2 (non-admin,
//! SECRET_TOKEN_2, no S3 folder permission).
#![cfg(feature = "parquet")]

use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";
const USER_TOKEN: &str = "SECRET_TOKEN_2";
const APP: &str = "u/test-user/s3onbehalf";
const DECLARED: &str = "provenance/allowed.csv";
const NON_PROVENANCE: &str = "evil/secret.csv";

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

    // `on_behalf_of` is auto-set to the creator (admin) for an anonymous app, so
    // the app reads S3 as that author; `DECLARED` is the only allowlisted key.
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP,
            "summary": "s3 onbehalf test",
            "value": {},
            "policy": {
                "execution_mode": "anonymous",
                "triggerables": {},
                "allowed_s3_keys": [{ "s3_path": DECLARED }]
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "app create: {}", resp.text().await?);

    // GET an app-scoped S3 route as `token`. No workspace storage is configured,
    // so a request that clears the provenance gate fails later at the storage
    // lookup (or the CE OSS stub), never with "File restricted" — which is what
    // lets these assertions distinguish "gate passed" from "gate denied".
    let get = |route: &str, token: &'static str| {
        let url = format!("{ws}/apps_u/{route}");
        authed(client().get(url), token).send()
    };
    let denied = |body: &str| body.contains("File restricted");

    // download_s3_file: author-on-behalf allowed for the declared key, denied for
    // a key the app never declared (the confused-deputy guard).
    let body = get(&format!("download_s3_file/{APP}?s3={DECLARED}"), USER_TOKEN)
        .await?
        .text()
        .await?;
    assert!(!denied(&body), "declared key must clear the gate: {body}");
    let body = get(
        &format!("download_s3_file/{APP}?s3={NON_PROVENANCE}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(denied(&body), "non-provenance key must be denied: {body}");

    // load_table_count and load_csv_preview enforce the same gate. The preview's
    // numeric `limit`/`offset` must deserialize (regression: a flattened query
    // struct 400s on them under serde_urlencoded).
    let body = get(
        &format!("load_table_count/{APP}?file_key={DECLARED}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        !denied(&body),
        "table_count declared key must clear the gate: {body}"
    );
    let body = get(
        &format!("load_table_count/{APP}?file_key={NON_PROVENANCE}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        denied(&body),
        "table_count non-provenance key must be denied: {body}"
    );

    let resp = get(
        &format!("load_csv_preview/{APP}?file_key={DECLARED}&limit=5&offset=0"),
        USER_TOKEN,
    )
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_ne!(status, 400, "numeric limit/offset must deserialize: {body}");
    assert!(
        !denied(&body),
        "csv_preview declared key must clear the gate: {body}"
    );

    // load_file_preview: `read_bytes_from` / `read_bytes_length` are required.
    let resp = get(
        &format!("load_file_preview/{APP}?file_key={DECLARED}"),
        USER_TOKEN,
    )
    .await?;
    assert_eq!(
        resp.status(),
        400,
        "file_preview without byte range must 400: {}",
        resp.text().await?
    );
    let body = get(
        &format!(
            "load_file_preview/{APP}?file_key={DECLARED}&read_bytes_from=0&read_bytes_length=4096"
        ),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        !denied(&body),
        "file_preview declared key must clear the gate: {body}"
    );

    Ok(())
}
