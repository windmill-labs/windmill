//! A workflow-as-code task child runs its parent's code with its parent's
//! arguments, so its result-cache key comes from what the dispatch seeded in its
//! checkpoint: the SDK's fingerprint of the task and the arguments the task was
//! called with, or, from an SDK that sends no fingerprint, its step key and the
//! parent's arguments.

use serde_json::{json, value::RawValue, Value};
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use uuid::Uuid;
use windmill_common::client::AuthedClient;
use windmill_common::jobs::JobKind;
use windmill_common::scripts::{ScriptHash, ScriptLang};
use windmill_queue::MiniPulledJob;
use windmill_worker::common::cached_result_path;

const W_ID: &str = "test-workspace";

async fn insert_job(db: &Pool<Postgres>, id: Uuid, parent: Option<Uuid>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, script_lang, runnable_path, tag, visible_to_owner, parent_job) \
         VALUES ($1, $2, 'test-user', now(), 'u/test-user', 'test@windmill.dev', \
            'flowscript', 'bun', 'f/system/wac/a', 'bun', true, $3)",
    )
    .bind(id)
    .bind(W_ID)
    .bind(parent)
    .execute(db)
    .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag) \
         VALUES ($1, $2, now(), true, 'bun')",
    )
    .bind(id)
    .bind(W_ID)
    .execute(db)
    .await?;
    Ok(())
}

/// The cache path of a task child carrying `parent_args`, whose checkpoint was
/// seeded with step key `key`, fingerprint `fn_id` and call arguments `{n}`.
async fn child_cache_path(
    db: &Pool<Postgres>,
    parent_args: Value,
    key: &str,
    fn_id: Option<&str>,
    n: i64,
) -> anyhow::Result<String> {
    let parent = Uuid::new_v4();
    let child = Uuid::new_v4();
    insert_job(db, parent, None).await?;
    insert_job(db, child, Some(parent)).await?;
    let mut checkpoint = json!({
        "completed_steps": {},
        "_executing_key": key,
        "_executing_args": { "n": n },
    });
    if let Some(fn_id) = fn_id {
        checkpoint["_executing_fn"] = json!(fn_id);
    }
    sqlx::query("INSERT INTO v2_job_status (id, workflow_as_code_status) VALUES ($1, $2)")
        .bind(child)
        .bind(json!({ "_checkpoint": checkpoint }))
        .execute(db)
        .await?;

    let args: HashMap<String, Box<RawValue>> = serde_json::from_str(&parent_args.to_string())?;
    let mut job = MiniPulledJob::new_inline(
        W_ID.to_string(),
        Some(args),
        "test-user".to_string(),
        "u/test-user".to_string(),
        "test@windmill.dev".to_string(),
        Some("f/system/wac/a".to_string()),
        JobKind::FlowScript,
        Some(ScriptHash(42)),
        "bun".to_string(),
        Some(ScriptLang::Bun),
    );
    job.id = child;
    job.parent_job = Some(parent);
    job.cache_ttl = Some(60);
    let client = AuthedClient::new(
        "http://localhost".to_string(),
        W_ID.to_string(),
        "tok".to_string(),
        None,
    );
    Ok(cached_result_path(db, &client, &job, None).await?)
}

#[sqlx::test(fixtures("base"))]
async fn a_task_child_is_cached_under_its_fingerprint_or_else_its_step_key(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let parent_args = || json!({ "x": 1 });

    let fingerprinted = child_cache_path(&db, parent_args(), "step", Some("f1"), 1).await?;
    assert_ne!(
        fingerprinted,
        child_cache_path(&db, parent_args(), "step", Some("f2"), 1).await?,
        "two tasks called at one position: the fingerprint tells them apart"
    );
    assert_eq!(
        fingerprinted,
        child_cache_path(&db, parent_args(), "step", Some("f1"), 1).await?,
        "one task at one step with one set of arguments is one entry"
    );
    assert_ne!(
        fingerprinted,
        child_cache_path(&db, parent_args(), "step_2", Some("f1"), 1).await?,
        "the step key stays in the key: a fingerprint cannot separate two bound \
         functions of one name, or two lambdas sharing a source line"
    );
    assert_eq!(
        fingerprinted,
        child_cache_path(&db, json!({ "x": 2 }), "step", Some("f1"), 1).await?,
        "with a fingerprint, the parent's arguments are not in the key"
    );
    assert_ne!(
        fingerprinted,
        child_cache_path(&db, parent_args(), "step", Some("f1"), 2).await?,
        "and the task's own arguments are"
    );

    let unfingerprinted = child_cache_path(&db, parent_args(), "step", None, 1).await?;
    assert_ne!(
        unfingerprinted,
        child_cache_path(&db, parent_args(), "step_2", None, 1).await?,
        "without a fingerprint, the step key is the identity"
    );
    assert_ne!(
        unfingerprinted,
        child_cache_path(&db, json!({ "x": 2 }), "step", None, 1).await?,
        "and the parent's arguments are in the key"
    );
    assert_eq!(
        unfingerprinted,
        child_cache_path(&db, parent_args(), "step", None, 2).await?,
        "and the task's own arguments are not"
    );
    assert_ne!(
        child_cache_path(&db, parent_args(), "f1", None, 1).await?,
        fingerprinted,
        "a step key never reads as a fingerprint"
    );
    Ok(())
}
