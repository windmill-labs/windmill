//! A caller's `_MODULES` never reaches a job: the worker builds that arg in as module code,
//! so on a deployed runnable it would run caller code as the runnable (and its
//! `on_behalf_of`). `push` drops it from `args` and `extra` alike and only sets it from a
//! preview's own `RawCode::modules`.

use std::collections::HashMap;

use serde_json::{json, value::RawValue};
use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{JobPayload, RawCode},
    runnable_settings::{ConcurrencySettings, DebouncingSettings},
    scripts::{ScriptHash, ScriptLang, ScriptModule},
};
use windmill_queue::{PushArgs, PushIsolationLevel};

fn modules(v: &str) -> serde_json::Value {
    json!({ "helper.ts": { "content": format!("export const v = \"{v}\""), "language": "bun" } })
}

/// Pushes `payload` with a caller `_MODULES` both in `args` and in `extra` (where webhook
/// query and headers land), and returns the stored `_MODULES`.
async fn stored_modules(db: &Pool<Postgres>, payload: JobPayload) -> Option<serde_json::Value> {
    let caller: Box<RawValue> = serde_json::value::to_raw_value(&modules("caller")).unwrap();
    let args = HashMap::from([("_MODULES".to_string(), caller.clone())]);
    let extra = HashMap::from([("_MODULES".to_string(), caller)]);
    let (id, tx) = windmill_queue::push(
        db,
        PushIsolationLevel::IsolatedRoot(db.clone()),
        "test-workspace",
        payload,
        PushArgs { args: &args, extra: Some(extra) },
        "test-user",
        "test@windmill.dev",
        "u/test-user".to_string(),
        None,
        None,
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
        None,
        None,
        None,
        None,
        false,
        None,
        None,
        None,
    )
    .await
    .expect("push must succeed");
    tx.commit().await.unwrap();
    sqlx::query_scalar::<_, Option<serde_json::Value>>(
        "SELECT args->'_MODULES' FROM v2_job WHERE id = $1",
    )
    .bind(id)
    .fetch_one(db)
    .await
    .unwrap()
}

#[sqlx::test(fixtures("base"))]
async fn caller_modules_never_reach_a_job(db: Pool<Postgres>) {
    let deployed = JobPayload::ScriptHash {
        hash: ScriptHash(123412),
        path: "f/system/hello".to_string(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        language: ScriptLang::Bun,
        priority: None,
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default(),
        debouncing_settings: DebouncingSettings::default(),
        labels: None,
    };
    assert_eq!(stored_modules(&db, deployed).await, None);

    let preview = |modules: Option<HashMap<String, ScriptModule>>| {
        JobPayload::Code(RawCode {
            hash: None,
            content: "import { v } from \"./helper\"; export function main() { return v }"
                .to_string(),
            path: None,
            language: ScriptLang::Bun,
            lock: None,
            concurrency_settings: ConcurrencySettings::default().into(),
            debouncing_settings: DebouncingSettings::default(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            modules,
            tag: None,
        })
    };
    assert_eq!(stored_modules(&db, preview(None)).await, None);
    let own = serde_json::from_value(modules("preview")).unwrap();
    assert_eq!(
        stored_modules(&db, preview(Some(own))).await,
        Some(modules("preview"))
    );
}
