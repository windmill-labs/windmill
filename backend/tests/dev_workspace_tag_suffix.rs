//! Sets the process-wide `DEV_WORKSPACE_ISOLATION`, so it lives in its own test binary: any other
//! test pushing a job in the same process would land on a suffixed tag.

use std::collections::HashMap;

use sqlx::{Pool, Postgres};
use windmill_common::{
    jobs::{JobPayload, RawCode},
    runnable_settings::{ConcurrencySettings, DebouncingSettings},
    scripts::{ScriptHash, ScriptLang},
    worker::{DEV_WORKSPACE_ISOLATION, DEV_WORKSPACE_ISOLATION_SKIP_NON_DEV_FORKS},
};
use windmill_queue::{PushArgs, PushIsolationLevel};

async fn pushed_tag(
    db: &Pool<Postgres>,
    workspace_id: &str,
    payload: JobPayload,
    tag: Option<&str>,
) -> String {
    try_pushed_tag(db, workspace_id, payload, tag)
        .await
        .expect("push must succeed")
}

async fn try_pushed_tag(
    db: &Pool<Postgres>,
    workspace_id: &str,
    payload: JobPayload,
    tag: Option<&str>,
) -> Result<String, windmill_common::error::Error> {
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
    .await?;
    tx.commit().await.unwrap();
    Ok(sqlx::query_scalar("SELECT tag FROM v2_job WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await
        .unwrap())
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

    // A dev workspace, a fork of it, a fork whose parent was deleted (`ON DELETE SET NULL`), and a
    // fork of the root with a fork of its own.
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id, is_dev_workspace) VALUES
            ('staging', 'staging', 'test-user', 'test-workspace', true),
            ('wm-fork-a', 'fork', 'test-user', 'staging', false),
            ('wm-fork-orphan', 'orphan', 'test-user', NULL, false),
            ('wm-fork-root', 'root fork', 'test-user', 'test-workspace', false),
            ('wm-fork-root-2', 'root fork 2', 'test-user', 'wm-fork-root', false)",
    )
    .execute(&db)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workspace_settings (workspace_id) VALUES
            ('staging'), ('wm-fork-a'), ('wm-fork-orphan'), ('wm-fork-root'), ('wm-fork-root-2')",
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
        ("wm-fork-root", None, "bash-dev"),
    ] {
        assert_eq!(
            pushed_tag(&db, workspace, preview(), tag).await,
            expected,
            "{workspace} with tag {tag:?}"
        );
    }

    // Only the forks whose lineage holds no dev workspace leave isolation; an orphaned fork's
    // lineage is unknown, so it stays.
    DEV_WORKSPACE_ISOLATION_SKIP_NON_DEV_FORKS.store(true, std::sync::atomic::Ordering::Relaxed);
    for (workspace, expected) in [
        ("wm-fork-root", "bash"),
        ("wm-fork-root-2", "bash"),
        ("staging", "bash-dev"),
        ("wm-fork-a", "bash-dev"),
        ("wm-fork-orphan", "bash-dev"),
    ] {
        assert_eq!(
            pushed_tag(&db, workspace, preview(), None).await,
            expected,
            "{workspace} with non-dev forks not isolated"
        );
    }
    DEV_WORKSPACE_ISOLATION_SKIP_NON_DEV_FORKS.store(false, std::sync::atomic::Ordering::Relaxed);

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

    // A hashed dedicated tag keeps only a prefix of its workspace id: a dev workspace sharing that
    // prefix does not get to use another workspace's dedicated worker.
    let root = "a".repeat(40);
    let dev = format!("{root}-dev");
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id, is_dev_workspace)
         VALUES ($1, 'root', 'test-user', NULL, false), ($2, 'dev', 'test-user', $1, true)",
    )
    .bind(&root)
    .bind(&dev)
    .execute(&db)
    .await
    .unwrap();
    sqlx::query("INSERT INTO workspace_settings (workspace_id) VALUES ($1), ($2)")
        .bind(&root)
        .bind(&dev)
        .execute(&db)
        .await
        .unwrap();
    let root_dedicated =
        windmill_common::worker::dedicated_worker_tag(&root, "f/some/long/script/path");
    assert!(
        try_pushed_tag(&db, &dev, preview(), Some(&root_dedicated))
            .await
            .is_err(),
        "{root_dedicated} must not stay unsuffixed, and suffixed it no longer fits a job tag"
    );
}
