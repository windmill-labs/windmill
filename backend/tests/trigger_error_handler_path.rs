//! A trigger's error handler is accepted in the `script/f/foo` spelling that
//! schedule and workspace error handlers use, but it is pushed as the failure
//! module of a single-step flow, which resolves a bare runnable path. Without
//! the prefix stripped the handler only fails once the trigger errors, which is
//! exactly when it is needed.

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use windmill_common::{
    jobs::JobPayload,
    runnable_settings::{ConcurrencySettings, DebouncingSettings},
    scripts::ScriptHash,
};
use windmill_test_utils::*;

const SCRIPT_PATH: &str = "u/test-user/rerun_script";
const SCRIPT_HASH: i64 = 1111111111;
const HANDLER_PATH: &str = "u/test-user/error_handler";

fn ssf_with_error_handler(error_handler_path: &str) -> JobPayload {
    JobPayload::SingleStepFlow {
        path: SCRIPT_PATH.to_string(),
        hash: Some(ScriptHash(SCRIPT_HASH)),
        flow_version: None,
        language: None,
        args: Default::default(),
        retry: None,
        error_handler_path: Some(error_handler_path.to_string()),
        error_handler_args: None,
        skip_handler: None,
        cache_ttl: None,
        cache_ignore_s3_path: None,
        priority: None,
        tag_override: None,
        trigger_path: Some("http_trigger/u/test-user/route".to_string()),
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default(),
        debouncing_settings: DebouncingSettings::default(),
    }
}

async fn failure_module_path(db: &Pool<Postgres>, id: Uuid) -> Option<String> {
    let raw_flow = sqlx::query_scalar::<_, Option<serde_json::Value>>(
        "SELECT raw_flow FROM v2_job WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await
    .unwrap()?;
    raw_flow
        .pointer("/failure_module/value/path")?
        .as_str()
        .map(str::to_string)
}

#[sqlx::test(fixtures("base", "batch_rerun"))]
async fn error_handler_path_prefix_is_stripped(db: Pool<Postgres>) -> anyhow::Result<()> {
    let prefixed = RunJob::from(ssf_with_error_handler(&format!("script/{HANDLER_PATH}")))
        .push(&db)
        .await;
    assert_eq!(
        failure_module_path(&db, prefixed).await.as_deref(),
        Some(HANDLER_PATH),
        "a `script/`-prefixed error handler must resolve to the bare runnable path"
    );

    let bare = RunJob::from(ssf_with_error_handler(HANDLER_PATH))
        .push(&db)
        .await;
    assert_eq!(
        failure_module_path(&db, bare).await.as_deref(),
        Some(HANDLER_PATH),
        "an unprefixed error handler is left alone"
    );

    let hub = RunJob::from(ssf_with_error_handler("hub/1234/windmill/handler"))
        .push(&db)
        .await;
    assert_eq!(
        failure_module_path(&db, hub).await.as_deref(),
        Some("hub/1234/windmill/handler"),
        "a hub error handler is left alone — the flow runner resolves `hub/` itself"
    );

    Ok(())
}
