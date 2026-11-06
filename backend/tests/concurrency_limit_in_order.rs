#![cfg(all(feature = "enterprise", feature = "private"))]

use sqlx::{Pool, Postgres};
use std::sync::atomic::Ordering;
use uuid::Uuid;
use windmill_common::worker::CONCURRENCY_LIMIT_IN_ORDER;
use windmill_queue::jobs_ee::update_concurrency_counter;

const KEY: &str = "in-order-key";

async fn queue_job(db: &Pool<Postgres>, due_secs_ago: i64) -> anyhow::Result<Uuid> {
    queue_job_on(db, due_secs_ago, "other", 0).await
}

async fn queue_job_on(
    db: &Pool<Postgres>,
    due_secs_ago: i64,
    tag: &str,
    priority: i16,
) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO v2_job (id, workspace_id) VALUES ($1, 'test-workspace')")
        .bind(id)
        .execute(db)
        .await?;
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag, priority)
         VALUES ($1, 'test-workspace', now() - make_interval(secs => $2), $3, $4)",
    )
    .bind(id)
    .bind(due_secs_ago as f64)
    .bind(tag)
    .bind(priority)
    .execute(db)
    .await?;
    sqlx::query("INSERT INTO concurrency_key (key, job_id) VALUES ($1, $2)")
        .bind(KEY)
        .bind(id)
        .execute(db)
        .await?;
    Ok(id)
}

/// Defers `id` an hour out, keeping its place in line, as `apply_concurrency_limit` does.
async fn defer(db: &Pool<Postgres>, id: Uuid) -> anyhow::Result<()> {
    sqlx::query(
        "WITH place AS (
            UPDATE concurrency_key ck
            SET original_scheduled_for = q.scheduled_for,
                place = COALESCE(ck.place, q.scheduled_for),
                tag = COALESCE(ck.tag, q.tag),
                priority = COALESCE(ck.priority, q.priority, -32768)
            FROM v2_job_queue q
            WHERE ck.job_id = $1 AND q.id = $1
        )
        UPDATE v2_job_queue SET scheduled_for = now() + interval '1 hour' WHERE id = $1",
    )
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

async fn admit(db: &Pool<Postgres>, id: Uuid) -> anyhow::Result<bool> {
    Ok(update_concurrency_counter(
        db,
        &id,
        KEY.to_string(),
        serde_json::json!({ id.to_string(): {} }),
        id.to_string(),
        0,
        1,
    )
    .await?
    .0)
}

/// A job deferred by the limit keeps its place: a free slot is refused to a job that
/// became due after it, and that refusal makes the deferred job due again.
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn free_slot_goes_to_the_job_first_in_line(db: Pool<Postgres>) -> anyhow::Result<()> {
    CONCURRENCY_LIMIT_IN_ORDER.store(true, Ordering::Relaxed);
    let first = queue_job(&db, 10).await?;
    let second = queue_job(&db, 5).await?;
    defer(&db, first).await?;

    let second_admitted = admit(&db, second).await;
    let first_due: bool =
        sqlx::query_scalar("SELECT scheduled_for <= now() FROM v2_job_queue WHERE id = $1")
            .bind(first)
            .fetch_one(&db)
            .await?;
    let first_admitted = admit(&db, first).await;
    // A job ahead of one that already holds the slot must not make it give the slot up
    // when it comes back through admission, as a resumed flow does.
    let earlier = queue_job(&db, 20).await?;
    defer(&db, earlier).await?;
    let first_readmitted = admit(&db, first).await;
    let earlier_admitted = admit(&db, earlier).await;
    CONCURRENCY_LIMIT_IN_ORDER.store(false, Ordering::Relaxed);

    assert!(!second_admitted?, "a later job took the slot");
    assert!(first_due, "the job first in line was not woken");
    assert!(
        first_admitted?,
        "the job first in line was refused a free slot"
    );
    assert!(first_readmitted?, "a job holding the slot was refused it");
    assert!(!earlier_admitted?, "a job was admitted past the limit");
    Ok(())
}

