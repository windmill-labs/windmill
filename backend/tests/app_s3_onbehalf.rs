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
/// `None` => an ordinary direct `/jobs/run` (no app marker). `created_by` is the user
/// the job ran as (the isolation key the gate confines downloads to).
async fn seed_completed_job(
    db: &Pool<Postgres>,
    created_by: &str,
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
                    $1, 'u/test-user',
                    CASE WHEN $2::text IS NULL THEN NULL ELSE 'app'::job_trigger_kind END, $2)
            RETURNING id
        )
        INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status, result, started_at)
        SELECT id, 'test-workspace', 1, 'success', $3::jsonb, now() FROM j
        "#,
    )
    .bind(created_by)
    .bind(app_trigger)
    .bind(&result)
    .execute(db)
    .await?;
    Ok(())
}

/// A deployed app that renders S3 files it produced (e.g. a SQL query persisted to
/// S3 by a component) must clear the provenance gate for the viewer whose own app
/// run produced them, while (a) a viewer cannot forge provenance by running a
/// runnable directly (no app marker), (b) another app's outputs stay denied, and
/// (c) another viewer's outputs stay denied (cross-viewer isolation). Provenance is
/// keyed on the app-origination marker (`trigger_kind='app'` + `trigger=<app path>`)
/// that `execute_component` stamps, plus `created_by = <this caller>` for isolation.
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
    // Produced by test-user-2's own app run of THIS app.
    const USER_KEY: &str = "results/user2_output.parquet";
    // Produced by test-user's own app run of THIS app.
    const ADMIN_KEY: &str = "results/admin_output.parquet";
    // Produced by an app run of a DIFFERENT app → must stay denied.
    const OTHER_APP_KEY: &str = "results/other_app_output.parquet";
    // Produced by a DIRECT run (no app marker) → the forgery attempt, must stay denied.
    const FORGED_KEY: &str = "results/author_only_secret.parquet";

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
    seed_completed_job(&db, "test-user-2", Some(FS_APP), USER_KEY).await?;
    seed_completed_job(&db, "test-user", Some(FS_APP), ADMIN_KEY).await?;
    seed_completed_job(&db, "test-user-2", Some(OTHER_APP), OTHER_APP_KEY).await?;
    seed_completed_job(&db, "test-user-2", None, FORGED_KEY).await?;

    let get = |route: &str, token: &'static str| {
        let url = format!("{ws}/apps_u/{route}");
        authed(client().get(url), token).send()
    };
    let denied = |body: &str| body.contains("File restricted");
    let body_of = |route: String, token: &'static str| async move {
        get(&route, token).await.unwrap().text().await.unwrap()
    };

    // The viewer's own app run's output clears the gate (the case that regressed to
    // "File restricted").
    let body = body_of(
        format!("download_s3_file/{FS_APP}?s3={USER_KEY}"),
        USER_TOKEN,
    )
    .await;
    assert!(
        !denied(&body),
        "viewer's own app-produced key must clear the gate: {body}"
    );

    // The admin viewer's own app run's output clears — the gate has no admin bypass,
    // it just matches the caller's own runs.
    let body = body_of(
        format!("download_s3_file/{FS_APP}?s3={ADMIN_KEY}"),
        ADMIN_TOKEN,
    )
    .await;
    assert!(
        !denied(&body),
        "admin's own app-produced key must clear the gate: {body}"
    );

    // Cross-viewer isolation: the admin cannot pull test-user-2's result even though
    // it is a genuine app-marked job of the same app (no admin bypass either).
    let body = body_of(
        format!("download_s3_file/{FS_APP}?s3={USER_KEY}"),
        ADMIN_TOKEN,
    )
    .await;
    assert!(
        denied(&body),
        "another viewer's app-produced key must stay denied (isolation): {body}"
    );

    // A key produced by a direct run (no app marker) stays denied — the forgery the
    // app-origination marker closes.
    let body = body_of(
        format!("download_s3_file/{FS_APP}?s3={FORGED_KEY}"),
        USER_TOKEN,
    )
    .await;
    assert!(
        denied(&body),
        "key from a direct run (no app marker) must stay denied: {body}"
    );

    // A key produced by a DIFFERENT app stays denied — provenance is scoped to THIS
    // app's path.
    let body = body_of(
        format!("download_s3_file/{FS_APP}?s3={OTHER_APP_KEY}"),
        USER_TOKEN,
    )
    .await;
    assert!(
        denied(&body),
        "key produced by a different app must stay denied: {body}"
    );

    Ok(())
}

