//! Regression: run mode of `execute_component`'s no-id inline-`raw_code` arm runs
//! *only* the `rawscript/<sha>`-pinned `content`, dropping the caller `hash`,
//! `lock`, `modules` and `dedicated_worker` and deriving `path` server-side —
//! all of which would otherwise run or install unpinned code as the app identity.
//! Preview mode keeps honoring the caller's fields.

use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(b: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    b.header("Authorization", format!("Bearer {}", token))
}

const CONTENT: &str = "print('benign')\n";
// A caller lock whose presence is the whole point: if it reaches the job, the
// worker installs it. The value only needs to be recognizable in `v2_job`.
const CALLER_LOCK: &str = "evilpkg @ file:///tmp/attacker-controlled-sdist";
// A non-codebase-sentinel hash: if it reaches the job as `runnable_id`, the
// worker fetches (and runs) a deployed script by hash instead of the pinned
// content. It need not resolve to a real row — the guard is that it never
// becomes `runnable_id`.
const CALLER_HASH: i64 = 123456789;
// A caller path in someone else's namespace: if it reaches the job as
// `runnable_path` it redirects where the pinned content's relative imports
// resolve. Run mode must instead derive the path from `<app_path>/<component>`.
const CALLER_PATH: &str = "u/attacker/evil/comp";

/// The pin key `execute_component` computes for a no-id inline script:
/// `rawscript/<sha256(content)>`.
fn rawscript_pin(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content);
    format!("rawscript/{:x}", h.finalize())
}

fn inline_raw_code(hash: Option<i64>, dedicated: bool) -> serde_json::Value {
    let mut rc = json!({
        "language": "python3",
        "content": CONTENT,
        "path": CALLER_PATH,
        "lock": CALLER_LOCK,
        "modules": {
            "m.py": { "content": "print('x')\n", "language": "python3", "lock": CALLER_LOCK }
        }
    });
    if let Some(h) = hash {
        rc["hash"] = json!(h);
    }
    if dedicated {
        rc["dedicated_worker"] = json!(true);
    }
    rc
}

/// Fetch `(raw_lock, args-has-_MODULES, runnable_id, tag, runnable_path)` for an
/// enqueued job.
async fn job_fields(
    db: &Pool<Postgres>,
    uuid: uuid::Uuid,
) -> anyhow::Result<(Option<String>, bool, Option<i64>, String, Option<String>)> {
    Ok(sqlx::query_as(
        "SELECT raw_lock, (args ? '_MODULES'), runnable_id, tag, runnable_path \
         FROM v2_job WHERE id = $1",
    )
    .bind(uuid)
    .fetch_one(db)
    .await?)
}

