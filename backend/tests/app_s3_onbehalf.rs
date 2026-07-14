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

/// Seed a completed job whose result carries an s3 object, as if a deployed
/// script/flow component of an app had just produced it. Returns nothing; the
/// provenance gate matches on (workspace, kind, runnable_path, created_by /
/// permissioned_as, started_at, result).
async fn seed_completed_job(
    db: &Pool<Postgres>,
    kind: &str,
    runnable_path: &str,
    created_by: &str,
    permissioned_as: &str,
    s3_key: &str,
) -> anyhow::Result<()> {
    let result = format!(r#"{{"s3":"{s3_key}"}}"#);
    sqlx::query(
        r#"
        WITH j AS (
            INSERT INTO v2_job (id, workspace_id, kind, runnable_path, created_by, permissioned_as)
            VALUES (gen_random_uuid(), 'test-workspace', $1::job_kind, $2, $3, $4)
            RETURNING id
        )
        INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, result, started_at)
        SELECT id, 'test-workspace', 1, 'success', $5::jsonb, now() FROM j
        "#,
    )
    .bind(kind)
    .bind(runnable_path)
    .bind(created_by)
    .bind(permissioned_as)
    .bind(&result)
    .execute(db)
    .await?;
    Ok(())
}

/// A deployed app that renders S3 files produced by a wired-in **script/flow**
/// component (e.g. a SQL query persisted to S3) must clear the provenance gate for
/// logged-in viewers — the gate previously matched only inline `appscript`/`preview`
/// jobs under the app path, so script/flow outputs were denied "File restricted"
/// for every viewer (admins included). Reads outside the app's declared triggerables
/// stay denied (confused-deputy guard).
#[sqlx::test(fixtures("base"))]
async fn test_deployed_app_s3_onbehalf_flow_script_provenance(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    const FS_APP: &str = "u/test-user/s3flowscript";
    // The deployed script/flow components this app is wired to trigger. A
    // script/flow job's `runnable_path` is the bare path; a flow *step* job's is
    // `<flow_path>/<step_id>`.
    const TRIGGERED_SCRIPT: &str = "u/test-user/query_to_s3";
    const TRIGGERED_FLOW: &str = "u/test-user/query_flow";
    const OFF_APP_SCRIPT: &str = "u/test-user/unrelated";
    // Produced by the wired-in script → provenance-covered.
    const PRODUCED_KEY: &str = "results/query_output.parquet";
    // Produced by a step of the wired-in flow (flowscript job) → provenance-covered.
    const FLOW_STEP_KEY: &str = "results/flow_step_output.parquet";
    // Produced by a script the app does not trigger → must stay denied.
    const OFF_APP_KEY: &str = "results/unrelated_output.parquet";
    // Produced on-behalf of the author (created_by is a different user, but the
    // job's permissioned_as matches the app's on_behalf_of) → provenance-covered.
    const ONBEHALF_KEY: &str = "results/author_produced.parquet";
    // A viewer ran the declared script DIRECTLY (as themselves, not through the app)
    // and crafted a result pointing at an author-only key. permissioned_as is the
    // viewer, not the author, so this must NOT forge app provenance.
    const FORGED_KEY: &str = "results/author_only_secret.parquet";

    // Anonymous app: on_behalf_of auto-sets to the creator (u/test-user). It
    // declares triggering `TRIGGERED_SCRIPT`/`TRIGGERED_FLOW` but has no static
    // allowed_s3_keys, so only the recent-production path can clear the gate. The
    // keys use the production component-prefixed form (`<component_id>:script/...`)
    // that the frontend emits, exercising the prefix-stripping parser branch.
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": FS_APP,
            "summary": "s3 flow/script provenance test",
            "value": {},
            "policy": {
                "execution_mode": "anonymous",
                "triggerables_v2": {
                    format!("a:script/{TRIGGERED_SCRIPT}"): {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    },
                    format!("b:flow/{TRIGGERED_FLOW}"): {
                        "static_inputs": {},
                        "one_of_inputs": {}
                    }
                }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "app create: {}", resp.text().await?);

    // Seed the produced-file jobs (all within the 3h window).
    seed_completed_job(
        &db,
        "script",
        TRIGGERED_SCRIPT,
        "test-user",
        "u/test-user",
        PRODUCED_KEY,
    )
    .await?;
    // A flow step: kind `flowscript`, runnable_path `<flow_path>/<step_id>`.
    seed_completed_job(
        &db,
        "flowscript",
        &format!("{TRIGGERED_FLOW}/step_a"),
        "test-user",
        "u/test-user",
        FLOW_STEP_KEY,
    )
    .await?;
    seed_completed_job(
        &db,
        "script",
        OFF_APP_SCRIPT,
        "test-user",
        "u/test-user",
        OFF_APP_KEY,
    )
    .await?;
    // created_by is someone else, but the job ran on-behalf of the app author
    // (permissioned_as = u/test-user) → provenance-covered (created_by is irrelevant).
    seed_completed_job(
        &db,
        "script",
        TRIGGERED_SCRIPT,
        "someone-else",
        "u/test-user",
        ONBEHALF_KEY,
    )
    .await?;
    // The non-admin viewer ran the SAME declared script directly, as themselves
    // (permissioned_as = u/test-user-2), producing a result that names an author-only
    // key. This must not clear the gate — it is not an app-launched author execution.
    seed_completed_job(
        &db,
        "script",
        TRIGGERED_SCRIPT,
        "test-user-2",
        "u/test-user-2",
        FORGED_KEY,
    )
    .await?;

    let get = |route: &str, token: &'static str| {
        let url = format!("{ws}/apps_u/{route}");
        authed(client().get(url), token).send()
    };
    let denied = |body: &str| body.contains("File restricted");

    // A key produced by the app's wired-in script clears the gate for a logged-in
    // non-admin viewer (the exact case that regressed to "File restricted").
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={PRODUCED_KEY}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        !denied(&body),
        "script-produced key wired into the app must clear the gate: {body}"
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
        "script-produced key must clear the gate for admin viewer too: {body}"
    );

    // A key produced by a *step* of the app's wired-in flow (flowscript job, whose
    // runnable_path is `<flow_path>/<step_id>`) clears the gate via the flow-path
    // prefix match.
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={FLOW_STEP_KEY}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        !denied(&body),
        "flow-step-produced key under a wired-in flow must clear the gate: {body}"
    );

    // A file produced on-behalf of the app author (permissioned_as match) clears
    // the gate even when created_by is a different user.
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={ONBEHALF_KEY}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        !denied(&body),
        "on-behalf-produced key (permissioned_as match) must clear the gate: {body}"
    );

    // A file produced by a script the app does NOT trigger stays denied — the fix
    // must not widen provenance to arbitrary author outputs (confused-deputy guard).
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={OFF_APP_KEY}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        denied(&body),
        "key produced by a non-triggered script must stay denied: {body}"
    );

    // A file a viewer produced by running the declared script DIRECTLY (as
    // themselves) must stay denied — provenance is keyed on the on-behalf identity,
    // so a viewer cannot forge author-provenance to launder the author's S3 perms.
    let body = get(
        &format!("download_s3_file/{FS_APP}?s3={FORGED_KEY}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        denied(&body),
        "key produced by a viewer's own direct run must stay denied: {body}"
    );

    Ok(())
}
