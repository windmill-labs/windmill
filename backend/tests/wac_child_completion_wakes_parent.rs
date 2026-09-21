//! A WAC v2 parent parks on its dispatched children and is woken by their
//! completions. A child does not always complete through the worker that ran it:
//! the zombie monitor and a force cancel both go straight to
//! `add_completed_job_error`. The parent must be woken from there too, or it sits
//! out its whole suspend window and then runs the task a second time.

use serde_json::{json, Value};
use sqlx::{types::Json, Pool, Postgres};
use uuid::Uuid;
use windmill_queue::{add_completed_job, add_completed_job_error, get_mini_completed_job};

const W_ID: &str = "test-workspace";

async fn insert_job(db: &Pool<Postgres>, id: Uuid, parent: Option<Uuid>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, script_lang, runnable_path, tag, visible_to_owner, parent_job) \
         VALUES ($1, $2, 'test-user', now(), 'u/test-user', 'test@windmill.dev', \
            'script', 'bun', 'u/test-user/wac', 'bun', true, $3)",
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

/// A parent parked on `steps` (step key → child job), the shape
/// `handle_wac_v2_output` leaves behind once the children are pushed.
async fn plant_parked_parent(db: &Pool<Postgres>, steps: &[(&str, Uuid)]) -> anyhow::Result<Uuid> {
    let parent = Uuid::new_v4();
    insert_job(db, parent, None).await?;
    sqlx::query(
        "UPDATE v2_job_queue SET suspend = $2, suspend_until = now() + interval '14 days' \
         WHERE id = $1",
    )
    .bind(parent)
    .bind(steps.len() as i32)
    .execute(db)
    .await?;
    let job_ids: serde_json::Map<String, Value> = steps
        .iter()
        .map(|(k, id)| (k.to_string(), json!(id.to_string())))
        .collect();
    let keys: Vec<&str> = steps.iter().map(|(k, _)| *k).collect();
    sqlx::query("INSERT INTO v2_job_status (id, workflow_as_code_status) VALUES ($1, $2)")
        .bind(parent)
        .bind(json!({
            "_checkpoint": {
                "completed_steps": {},
                "pending_steps": { "mode": "dispatch", "keys": keys, "job_ids": job_ids },
                "job_ids": job_ids,
            }
        }))
        .execute(db)
        .await?;
    for (_, child) in steps {
        insert_job(db, *child, Some(parent)).await?;
    }
    Ok(parent)
}

async fn parent_state(db: &Pool<Postgres>, parent: Uuid) -> anyhow::Result<(i32, bool, Value)> {
    let (suspend, parked, status): (i32, bool, Value) = sqlx::query_as(
        "SELECT q.suspend, q.suspend_until IS NOT NULL, s.workflow_as_code_status \
         FROM v2_job_queue q JOIN v2_job_status s USING (id) WHERE q.id = $1",
    )
    .bind(parent)
    .fetch_one(db)
    .await?;
    Ok((suspend, parked, status))
}

/// The zombie monitor's path: `handle_job_error` → `add_completed_job_error`, never
/// the worker's result processor. The parent must come out of it pullable, with the
/// failure recorded under the step so the workflow's `try/catch` sees a task error.
#[sqlx::test(fixtures("base"))]
async fn a_child_failed_outside_the_worker_wakes_its_parent(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let child = Uuid::new_v4();
    let parent = plant_parked_parent(&db, &[("slowTask", child)]).await?;
    let child_job = get_mini_completed_job(&child, W_ID, &db).await?.unwrap();

    add_completed_job_error(
        &db,
        &child_job,
        0,
        None,
        json!({"name": "ExecutionErr", "message": "Job timed out after no ping"}),
        "monitor",
        false,
        None,
    )
    .await?;

    let (suspend, parked, status) = parent_state(&db, parent).await?;
    assert_eq!(suspend, 0, "the parent must be released");
    assert!(
        parked,
        "suspend_until stays set: the suspended pull query keys on it"
    );
    let step = &status["_checkpoint"]["completed_steps"]["slowTask"];
    assert_eq!(step["__wmill_error"], json!(true), "{status}");
    assert_eq!(step["child_job_id"], json!(child.to_string()));
    assert_eq!(
        step["result"]["error"]["message"],
        json!("Job timed out after no ping")
    );
    assert!(
        status["_checkpoint"].get("pending_steps").is_none(),
        "nothing left to wait on: {status}"
    );
    assert!(
        windmill_common::wac::WAC_SUSPEND_READY.swap(false, std::sync::atomic::Ordering::Relaxed)
    );
    Ok(())
}

/// Only a child the parent is waiting on moves the counter. A child the body
/// launched itself, or a completion arriving after the key was re-dispatched to
/// another job, records its timeline entry and nothing else.
#[sqlx::test(fixtures("base"))]
async fn a_child_the_parent_is_not_waiting_on_leaves_it_parked(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let awaited = Uuid::new_v4();
    let parent = plant_parked_parent(&db, &[("task", awaited)]).await?;
    let stray = Uuid::new_v4();
    insert_job(&db, stray, Some(parent)).await?;

    let stray_job = get_mini_completed_job(&stray, W_ID, &db).await?.unwrap();
    add_completed_job_error(
        &db,
        &stray_job,
        0,
        None,
        json!({"message": "boom"}),
        "w",
        false,
        None,
    )
    .await?;

    let (suspend, _, status) = parent_state(&db, parent).await?;
    assert_eq!(
        suspend, 1,
        "a stray child must not release the parent: {status}"
    );
    assert_eq!(status["_checkpoint"]["completed_steps"], json!({}));
    assert!(
        status[stray.to_string()]["duration_ms"].is_number(),
        "the timeline entry is still stamped: {status}"
    );

    let awaited_job = get_mini_completed_job(&awaited, W_ID, &db).await?.unwrap();
    let result = serde_json::value::to_raw_value(&json!("done"))?;
    add_completed_job(
        &db,
        &awaited_job,
        true,
        false,
        Json(&result),
        None,
        0,
        None,
        false,
        None,
        false,
    )
    .await?;

    let (suspend, _, status) = parent_state(&db, parent).await?;
    assert_eq!(suspend, 0);
    assert_eq!(
        status["_checkpoint"]["completed_steps"]["task"],
        json!("done")
    );
    Ok(())
}
