use sqlx::{Pool, Postgres};
use windmill_common::queue::get_queue_stats;

const WORKSPACE: &str = "test-workspace";

async fn queue_job(
    db: &Pool<Postgres>,
    tag: &str,
    priority: Option<i16>,
    waited_secs: f64,
    running: bool,
) {
    sqlx::query(
        "WITH job AS (
            INSERT INTO v2_job (id, workspace_id, tag) VALUES (gen_random_uuid(), $1, $2)
            RETURNING id
        )
        INSERT INTO v2_job_queue (id, workspace_id, tag, priority, running, scheduled_for)
        SELECT id, $1, $2, $3, $4, now() - make_interval(secs => $5) FROM job",
    )
    .bind(WORKSPACE)
    .bind(tag)
    .bind(priority)
    .bind(running)
    .bind(waited_secs)
    .execute(db)
    .await
    .expect("failed to queue job");
}

/// The delay reported for a tag is that of the job the worker pull takes first, ordered
/// `priority DESC NULLS LAST, scheduled_for`, not simply the oldest one waiting. Running jobs
/// and jobs less than 3 seconds past due are not part of the backlog at all.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn queue_stats_report_the_delay_of_the_job_pulled_next(db: Pool<Postgres>) {
    // The oldest job has no priority, so every prioritized job runs before it.
    queue_job(&db, "mixed", None, 900.0, false).await;
    queue_job(&db, "mixed", Some(1), 600.0, false).await;
    queue_job(&db, "mixed", Some(5), 300.0, false).await;
    queue_job(&db, "mixed", Some(5), 100.0, false).await;
    // Highest priority, but not backlog: already running, or not yet 3 seconds past due.
    queue_job(&db, "mixed", Some(9), 1200.0, true).await;
    queue_job(&db, "mixed", Some(9), 1.0, false).await;
    queue_job(&db, "unprioritized", None, 500.0, false).await;
    queue_job(&db, "unprioritized", None, 50.0, false).await;

    let stats = get_queue_stats(&db).await.unwrap();

    let mixed = &stats["mixed"];
    assert_eq!(mixed.count, 4);
    assert!(
        (mixed.delay - 300.0).abs() < 5.0,
        "expected the oldest job of the highest priority, got a delay of {}",
        mixed.delay
    );
    let unprioritized = &stats["unprioritized"];
    assert_eq!(unprioritized.count, 2);
    assert!(
        (unprioritized.delay - 500.0).abs() < 5.0,
        "expected the oldest job, got a delay of {}",
        unprioritized.delay
    );
}