/// Seed a minimal deployed script so `execute_component` can resolve `script/<path>`.
async fn seed_script(db: &Pool<Postgres>, path: &str, content: &str) -> anyhow::Result<()> {
    let mut h = 0i64;
    for b in path.bytes().chain(content.bytes()) {
        h = h.wrapping_mul(31).wrapping_add(b as i64);
    }
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, summary, description, content,
                                created_by, language, tag, lock)
           VALUES ('test-workspace', $1, $2, '', '', $3, 'test-user', 'deno'::script_lang, 'deno', '')
           ON CONFLICT DO NOTHING"#,
    )
    .bind(h)
    .bind(path)
    .bind(content)
    .execute(db)
    .await?;
    // #[sqlx::test] isolated DBs share one workspace id and reuse script paths; the
    // process-global deployed-script cache is keyed by (workspace, path), so disable
    // it here so `execute_component` resolves against this test's own DB.
    windmill_common::DEPLOYED_SCRIPT_CACHE_DISABLED
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// End-to-end: `execute_component` must stamp the job it enqueues with
/// `trigger_kind = 'app'` + `trigger = <app path>`. This is the marker the S3
/// provenance gate relies on; the gate tests seed it directly, so this test proves
/// the runtime actually produces it. `execute_component` commits the job row and
/// returns its id, so we assert on the row without needing a worker to run it.
#[sqlx::test(fixtures("base"))]
async fn test_execute_component_stamps_app_trigger(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    const APP_PATH: &str = "u/test-user/trigger_marker_app";
    const SCRIPT_PATH: &str = "u/test-user/query_to_s3";

    seed_script(&db, SCRIPT_PATH, "export function main() { return 1 }").await?;

    // Anonymous app wired to run the deployed script; keys use the production
    // component-prefixed triggerable form.
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP_PATH,
            "summary": "trigger marker test",
            "value": {},
            "policy": {
                "execution_mode": "anonymous",
                "triggerables_v2": {
                    format!("comp1:script/{SCRIPT_PATH}"): { "static_inputs": {}, "one_of_inputs": {} }
                }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "app create: {}", resp.text().await?);

    // Run the script component through the app runtime.
    let resp = authed(
        client().post(format!("{ws}/apps_u/execute_component/{APP_PATH}")),
        ADMIN_TOKEN,
    )
    .json(&json!({
        "component": "comp1",
        "path": format!("script/{SCRIPT_PATH}"),
        "args": {}
    }))
    .send()
    .await?;
    let status = resp.status();
    let job_id = resp.text().await?;
    assert_eq!(status, 200, "execute_component: {job_id}");
    let job_id = job_id.trim().trim_matches('"');

    // The enqueued job must carry the app-origination marker: trigger_kind = 'app'
    // and trigger = the app path (NOT the runnable path).
    let (trigger_kind, trigger): (Option<String>, Option<String>) = sqlx::query_as(
        "SELECT trigger_kind::text, trigger FROM v2_job WHERE id = $1::uuid AND workspace_id = 'test-workspace'",
    )
    .bind(job_id)
    .fetch_one(&db)
    .await?;

    assert_eq!(
        trigger_kind.as_deref(),
        Some("app"),
        "execute_component must stamp trigger_kind = 'app' (got {trigger_kind:?})"
    );
    assert_eq!(
        trigger.as_deref(),
        Some(APP_PATH),
        "trigger must be the app path, not the runnable path (got {trigger:?})"
    );

    Ok(())
}

