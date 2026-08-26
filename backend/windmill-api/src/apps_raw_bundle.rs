//! Server-side bundling of a raw app's sources.
//!
//! A raw app is served as a compiled js/css bundle, so every deploy path has to
//! compile first: the editor bundles in a browser iframe, the CLI bundles on the
//! developer's machine. Neither is reachable from a plain API call, which is why
//! `/apps/update_raw` takes the bundle as multipart and why there was no way to
//! deploy a raw app from an API client (an MCP agent, most of all — its only
//! app-write tool was the low-code one, which converted the app instead).
//!
//! The compile runs as a normal bun job on a worker: no new job kind, no new
//! executor, and the build's logs, timeout, cancellation and attribution are the
//! ones every other job gets. The build itself is `wmill app bundle`, so this
//! adds no bundler of its own to keep in step with the CLI's and the editor's.

use std::collections::HashMap;
use std::io::Read;

use base64::Engine;
use serde::Deserialize;
use uuid::Uuid;
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
    worker::to_raw_value,
    DB,
};
use windmill_queue::{push, PushArgs, PushIsolationLevel};

use crate::apps::PolicyTriggerableInputs;
use crate::db::ApiAuthed;

/// The bundle job's script, as its own file so it stays readable TypeScript.
const BUNDLER_TS: &str = include_str!("apps_raw_bundler.ts");

/// The frontend's policy derivation, bundled by `cli/generate-app-policy.ts` and
/// prepended to the job so it runs there. Carried in the binary rather than
/// invoked through the job's `wmill`: the images install the CLI unpinned, so an
/// image can hold one older than its server, and reaching for a pinned release
/// off npm is what the installed-CLI branch below exists to avoid.
const POLICY_JS: &str = include_str!("apps_raw_policy.gen.js");

/// What the job runs: the derivation, then the bundler that calls it.
fn bundle_job_script() -> String {
    format!("{POLICY_JS}\n{BUNDLER_TS}")
}

/// Cap the job so a pathological `package.json` can't sit on a worker forever.
/// This bounds the *run*, not the wait: see `wait_for_bundle`.
const BUNDLE_TIMEOUT_SECS: i32 = 300;

#[derive(Deserialize)]
struct BundleResult {
    js_gz: String,
    css_gz: String,
    /// Parsed rather than passed through: a policy the server cannot read is a
    /// failed deploy, not an app whose runnables are refused at run time.
    #[serde(default)]
    triggerables_v2: HashMap<String, PolicyTriggerableInputs>,
}

/// What a bundle produced: the js/css a deployed raw app serves, and the
/// `triggerables_v2` its policy grants.
pub(crate) struct BundledApp {
    pub js: String,
    pub css: String,
    pub triggerables_v2: HashMap<String, PolicyTriggerableInputs>,
}

/// This server's release, without the git describe suffix an off-tag build
/// carries — the CLI is published per release, so that is the version to ask npm
/// for, and the one an installed CLI must report to be used instead.
fn release_version() -> String {
    let v = &*windmill_common::utils::GIT_SEM_VERSION;
    format!("{}.{}.{}", v.major, v.minor, v.patch)
}

/// The build command the job falls back to when the worker has no usable `wmill`
/// installed: the CLI for this server's release, fetched on the spot. A dev
/// server is off-tag and so asks for the last release; to build with an
/// unreleased CLI set `WM_RAW_APP_BUNDLER_CLI` to the whole command, e.g.
/// `bun run /path/to/cli/src/main.ts app bundle`, which also stops the job from
/// preferring an installed `wmill`.
fn bundler_cli_command() -> Vec<String> {
    match std::env::var("WM_RAW_APP_BUNDLER_CLI") {
        Ok(cmd) if !cmd.trim().is_empty() => {
            cmd.split_whitespace().map(|s| s.to_string()).collect()
        }
        // `bun x`, not `bunx`: the images copy the `bun` binary alone, so the
        // `bunx` entry point isn't on a worker's PATH.
        _ => vec![
            "bun".to_string(),
            "x".to_string(),
            "--bun".to_string(),
            format!("windmill-cli@{}", release_version()),
            "app".to_string(),
            "bundle".to_string(),
        ],
    }
}

