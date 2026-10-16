//! Sets the process-wide `DEV_WORKSPACE_ISOLATION`, so it lives in its own test binary: any other
//! test pushing a job in the same process would land on a suffixed tag.

use std::collections::HashMap;

use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{JobPayload, RawCode},
    runnable_settings::{ConcurrencySettings, DebouncingSettings},
    scripts::{ScriptHash, ScriptLang},
    worker::DEV_WORKSPACE_ISOLATION,
};
use windmill_queue::{PushArgs, PushIsolationLevel};

async fn pushed_tag(
    db: &Pool<Postgres>,
    workspace_id: &str,
    payload: JobPayload,
    tag: Option<&str>,
) -> String {
    let args = HashMap::new();
    let (id, tx) = windmill_queue::push(
        db,
        PushIsolationLevel::IsolatedRoot(db.clone()),
        workspace_id,
        payload,
        PushArgs::from(&args),
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
        tag.map(str::to_string),
        None,
        None,
        None,
        None,
        false,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("push must succeed");
    tx.commit().await.unwrap();
    sqlx::query_scalar("SELECT tag FROM v2_job WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await
        .unwrap()
}

fn preview() -> JobPayload {
    JobPayload::Code(RawCode {
        hash: None,
        content: "echo hi".to_string(),
        path: None,
        language: ScriptLang::Bash,
        lock: None,
        concurrency_settings: ConcurrencySettings::default().into(),
        debouncing_settings: DebouncingSettings::default(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: None,
        modules: None,
        tag: None,
    })
}

#[sqlx::test(fixtures("base"))]
async fn dev_workspace_and_fork_jobs_run_on_suffixed_tags(db: Pool<Postgres>) {
    DEV_WORKSPACE_ISOLATION.store(true, std::sync::atomic::Ordering::Relaxed);

    // A dev workspace, a fork of it, and a fork whose parent was deleted (`ON DELETE SET NULL`).
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id, is_dev_workspace) VALUES
            ('staging', 'staging', 'test-user', 'test-workspace', true),
            ('wm-fork-a', 'fork', 'test-user', 'staging', false),
            ('wm-fork-orphan', 'orphan', 'test-user', NULL, false)",
    )
    .execute(&db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workspace_settings (workspace_id)
         VALUES ('staging'), ('wm-fork-a'), ('wm-fork-orphan')",
    )
    .execute(&db)
    .await
    .unwrap();

    for (workspace, tag, expected) in [
        ("test-workspace", None, "bash"),
        ("test-workspace", Some("gpu"), "gpu"),
        ("staging", None, "bash-dev"),
        ("staging", Some("gpu"), "gpu-dev"),
        // A tag inherited from a dev job (a flow step, an agent's tool) is not suffixed twice.
        ("staging", Some("gpu-dev"), "gpu-dev"),
        ("wm-fork-a", None, "bash-dev"),
        ("wm-fork-orphan", None, "bash-dev"),
    ] {
        assert_eq!(
            pushed_tag(&db, workspace, preview(), tag).await,
            expected,
            "{workspace} with tag {tag:?}"
        );
    }

    // A dedicated worker tag already names the workspace, and its worker is placed by hand.
    let dedicated = JobPayload::ScriptHash {
        hash: ScriptHash(123412),
        path: "f/system/hello".to_string(),
        cache_ttl: None,
        cache_ignore_s3_path: None,
        dedicated_worker: Some(true),
        language: ScriptLang::Bun,
        priority: None,
        apply_preprocessor: false,
        concurrency_settings: ConcurrencySettings::default(),
        debouncing_settings: DebouncingSettings::default(),
        labels: None,
        job_token_scopes: None,
    };
    assert_eq!(
        pushed_tag(&db, "staging", dedicated, None).await,
        "staging:f/system/hello"
    );
}
