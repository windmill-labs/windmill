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

/// Seed a completed job whose result carries an s3 object. `app_trigger` sets the
/// app-origination marker exactly as `execute_component` stamps it: `Some(app_path)`
/// => `trigger_kind = 'app'` + `trigger = <app_path>` (an app-launched run);
/// `None` => an ordinary direct `/jobs/run` (no app marker), used to model a viewer's
/// own execution of a declared runnable. `permissioned_as` is set independently so a
/// direct run can be modeled even when it resolves to the author (runnable on-behalf).
async fn seed_completed_job(
    db: &Pool<Postgres>,
    permissioned_as: &str,
    app_trigger: Option<&str>,
    s3_key: &str,
) -> anyhow::Result<()> {
    let result = format!(r#"{{"s3":"{s3_key}"}}"#);
    sqlx::query(
        r#"
        WITH j AS (
            INSERT INTO v2_job (id, workspace_id, kind, runnable_path, created_by,
                                permissioned_as, trigger_kind, trigger)
            VALUES (gen_random_uuid(), 'test-workspace', 'script', 'u/test-user/query_to_s3',
                    'test-user', $1,
                    CASE WHEN $2::text IS NULL THEN NULL ELSE 'app'::job_trigger_kind END, $2)
            RETURNING id
        )
        INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, result, started_at)
        SELECT id, 'test-workspace', 1, 'success', $3::jsonb, now() FROM j
        "#,
    )
    .bind(permissioned_as)
    .bind(app_trigger)
    .bind(&result)
    .execute(db)
    .await?;
    Ok(())
}

/// A deployed app that renders S3 files it produced (e.g. a SQL query persisted to
/// S3 by a script/flow component) must clear the provenance gate for logged-in
/// viewers, while a viewer cannot forge provenance by running a declared runnable
/// directly. Provenance is keyed on the app-origination marker (`trigger_kind='app'`
/// + `trigger = <app path>`) that `execute_component` stamps — not on
/// created_by/permissioned_as/runnable_path, which a direct run can match.
#[sqlx::test(fixtures("base"))]
async fn test_deployed_app_s3_onbehalf_flow_script_provenance(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    const FS_APP: &str = "u/test-user/s3flowscript";
    const OTHER_APP: &str = "u/test-user/other_app";
    // Produced by an app-launched run of this app → provenance-covered.
    const PRODUCED_KEY: &str = "results/query_output.parquet";
    // Produced by an app-launched run of a DIFFERENT app → must stay denied.
    const OTHER_APP_KEY: &str = "results/other_app_output.parquet";
    // A viewer ran the declared script DIRECTLY (no app marker). Even though the
    // runnable's on-behalf makes permissioned_as = the author, this is not an
    // app-launched run, so it must NOT forge app provenance.
    const FORGED_KEY: &str = "results/author_only_secret.parquet";

    // Anonymous app: on_behalf_of auto-sets to the creator (u/test-user). No static
    // allowed_s3_keys, so only the app-originated recent-production path can clear
    // the gate.
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": FS_APP,
            "summary": "s3 app-origination provenance test",
            "value": {},
            "policy": { "execution_mode": "anonymous", "triggerables": {} }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "app create: {}", resp.text().await?);

    // Seed the produced-file jobs (all within the 3h window).
    // App-launched by THIS app.
    seed_completed_job(&db, "u/test-user", Some(FS_APP), PRODUCED_KEY).await?;
    // App-launched by a DIFFERENT app (same author identity).
    seed_completed_job(&db, "u/test-user", Some(OTHER_APP), OTHER_APP_KEY).await?;
    // A direct run (no app marker) whose permissioned_as resolves to the author,
    // as a runnable configured with on_behalf_of would — the forgery attempt.
    seed_completed_job(&db, "u/test-user", None, FORGED_KEY).await?;

    let get = |route: &str, token: &'static str| {
        let url = format!("{ws}/apps_u/{route}");
        authed(client().get(url), token).send()
    };
    let denied = |body: &str| body.contains("File restricted");

    // A key this app produced clears the gate for a logged-in non-admin viewer (the
    // exact case that regressed to "File restricted").
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={PRODUCED_KEY}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        !denied(&body),
        "app-produced key must clear the gate for a non-admin viewer: {body}"
    );

    // Same for the admin viewer — the gate has no admin bypass, so this also
    // regressed before the fix.
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={PRODUCED_KEY}"),
        ADMIN_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        !denied(&body),
        "app-produced key must clear the gate for admin viewer too: {body}"
    );

    // A file a viewer produced by running the declared runnable DIRECTLY (no app
    // marker) must stay denied — even though permissioned_as resolves to the author.
    // This is the confused-deputy the app-origination marker closes.
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={FORGED_KEY}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        denied(&body),
        "key from a viewer's own direct run (no app marker) must stay denied: {body}"
    );

    // A file another app produced must stay denied — provenance is scoped to THIS
    // app's path, not any app-launched job by the same author.
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={OTHER_APP_KEY}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        denied(&body),
        "key produced by a different app must stay denied: {body}"
    );

    Ok(())
}
