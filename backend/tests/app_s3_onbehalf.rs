//! Deployed-app S3 reads authorize on-behalf of the app author and are confined to
//! app provenance (declared keys or recent job outputs): an anonymous viewer cannot
//! read an arbitrary `file_key` as the author. A viewer on a full (unscoped) session
//! instead falls back to reading as THEMSELVES (bounded by their own S3 perms), so the
//! gate is exercised here through the anonymous identity it still fully protects.
//! Requires the `parquet` feature — the real `apps_u/*` S3 handlers are gated on it.
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

/// Mint an API token for test-user (admin) restricted to `scopes`.
async fn mint_scoped_token(port: u16, scopes: Vec<&str>) -> anyhow::Result<String> {
    let resp = authed(
        client().post(format!("http://localhost:{port}/api/users/tokens/create")),
        ADMIN_TOKEN,
    )
    .json(&json!({ "label": "scoped", "scopes": scopes, "workspace_id": "test-workspace" }))
    .send()
    .await?;
    assert_eq!(resp.status(), 201, "mint scoped token");
    Ok(resp.text().await?)
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

    // GET an app-scoped S3 route ANONYMOUSLY. Anonymous callers have no viewer
    // identity to fall back to, so the provenance gate still fully applies to them
    // (unlike logged-in viewers, who now read as themselves — see the union test).
    // No workspace storage is configured, so a request that clears the gate fails
    // later at the storage lookup (or the CE OSS stub), never with the denial
    // message — which is what lets these assertions distinguish pass from deny.
    let get = |route: &str| {
        let url = format!("{ws}/apps_u/{route}");
        client().get(url).send()
    };
    let denied = |body: &str| body.contains("is not accessible from this app");

    // download_s3_file: allowed for the declared key, denied for a key the app never
    // declared (the confused-deputy guard).
    let body = get(&format!("download_s3_file/{APP}?s3={DECLARED}"))
        .await?
        .text()
        .await?;
    assert!(!denied(&body), "declared key must clear the gate: {body}");
    let body = get(&format!("download_s3_file/{APP}?s3={NON_PROVENANCE}"))
        .await?
        .text()
        .await?;
    assert!(denied(&body), "non-provenance key must be denied: {body}");

    // load_table_count and load_csv_preview enforce the same gate. The preview's
    // numeric `limit`/`offset` must deserialize (regression: a flattened query
    // struct 400s on them under serde_urlencoded).
    let body = get(&format!("load_table_count/{APP}?file_key={DECLARED}"))
        .await?
        .text()
        .await?;
    assert!(
        !denied(&body),
        "table_count declared key must clear the gate: {body}"
    );
    let body = get(&format!("load_table_count/{APP}?file_key={NON_PROVENANCE}"))
        .await?
        .text()
        .await?;
    assert!(
        denied(&body),
        "table_count non-provenance key must be denied: {body}"
    );

    let resp = get(&format!(
        "load_csv_preview/{APP}?file_key={DECLARED}&limit=5&offset=0"
    ))
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_ne!(status, 400, "numeric limit/offset must deserialize: {body}");
    assert!(
        !denied(&body),
        "csv_preview declared key must clear the gate: {body}"
    );

    // load_file_preview: `read_bytes_from` / `read_bytes_length` are required.
    let resp = get(&format!("load_file_preview/{APP}?file_key={DECLARED}")).await?;
    assert_eq!(
        resp.status(),
        400,
        "file_preview without byte range must 400: {}",
        resp.text().await?
    );
    let body = get(&format!(
        "load_file_preview/{APP}?file_key={DECLARED}&read_bytes_from=0&read_bytes_length=4096"
    ))
    .await?
    .text()
    .await?;
    assert!(
        !denied(&body),
        "file_preview declared key must clear the gate: {body}"
    );

    Ok(())
}