/// Compile `files` into the js/css a deployed raw app serves. Returns the
/// build's own error when it fails, so the caller sees the compile error rather
/// than a generic failure.
///
/// This makes a worker run a build on caller-supplied sources, so it requires
/// `jobs:run` here rather than trusting each caller to have checked: a token
/// that can't run jobs must not gain that by writing an app.
pub(crate) async fn bundle_raw_app_sources(
    db: &DB,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    files: &HashMap<String, String>,
    runnables: &serde_json::value::RawValue,
) -> Result<BundledApp> {
    crate::utils::check_scopes(authed, || "jobs:run".to_string())?;

    if files.is_empty() {
        return Err(Error::BadRequest(
            "app value has no `files` to bundle".to_string(),
        ));
    }

    // A queued bundle holds this request open until it runs, so refuse early
    // rather than pile up connections waiting behind a backlog.
    windmill_api_jobs::execution::check_queue_too_long(
        db,
        *windmill_api_jobs::execution::QUEUE_LIMIT_WAIT_RESULT,
    )
    .await?;

    let shared_ui = shared_ui_files(user_db, authed, w_id).await?;

    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    args.insert("files".to_string(), to_raw_value(files));
    args.insert("shared_ui".to_string(), to_raw_value(&shared_ui));
    let overridden = std::env::var("WM_RAW_APP_BUNDLER_CLI").is_ok_and(|c| !c.trim().is_empty());
    args.insert(
        "cli_command".to_string(),
        to_raw_value(&bundler_cli_command()),
    );
    args.insert(
        "prefer_installed_cli".to_string(),
        to_raw_value(&!overridden),
    );
    args.insert("runnables".to_string(), runnables.to_owned());

    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into());
    let (uuid, tx) = push(
        db,
        tx,
        w_id,
        JobPayload::Code(RawCode {
            hash: None,
            content: bundle_job_script(),
            path: Some("bundle raw app".to_string()),
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: Default::default(),
            debouncing_settings: Default::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            modules: None,
            tag: None,
        }),
        PushArgs { args: &args, extra: None },
        authed.display_username(),
        &authed.email,
        windmill_common::users::username_to_permissioned_as(&authed.username),
        authed.token_prefix.as_deref(),
        authed.username_override.as_deref(),
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        Some(BUNDLE_TIMEOUT_SECS),
        None,
        None,
        Some(&authed.clone().into()),
        false,
        None,
        authed.trigger_or_fallback(None),
        None,
    )
    .await?;
    tx.commit().await?;

    wait_for_bundle(db, w_id, uuid, authed).await
}

/// Waits for the bundle job. The wait itself is bounded by
/// `TIMEOUT_WAIT_RESULT`, not by `BUNDLE_TIMEOUT_SECS` — the job's timeout only
/// starts once a worker picks it up.
async fn wait_for_bundle(
    db: &DB,
    w_id: &str,
    uuid: Uuid,
    authed: &ApiAuthed,
) -> Result<BundledApp> {
    let (result, success) = windmill_api_jobs::execution::run_wait_result_internal(
        db,
        uuid,
        w_id,
        None,
        false,
        &authed.username,
    )
    .await?;

    if !success {
        // The job's error is the compile error the caller needs to act on.
        return Err(Error::BadRequest(format!(
            "raw app bundling failed (job {uuid}): {}",
            result.get()
        )));
    }

    let bundle: BundleResult = serde_json::from_str(result.get()).map_err(|e| {
        Error::internal_err(format!(
            "unexpected raw app bundler result (job {uuid}): {e}"
        ))
    })?;
    // One budget across both, so the pair can't hold twice the limit in memory.
    let limit = *crate::REQUEST_SIZE_LIMIT.read().await * 5;
    let js = gunzip_b64(&bundle.js_gz, limit)?;
    let css = gunzip_b64(&bundle.css_gz, limit - js.len())?;
    Ok(BundledApp { js, css, triggerables_v2: bundle.triggerables_v2 })
}

/// Bounded: what the job returns is compressed, so the result-size cap says
/// nothing about what it expands to, and the sources that produced it came from
/// the caller. The budget is shared by the js and css of one bundle, and matches
/// what `/apps/update_raw` accepts as a whole body.
fn gunzip_b64(b64: &str, limit: usize) -> Result<String> {
    let compressed = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| Error::internal_err(format!("raw app bundle is not valid base64: {e}")))?;
    // Bytes, not a String: over the limit the read stops mid-stream, and
    // read_to_string would report that as invalid utf-8 rather than as the size
    // it is. One byte past, so a bundle that just fits is told from one cut short.
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(&compressed[..])
        .take(limit as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| Error::internal_err(format!("raw app bundle is not valid gzip: {e}")))?;
    if out.len() > limit {
        // `limit` is what is left of the budget, not the whole of it, so say so
        // rather than report a nearly-exhausted budget as the limit itself.
        return Err(Error::BadRequest(format!(
            "raw app bundle is too large: {limit} bytes left of the budget the js and css share"
        )));
    }
    String::from_utf8(out)
        .map_err(|e| Error::internal_err(format!("raw app bundle is not valid utf-8: {e}")))
}

async fn shared_ui_files(
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
) -> Result<HashMap<String, String>> {
    let mut tx = user_db.clone().begin(authed).await?;
    let files = sqlx::query_scalar!(
        "SELECT files FROM workspace_shared_ui WHERE workspace_id = $1",
        w_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(files
        .and_then(|f| serde_json::from_value::<HashMap<String, String>>(f).ok())
        .unwrap_or_default())
}
