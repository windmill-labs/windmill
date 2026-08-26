//! Pins the plan of the suspended-job pull. Its resume test degrades silently: once the
//! query expression and `queue_suspended_v2` stop matching, Postgres still returns the right
//! job, just by falling back to a heap filter and fetching one tuple per suspended row on
//! every worker poll. No functional test can see that, so assert on the plan instead.

use sqlx::{Pool, Postgres};
use windmill_common::worker::make_suspended_pull_query;

#[sqlx::test(fixtures("base"))]
async fn suspended_pull_tests_resume_time_inside_the_index(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, created_at, scheduled_for, running, suspend, suspend_until, tag)
         SELECT gen_random_uuid(), 'test-workspace', now() - make_interval(secs => i),
                now(), true, 1 + (i % 3), now() + interval '7 day', 'flow'
         FROM generate_series(1, 2000) i",
    )
    .execute(&db)
    .await?;
    sqlx::query("ANALYZE v2_job_queue").execute(&db).await?;

    // Both plans are cheap on a 2000-row table, and which one wins there says nothing
    // about a queue with a large suspended backlog. Force the index path, which is the
    // one production takes, and assert on how it evaluates the resume test.
    let mut conn = db.acquire().await?;
    sqlx::query("SET enable_seqscan = off")
        .execute(&mut *conn)
        .await?;
    let plan: Vec<String> = sqlx::query_scalar(&format!(
        "EXPLAIN {}",
        make_suspended_pull_query(&["flow".to_string()])
    ))
    .bind("test-worker")
    .fetch_all(&mut *conn)
    .await?;
    let plan = plan.join("\n");

    let scan = plan
        .lines()
        .position(|l| l.contains("Index Scan using queue_suspended_v2 on v2_job_queue"))
        .unwrap_or_else(|| panic!("suspended pull did not scan queue_suspended_v2:\n{plan}"));
    // The node's own qual lines run until the next node, and only `Index Cond` is checked
    // against the index tuple — a `Filter` is what costs the heap fetch per row.
    let quals: Vec<&str> = plan
        .lines()
        .skip(scan + 1)
        .take_while(|l| !l.contains("->"))
        .collect();
    let cond = quals
        .iter()
        .find(|l| l.trim_start().starts_with("Index Cond:"))
        .unwrap_or_else(|| panic!("no Index Cond on the suspended pull scan:\n{plan}"));
    assert!(
        cond.contains("CASE WHEN") && cond.contains("suspend_until IS NOT NULL"),
        "resume test is not an index condition:\n{plan}"
    );
    assert!(
        !quals.iter().any(|l| l.trim_start().starts_with("Filter:")),
        "suspended pull fell back to a heap filter:\n{plan}"
    );
    Ok(())
}