/// The viewer-perm union: a viewer on a full (unscoped) session is no longer hard-denied
/// by the provenance gate for a pre-existing file. It falls back to reading as ITSELF
/// (bounded by its own S3 perms downstream), while an anonymous caller (no identity) and
/// a scope-restricted token (can hit `apps_u/*` but not `job_helpers/*`, so the fallback
/// would be a new capability) both stay fully gated with the actionable denial.
#[sqlx::test(fixtures("base"))]
async fn test_deployed_app_s3_viewer_union(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP,
            "summary": "s3 viewer union test",
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

    let url = format!("{ws}/apps_u/download_s3_file/{APP}?s3={NON_PROVENANCE}");

    // Anonymous: still gated. The denial is the actionable message and echoes the key.
    let body = client().get(&url).send().await?.text().await?;
    assert!(
        body.contains("is not accessible from this app"),
        "anonymous viewer must stay gated with the actionable denial: {body}"
    );
    assert!(
        body.contains(NON_PROVENANCE),
        "denial must echo the requested key: {body}"
    );

    // Logged-in viewer: no longer hard-denied — the gate delegates to reading as the
    // viewer, so the request falls through to the storage read (no gate denial in
    // EITHER the old or new form). No workspace storage is configured here, so it
    // surfaces a downstream storage/OSS error, not a gate denial.
    let body = authed(client().get(&url), USER_TOKEN)
        .send()
        .await?
        .text()
        .await?;
    assert!(
        !body.contains("is not accessible from this app") && !body.contains("File restricted"),
        "logged-in viewer must delegate to its own read, not be gate-denied: {body}"
    );

    // Scope-restricted token: an `apps:read:<app>` token reaches this route but is
    // REJECTED by the route-scope middleware on `job_helpers/*`, so it must NOT get the
    // viewer fallback (that would be a capability it cannot obtain directly). It stays
    // gated with the denial, unlike the unscoped session above.
    let apps_read_scope = format!("apps:read:{APP}");
    let scoped = mint_scoped_token(port, vec![apps_read_scope.as_str()]).await?;
    let body = authed(client().get(&url), &scoped)
        .send()
        .await?
        .text()
        .await?;
    assert!(
        body.contains("is not accessible from this app"),
        "scope-restricted token must stay gated, not get the viewer fallback: {body}"
    );

    // A filter-tags-only token carries no real scope restriction (the route-scope
    // middleware treats it as unscoped), so it can read via job_helpers directly and
    // MUST get the viewer fallback here — not be gated like a genuinely scoped token.
    let tag_only = mint_scoped_token(port, vec!["if_jobs:filter_tags:default"]).await?;
    let body = authed(client().get(&url), &tag_only)
        .send()
        .await?
        .text()
        .await?;
    assert!(
        !body.contains("is not accessible from this app") && !body.contains("File restricted"),
        "filter-tags-only token is effectively unscoped and must delegate, not be gated: {body}"
    );

    Ok(())
}

/// Mint a presigned bearer (`exp=..&sig=..`) exactly as `sign_s3_objects` does:
/// `HMAC-SHA256(workspace_key, "file_key={s3}&exp={exp}")` (no storage param, since
/// these routes send none). `validate_s3_signature` is `private`-gated, so this test
/// only runs with the `private` feature.
#[cfg(feature = "private")]
fn mint_presigned(workspace_key: &str, s3: &str, exp: i64) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(workspace_key.as_bytes()).unwrap();
    mac.update(format!("file_key={s3}&exp={exp}").as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    format!("exp={exp}&sig={sig}")
}