/// A completion in order refills every free slot from the front of the line, not one per
/// completion, so a woken job that is lost does not leave the key below its limit.
#[sqlx::test(fixtures("base"))]
async fn completion_fills_free_slots_from_the_front_of_the_line(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    // Queued first, then debounced a minute out without ever being pulled: it must keep waiting.
    let debounced = queue_job(&db, 40).await?;
    sqlx::query(
        "UPDATE v2_job_queue SET scheduled_for = now() + interval '1 minute' WHERE id = $1",
    )
    .bind(debounced)
    .execute(&db)
    .await?;
    let mut line = vec![];
    for due_secs_ago in [30, 20, 10] {
        let id = queue_job(&db, due_secs_ago).await?;
        defer(&db, id).await?;
        line.push(id);
    }

    windmill_queue::fill_concurrency_key_slots_in_order(&db, KEY, 2).await?;
    line.push(debounced);

    let mut due = vec![];
    for id in &line {
        due.push(
            sqlx::query_scalar::<_, bool>(
                "SELECT scheduled_for <= now() FROM v2_job_queue WHERE id = $1",
            )
            .bind(id)
            .fetch_one(&db)
            .await?,
        );
    }
    assert_eq!(
        due,
        vec![true, true, false, false],
        "the two jobs first in line were not both woken, or a debounced job was"
    );
    Ok(())
}

/// Each tag of a key has its own line, by priority then place, the order workers pull jobs in:
/// a later job of higher priority is ahead of an earlier one, and a job of another tag that no
/// worker picks up holds back neither the admission nor the wakes of this tag's line.
#[sqlx::test(fixtures("base"))]
#[serial_test::serial]
async fn lines_are_per_tag_by_priority_then_place(db: Pool<Postgres>) -> anyhow::Result<()> {
    CONCURRENCY_LIMIT_IN_ORDER.store(true, Ordering::Relaxed);
    // Woken a minute ago on a tag no worker serves: due, never pulled again. Its priority puts it
    // ahead of every other job, so only leaving out long-due jobs keeps it from taking the wake.
    let stuck = queue_job_on(&db, 60, "gpu", 9).await?;
    defer(&db, stuck).await?;
    sqlx::query(
        "UPDATE v2_job_queue SET scheduled_for = now() - interval '1 minute' WHERE id = $1",
    )
    .bind(stuck)
    .execute(&db)
    .await?;
    let low = queue_job_on(&db, 30, "other", 0).await?;
    defer(&db, low).await?;
    let urgent = queue_job_on(&db, 10, "other", 5).await?;
    defer(&db, urgent).await?;

    let low_admitted = admit(&db, low).await;
    windmill_queue::fill_concurrency_key_slots_in_order(&db, KEY, 1).await?;
    let urgent_woken: bool =
        sqlx::query_scalar("SELECT scheduled_for <= now() FROM v2_job_queue WHERE id = $1")
            .bind(urgent)
            .fetch_one(&db)
            .await?;
    let urgent_admitted = admit(&db, urgent).await;
    CONCURRENCY_LIMIT_IN_ORDER.store(false, Ordering::Relaxed);

    assert!(
        !low_admitted?,
        "an earlier job of lower priority took the slot"
    );
    assert!(urgent_woken, "the stuck job of another tag took the wake");
    assert!(
        urgent_admitted?,
        "the job first in its tag's line was held back by another tag"
    );
    Ok(())
}

/// A job woken a moment ago, not pulled yet, fills the slot it was woken for: a completion does
/// not wake the job behind it too, which admission would only refuse and defer again.
#[sqlx::test(fixtures("base"))]
async fn a_job_just_woken_fills_its_slot(db: Pool<Postgres>) -> anyhow::Result<()> {
    let woken = queue_job(&db, 20).await?;
    defer(&db, woken).await?;
    sqlx::query("UPDATE v2_job_queue SET scheduled_for = now() WHERE id = $1")
        .bind(woken)
        .execute(&db)
        .await?;
    let behind = queue_job(&db, 10).await?;
    defer(&db, behind).await?;

    windmill_queue::fill_concurrency_key_slots_in_order(&db, KEY, 1).await?;

    let behind_due: bool =
        sqlx::query_scalar("SELECT scheduled_for <= now() FROM v2_job_queue WHERE id = $1")
            .bind(behind)
            .fetch_one(&db)
            .await?;
    assert!(!behind_due, "a second job was woken for a slot the first already fills");
    Ok(())
}
