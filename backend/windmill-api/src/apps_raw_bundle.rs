//! Server-side bundling of a raw app's sources.
//!
//! A raw app is served as a compiled js/css bundle, so every deploy path has to
//! compile first: the editor bundles in a browser iframe, the CLI bundles with
//! esbuild on the developer's machine. Neither is reachable from a plain API
//! call, which is why `/apps/update_raw` takes the bundle as multipart and why
//! there was no way to deploy a raw app from an API client (an MCP agent, most
//! of all — its only app-write tool was the low-code one, which converted the
//! app instead).
//!
//! This runs the compile as a normal bun job on a worker: no new job kind, no
//! new executor, and the build's logs, timeout, cancellation and attribution are
//! the ones every other job gets.

use std::collections::HashMap;

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

use crate::db::ApiAuthed;

/// The bundler, as its own file so it stays readable/lintable TypeScript.
const BUNDLER_TS: &str = include_str!("apps_raw_bundler.ts");

/// The raw-app `wmill` client, vendored from `frontend/src/lib/rawAppWmillTs.ts`
/// because the backend build has no access to the frontend tree. The bundler
/// injects it as a virtual module, exactly as the other two bundlers do.
/// `test_vendored_wmill_ts_matches_frontend` fails if the copies drift.
const RAW_APP_WMILL_TS: &str = include_str!("apps_raw_wmill_ts.ts");

/// Cap the compile so a pathological `package.json` can't hold the request open
/// for the full `TIMEOUT_WAIT_RESULT`.
const BUNDLE_TIMEOUT_SECS: i32 = 300;

#[derive(Deserialize)]
struct BundleResult {
    js: String,
    css: String,
}

/// Compile `files` into the js/css a deployed raw app serves. Returns the
/// bundler's own error when the build fails, so the caller sees the compile
/// error rather than a generic failure.
pub async fn bundle_raw_app_sources(
    db: &DB,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    files: &HashMap<String, String>,
) -> Result<(String, String)> {
    if files.is_empty() {
        return Err(Error::BadRequest(
            "app value has no `files` to bundle".to_string(),
        ));
    }

    let shared_ui = shared_ui_files(user_db, authed, w_id).await?;

    let mut args: HashMap<String, Box<serde_json::value::RawValue>> = HashMap::new();
    args.insert("files".to_string(), to_raw_value(files));
    args.insert("wmill_ts".to_string(), to_raw_value(&RAW_APP_WMILL_TS));
    args.insert("shared_ui".to_string(), to_raw_value(&shared_ui));

    let tx = PushIsolationLevel::Isolated(user_db.clone(), authed.clone().into());
    let (uuid, tx) = push(
        db,
        tx,
        w_id,
        JobPayload::Code(RawCode {
            hash: None,
            content: BUNDLER_TS.to_string(),
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

async fn wait_for_bundle(
    db: &DB,
    w_id: &str,
    uuid: Uuid,
    authed: &ApiAuthed,
) -> Result<(String, String)> {
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
    Ok((bundle.js, bundle.css))
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

#[cfg(test)]
mod tests {
    /// The vendored copy is what the bundler injects, so a change to the
    /// frontend's `wmill` client that doesn't reach it would ship apps compiled
    /// against a different client than the editor compiles against.
    #[test]
    fn test_vendored_wmill_ts_matches_frontend() {
        let frontend = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../frontend/src/lib/rawAppWmillTs.ts");
        // Absent in a backend-only checkout (the published crate, a Docker build
        // context); there is nothing to compare against then.
        let Ok(expected) = std::fs::read_to_string(&frontend) else {
            return;
        };
        assert_eq!(
            expected.replace("\r\n", "\n"),
            super::RAW_APP_WMILL_TS.replace("\r\n", "\n"),
            "backend/windmill-api/src/apps_raw_wmill_ts.ts is stale — copy \
             frontend/src/lib/rawAppWmillTs.ts over it"
        );
    }
}
