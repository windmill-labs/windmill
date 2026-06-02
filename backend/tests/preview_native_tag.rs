/*
 * Regression tests for WIN-2007.
 *
 * Previewing a TypeScript script carrying the `//native` annotation used to be
 * pushed with `language = bun` (what the editor sends), so the job was tagged
 * `bun` and routed to a regular bun worker. A native-mode worker neither matches
 * the `bun` tag nor accepts a non-native `script_lang`, so previewing a `//native`
 * script on a native-only worker setup failed even though the *deployed* version
 * of the same script runs fine (as `bunnative` / tag `nativets`).
 *
 * `push` now reconciles the preview language with the `//native` annotation,
 * mirroring the deploy-time logic in `worker_lockfiles`. These tests assert the
 * queued job ends up with the right `script_lang` and `tag` for every combination
 * of declared language and annotation. No worker is spawned — we only inspect the
 * row `push` writes.
 */

use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{JobPayload, RawCode},
    scripts::ScriptLang,
};
use windmill_queue::PushIsolationLevel;

async fn push_preview_and_get_row(
    db: &Pool<Postgres>,
    content: &str,
    language: ScriptLang,
) -> (String, Option<ScriptLang>) {
    let hm_args = std::collections::HashMap::new();

    let job = JobPayload::Code(RawCode {
        hash: None,
        content: content.to_string(),
        path: None,
        language,
        lock: None,
        concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default()
            .into(),
        debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        modules: None,
        tag: None,
    });

    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let (uuid, tx) = windmill_queue::push(
        db,
        tx,
        "test-workspace",
        job,
        windmill_queue::PushArgs::from(&hm_args),
        /* user */ "test-user",
        /* email */ "test@windmill.dev",
        /* permissioned_as */ "u/test-user".to_string(),
        /* token_prefix */ None,
        /* scheduled_for */ None,
        /* schedule_path */ None,
        /* parent_job */ None,
        /* root_job */ None,
        /* flow_innermost_root_job */ None,
        /* job_id */ None,
        /* is_flow_step */ false,
        /* same_worker */ false,
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

    let row = sqlx::query!(
        r#"SELECT tag, script_lang AS "script_lang: ScriptLang" FROM v2_job WHERE id = $1"#,
        uuid
    )
    .fetch_one(db)
    .await
    .unwrap();
    (row.tag, row.script_lang)
}

const NATIVE_CONTENT: &str = r#"//native

export function main(x: number) {
    return x;
}
"#;

const PLAIN_CONTENT: &str = r#"export function main(x: number) {
    return x;
}
"#;

/// The reported case: editor sends `bun`, content has `//native`. The preview
/// must be promoted to `bunnative` so it tags `nativets` and a native worker
/// (which rejects non-native `script_lang`) can run it.
#[sqlx::test(fixtures("base"))]
async fn test_bun_with_native_annotation_becomes_nativets(db: Pool<Postgres>) {
    let (tag, lang) = push_preview_and_get_row(&db, NATIVE_CONTENT, ScriptLang::Bun).await;
    assert_eq!(lang, Some(ScriptLang::Bunnative));
    assert_eq!(tag, "nativets");
}

/// A plain bun preview (no `//native`) must stay `bun` / tag `bun`.
#[sqlx::test(fixtures("base"))]
async fn test_bun_without_native_annotation_stays_bun(db: Pool<Postgres>) {
    let (tag, lang) = push_preview_and_get_row(&db, PLAIN_CONTENT, ScriptLang::Bun).await;
    assert_eq!(lang, Some(ScriptLang::Bun));
    assert_eq!(tag, "bun");
}

/// Symmetric to deploy-time logic: a `bunnative` declared language whose content
/// dropped the `//native` annotation is demoted back to `bun` / tag `bun`.
#[sqlx::test(fixtures("base"))]
async fn test_bunnative_without_native_annotation_becomes_bun(db: Pool<Postgres>) {
    let (tag, lang) = push_preview_and_get_row(&db, PLAIN_CONTENT, ScriptLang::Bunnative).await;
    assert_eq!(lang, Some(ScriptLang::Bun));
    assert_eq!(tag, "bun");
}

/// A `bunnative` preview that keeps `//native` stays `bunnative` / tag `nativets`.
#[sqlx::test(fixtures("base"))]
async fn test_bunnative_with_native_annotation_stays_nativets(db: Pool<Postgres>) {
    let (tag, lang) = push_preview_and_get_row(&db, NATIVE_CONTENT, ScriptLang::Bunnative).await;
    assert_eq!(lang, Some(ScriptLang::Bunnative));
    assert_eq!(tag, "nativets");
}