/// `JobTriggerKind::App` (added for the app-origination S3 marker) is now a valid
/// value for the suspended-trigger reassignment routes, but there is no
/// `app_trigger` table. The handler must reject it with a clean 400 rather than
/// failing on a missing-relation database error (500).
#[sqlx::test(fixtures("base"))]
async fn test_app_trigger_kind_rejected_for_reassignment(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let resp = authed(
        client().post(format!(
            "{ws}/trigger/app/resume_suspended_trigger_jobs/u/test-user/x"
        )),
        ADMIN_TOKEN,
    )
    .json(&json!({}))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 400,
        "app reassignment must be a clean 400, not 500: {body}"
    );
    assert!(
        body.contains("do not support job reassignment"),
        "expected reassignment-unsupported message, got: {body}"
    );

    Ok(())
}

/// A preview is stamped with the app-origination marker ONLY when the caller can
/// edit that app. Preview mode lets a `jobs:run` caller supply arbitrary `raw_code`
/// against any app path; marking such a run for a victim app the caller cannot edit
/// would forge the marker the S3 gate trusts. An app editor previewing their own app
/// IS marked (they already wield the app's author identity), so their preview
/// results stay downloadable.
#[sqlx::test(fixtures("base"))]
async fn test_preview_marker_requires_app_write(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // App owned by test-user (ADMIN_TOKEN). test-user-2 (USER_TOKEN) cannot edit it.
    const APP: &str = "u/test-user/preview_app";
    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP,
            "summary": "preview marker test",
            "value": {},
            "policy": { "execution_mode": "anonymous", "triggerables": {} }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "app create: {}", resp.text().await?);

    // Preview arbitrary inline code against the app (force_viewer_static_fields =>
    // preview mode; raw_code with no path/id skips all app authorization), as `token`.
    let preview_trigger_kind = |token: &'static str| {
        let ws = ws.clone();
        let db = db.clone();
        async move {
            let resp = authed(
                client().post(format!("{ws}/apps_u/execute_component/{APP}")),
                token,
            )
            .json(&json!({
                "component": "comp1",
                "raw_code": { "content": "export function main() { return 1 }", "language": "deno" },
                "force_viewer_static_fields": {},
                "args": {}
            }))
            .send()
            .await
            .unwrap();
            let status = resp.status();
            let job_id = resp.text().await.unwrap();
            assert_eq!(status, 200, "preview execute_component: {job_id}");
            let job_id = job_id.trim().trim_matches('"').to_string();
            let (trigger_kind, trigger): (Option<String>, Option<String>) = sqlx::query_as(
                "SELECT trigger_kind::text, trigger FROM v2_job WHERE id = $1::uuid AND workspace_id = 'test-workspace'",
            )
            .bind(job_id)
            .fetch_one(&db)
            .await
            .unwrap();
            (trigger_kind, trigger)
        }
    };

    // Non-editor (test-user-2) preview against an app they cannot edit → NOT marked.
    let (trigger_kind, trigger) = preview_trigger_kind(USER_TOKEN).await;
    assert_eq!(
        trigger_kind, None,
        "a non-editor's preview must NOT be stamped trigger_kind='app' (got {trigger_kind:?}/{trigger:?})"
    );

    // The app owner (test-user) previewing their own app → marked, so their preview
    // results remain downloadable in the editor.
    let (trigger_kind, trigger) = preview_trigger_kind(ADMIN_TOKEN).await;
    assert_eq!(
        trigger_kind.as_deref(),
        Some("app"),
        "an app editor's own preview must be stamped trigger_kind='app' (got {trigger_kind:?})"
    );
    assert_eq!(
        trigger.as_deref(),
        Some(APP),
        "the editor preview marker's trigger must be the app path (got {trigger:?})"
    );

    Ok(())
}