#[sqlx::test(fixtures("base"))]
async fn test_run_mode_strips_caller_lock_and_modules(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let app_path = "u/test-user/lockstrip";
    let pin = format!("comp:{}", rawscript_pin(CONTENT));

    // Deployed Viewer-mode app whose only runnable is an inline script pinned by
    // content hash and with no `app_script` row — the legacy `rawscript/<sha>`
    // case that reaches the no-id run-mode arm this fix touches.
    let resp = authed(client().post(format!("{ws}/apps/create")), "SECRET_TOKEN")
        .json(&json!({
            "path": app_path,
            "summary": "",
            "value": {},
            "policy": {
                "execution_mode": "viewer",
                "triggerables_v2": { pin: { "static_inputs": {}, "one_of_inputs": {} } }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create app: {}", resp.text().await?);

    // Run mode (no `force_viewer_static_fields`): the pin authorizes the run, but
    // every caller field that selects, installs, or routes code — hash, lock,
    // modules, dedicated_worker — must be dropped.
    let resp = authed(
        client().post(format!("{ws}/apps_u/execute_component/{app_path}")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        // The args map is the other injection channel: an inline run is a
        // `JobKind::Preview` job, so the worker/executors read `_MODULES` and
        // `_TEMP_SCRIPT_REFS` back out of the job args. Both must be stripped.
        "args": {
            "_MODULES": { "m.py": { "content": "print('evil')\n", "language": "python3" } },
            "_TEMP_SCRIPT_REFS": { "../evil": "deadbeef" }
        },
        "component": "comp",
        "raw_code": inline_raw_code(Some(CALLER_HASH), true)
    }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(
        status, 200,
        "run-mode pinned inline run must be accepted: {body}"
    );
    let uuid = uuid::Uuid::parse_str(body.trim())?;
    let (raw_lock, has_modules, runnable_id, tag, runnable_path) = job_fields(&db, uuid).await?;
    assert_eq!(
        raw_lock, None,
        "run mode must strip the caller-supplied lock"
    );
    assert!(
        !has_modules,
        "run mode must strip caller modules (both `raw_code.modules` and an `_MODULES` arg)"
    );
    let has_temp_refs: bool =
        sqlx::query_scalar("SELECT (args ? '_TEMP_SCRIPT_REFS') FROM v2_job WHERE id = $1")
            .bind(uuid)
            .fetch_one(&db)
            .await?;
    assert!(
        !has_temp_refs,
        "run mode must strip a caller `_TEMP_SCRIPT_REFS` arg (relative-import redirect)"
    );
    assert_eq!(
        runnable_id, None,
        "run mode must strip the caller-supplied hash (no substituting a deployed script by hash)"
    );
    assert!(
        !tag.starts_with("dedi:"),
        "run mode must strip caller `dedicated_worker` (no routing to a path-keyed dedicated worker), got tag {tag:?}"
    );
    assert_eq!(
        runnable_path.as_deref(),
        Some(format!("{app_path}/comp").as_str()),
        "run mode must derive the path server-side, not trust the caller's (relative-import base)"
    );

    // Preview mode (editor): the caller runs their own code as themselves, so the
    // lock and modules are honored — the `/jobs/run/preview`-equivalent path.
    let resp = authed(
        client().post(format!("{ws}/apps_u/execute_component/{app_path}")),
        "SECRET_TOKEN_2",
    )
    .json(&json!({
        "args": {},
        "component": "comp",
        "raw_code": inline_raw_code(None, false),
        "force_viewer_static_fields": {}
    }))
    .send()
    .await?;
    let status = resp.status();
    let body = resp.text().await?;
    assert_eq!(status, 200, "preview must be accepted: {body}");
    let uuid = uuid::Uuid::parse_str(body.trim())?;
    let (raw_lock, has_modules, _, _, _) = job_fields(&db, uuid).await?;
    assert_eq!(
        raw_lock.as_deref(),
        Some(CALLER_LOCK),
        "preview must keep the caller-supplied lock"
    );
    assert!(has_modules, "preview must keep the caller-supplied modules");

    Ok(())
}

/// A bare `rawscript/<sha>` policy key (no `<component>:` prefix, as `empty_triggerables`
/// migrates v1 policies) matches for any `component`, so run mode must not let a
/// path-traversing `component` steer the server-derived `runnable_path`.
#[sqlx::test(fixtures("base"))]
async fn test_run_mode_rejects_traversal_component(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let app_path = "u/test-user/lockstrip_bare";
    // Bare key: no `comp:` prefix, so the pin matches regardless of `component`.
    let resp = authed(client().post(format!("{ws}/apps/create")), "SECRET_TOKEN")
        .json(&json!({
            "path": app_path,
            "summary": "",
            "value": {},
            "policy": {
                "execution_mode": "viewer",
                "triggerables_v2": { rawscript_pin(CONTENT): { "static_inputs": {}, "one_of_inputs": {} } }
            }
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), 201, "create app: {}", resp.text().await?);

    // A component that isn't a single plain segment steers the derived path's
    // base: separators and `..` traverse, and an empty one shifts it up a level.
    for bad in ["../../u/attacker/evil", "..", "a/b", ""] {
        let resp = authed(
            client().post(format!("{ws}/apps_u/execute_component/{app_path}")),
            "SECRET_TOKEN_2",
        )
        .json(&json!({
            "args": {},
            "component": bad,
            "raw_code": inline_raw_code(None, false)
        }))
        .send()
        .await?;
        let status = resp.status();
        let body = resp.text().await?;
        assert_eq!(
            status, 400,
            "run mode must reject component {bad:?}: got {status}: {body}"
        );
    }

    Ok(())
}