/// A presigned S3 object (bearer minted by `signS3Objects`) bypasses the provenance
/// gate on EVERY app-scoped display route, not just the raw `download_s3_file`
/// download: a valid signature clears the gate on preview/count/metadata/csv routes,
/// while an unsigned key stays denied and a forged/expired signature is rejected.
#[cfg(feature = "private")]
#[sqlx::test(fixtures("base"))]
async fn test_deployed_app_s3_presigned_bypasses_gate(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let resp = authed(client().post(format!("{ws}/apps/create")), ADMIN_TOKEN)
        .json(&json!({
            "path": APP,
            "summary": "s3 presigned test",
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

    let workspace_key: String = sqlx::query_scalar(
        "SELECT key FROM workspace_key WHERE workspace_id = 'test-workspace' AND kind = 'cloud'",
    )
    .fetch_one(&db)
    .await?;
    let exp = chrono::Utc::now().timestamp() + 3600;
    let presigned = mint_presigned(&workspace_key, NON_PROVENANCE, exp);

    let get = |route: String, token: &'static str| {
        let url = format!("{ws}/apps_u/{route}");
        authed(client().get(url), token).send()
    };
    let denied = |body: &str| body.contains("is not accessible from this app");

    // Control: NON_PROVENANCE without a signature is denied by the gate. Sent
    // anonymously — a logged-in viewer would instead fall back to reading as
    // themselves, so anonymous is the identity that isolates the presigned bypass.
    let body = client()
        .get(format!(
            "{ws}/apps_u/download_s3_file/{APP}?s3={NON_PROVENANCE}"
        ))
        .send()
        .await?
        .text()
        .await?;
    assert!(
        denied(&body),
        "unsigned non-provenance key must be denied: {body}"
    );

    // Every display route: a valid presigned key clears the gate (falls through to
    // the storage read, which fails with a storage error, never "File restricted").
    // `read_bytes_*` are required on load_file_preview.
    let routes = [
        format!("download_s3_file/{APP}?s3={NON_PROVENANCE}&{presigned}"),
        format!("load_table_count/{APP}?file_key={NON_PROVENANCE}&{presigned}"),
        format!("load_csv_preview/{APP}?file_key={NON_PROVENANCE}&limit=5&offset=0&{presigned}"),
        format!("load_parquet_preview/{APP}?file_key={NON_PROVENANCE}&limit=5&offset=0&{presigned}"),
        format!("load_file_metadata/{APP}?file_key={NON_PROVENANCE}&{presigned}"),
        format!(
            "load_file_preview/{APP}?file_key={NON_PROVENANCE}&read_bytes_from=0&read_bytes_length=4096&{presigned}"
        ),
        format!("download_s3_parquet_file_as_csv/{APP}?file_key={NON_PROVENANCE}&{presigned}"),
    ];
    for route in routes {
        let body = get(route.clone(), USER_TOKEN).await?.text().await?;
        assert!(
            !denied(&body),
            "presigned key must bypass the gate on {route}: {body}"
        );
    }

    // A tampered signature must NOT bypass: presence of `sig` commits to validation,
    // so a wrong sig is rejected outright ("Invalid signature") rather than falling
    // back to the provenance gate.
    let forged = format!("exp={exp}&sig={}", "00".repeat(32));
    let body = get(
        format!("download_s3_file/{APP}?s3={NON_PROVENANCE}&{forged}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        body.contains("Invalid signature"),
        "forged signature must be rejected: {body}"
    );

    // An expired-but-valid signature is rejected on expiry, not bypassed.
    let past = chrono::Utc::now().timestamp() - 10;
    let expired = mint_presigned(&workspace_key, NON_PROVENANCE, past);
    let body = get(
        format!("download_s3_file/{APP}?s3={NON_PROVENANCE}&{expired}"),
        USER_TOKEN,
    )
    .await?
    .text()
    .await?;
    assert!(
        body.contains("Signature expired"),
        "expired signature must be rejected: {body}"
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
/// S3 by a component) must clear the provenance gate for the caller whose own app run
/// produced them, while (a) provenance cannot be forged by running a runnable directly
/// (no app marker), (b) another app's outputs stay denied, and (c) another caller's
/// outputs stay denied (per-caller isolation). Provenance is keyed on the
/// app-origination marker (`trigger_kind='app'` + `trigger=<app path>`) that
/// `execute_component` stamps, plus `created_by = <this caller>` for isolation.
/// Exercised anonymously: the gate still fully governs anonymous callers, whereas a
/// logged-in viewer would instead fall back to reading as themselves.
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
    // Produced by the anonymous caller's own app run of THIS app.
    const OWN_KEY: &str = "results/own_output.parquet";
    // Produced by a DIFFERENT caller's app run of THIS app → isolation, must stay denied.
    const OTHER_CALLER_KEY: &str = "results/user2_output.parquet";
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

    // Seed the produced-file jobs (all within the 3h window). The gate's `created_by`
    // filter uses "anonymous" for an unauthenticated caller.
    seed_completed_job(&db, "anonymous", Some(FS_APP), OWN_KEY).await?;
    seed_completed_job(&db, "test-user-2", Some(FS_APP), OTHER_CALLER_KEY).await?;
    seed_completed_job(&db, "anonymous", Some(OTHER_APP), OTHER_APP_KEY).await?;
    seed_completed_job(&db, "anonymous", None, FORGED_KEY).await?;

    let denied = |body: &str| body.contains("is not accessible from this app");
    // Anonymous GET (borrows `ws`, reusable across calls: the URL is built before the
    // `async move` so only the owned `url` is moved into the future, not `ws`).
    let anon_body = |route: String| {
        let url = format!("{ws}/apps_u/{route}");
        async move {
            client()
                .get(url)
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        }
    };

    // The caller's own app run's output clears the gate (the case that regressed to
    // a hard denial).
    let body = anon_body(format!("download_s3_file/{FS_APP}?s3={OWN_KEY}")).await;
    assert!(
        !denied(&body),
        "caller's own app-produced key must clear the gate: {body}"
    );

    // Per-caller isolation: another caller's result stays denied even though it is a
    // genuine app-marked job of the same app.
    let body = anon_body(format!("download_s3_file/{FS_APP}?s3={OTHER_CALLER_KEY}")).await;
    assert!(
        denied(&body),
        "another caller's app-produced key must stay denied (isolation): {body}"
    );

    // A key produced by a direct run (no app marker) stays denied — the forgery the
    // app-origination marker closes.
    let body = anon_body(format!("download_s3_file/{FS_APP}?s3={FORGED_KEY}")).await;
    assert!(
        denied(&body),
        "key from a direct run (no app marker) must stay denied: {body}"
    );

    // A key produced by a DIFFERENT app stays denied — provenance is scoped to THIS
    // app's path.
    let body = anon_body(format!("download_s3_file/{FS_APP}?s3={OTHER_APP_KEY}")).await;
    assert!(
        denied(&body),
        "key produced by a different app must stay denied: {body}"
    );

    Ok(())
}

/// Seed a minimal deployed script so `execute_component` can resolve `script/<path>`.
/// The script is given its OWN `on_behalf_of` (created_by test-user-2), distinct from
/// any app author, so a test can assert an app component runs as the app's identity,
/// not the referenced script's on_behalf.
async fn seed_script(db: &Pool<Postgres>, path: &str, content: &str) -> anyhow::Result<()> {
    let mut h = 0i64;
    for b in path.bytes().chain(content.bytes()) {
        h = h.wrapping_mul(31).wrapping_add(b as i64);
    }
    sqlx::query(
        r#"INSERT INTO script (workspace_id, hash, path, summary, description, content,
                                created_by, on_behalf_of_email, language, tag, lock)
           VALUES ('test-workspace', $1, $2, '', '', $3, 'test-user-2', 'test2@windmill.dev',
                   'deno'::script_lang, 'deno', '')
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

    // The enqueued job must carry the app-origination marker (trigger_kind = 'app',
    // trigger = the app path, NOT the runnable path) and must run on-behalf of the
    // APP's identity (u/test-user, the anonymous app's author), NOT the referenced
    // script's own on_behalf (u/test-user-2).
    let (trigger_kind, trigger, permissioned_as): (Option<String>, Option<String>, String) =
        sqlx::query_as(
            "SELECT trigger_kind::text, trigger, permissioned_as FROM v2_job \
             WHERE id = $1::uuid AND workspace_id = 'test-workspace'",
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
    assert_eq!(
        permissioned_as, "u/test-user",
        "component must run on-behalf of the APP identity, not the referenced script's on_behalf (got {permissioned_as})"
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

/// A preview run is NEVER app-provenanced. Preview executes as the *caller* (Viewer
/// mode), so its results are read back as the caller via the viewer-scoped
/// job_helpers endpoint — never author-mode. Marking a preview would let any
/// `jobs:run` caller supply arbitrary `raw_code` against a victim app path and forge
/// the marker the S3 gate trusts; and it is never needed. Even the app owner's own
/// preview stays unmarked.
#[sqlx::test(fixtures("base"))]
async fn test_preview_is_not_app_provenanced(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

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
            let trigger_kind: Option<String> = sqlx::query_scalar(
                "SELECT trigger_kind::text FROM v2_job WHERE id = $1::uuid AND workspace_id = 'test-workspace'",
            )
            .bind(job_id)
            .fetch_one(&db)
            .await
            .unwrap();
            trigger_kind
        }
    };

    // The app owner (test-user, admin) previewing their own app → still NOT marked.
    let trigger_kind = preview_trigger_kind(ADMIN_TOKEN).await;
    assert_eq!(
        trigger_kind, None,
        "the app owner's own preview must NOT be app-provenanced (got {trigger_kind:?})"
    );

    // A non-editor (test-user-2) previewing a victim app → NOT marked.
    let trigger_kind = preview_trigger_kind(USER_TOKEN).await;
    assert_eq!(
        trigger_kind, None,
        "a non-editor's preview must NOT be app-provenanced (got {trigger_kind:?})"
    );

    Ok(())
}
