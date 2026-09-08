//! Pins the plan of the suspended-job pull. Its resume test degrades silently: once the
//! query expression and `queue_suspended_v2` stop matching, Postgres still returns the right
//! job, just by falling back to a heap filter and fetching one tuple per suspended row on
//! every worker poll. No functional test can see that, so assert on the plan instead.

use serde_json::Value;
use sqlx::{Pool, Postgres};
use windmill_common::worker::make_suspended_pull_query;

/// Depth-first walk of an `EXPLAIN (FORMAT JSON)` plan tree.
fn nodes(plan: &Value, out: &mut Vec<Value>) {
    out.push(plan.clone());
    for child in plan["Plans"].as_array().unwrap_or(&vec![]) {
        nodes(child, out);
    }
}

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
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&mut *conn)
        .await?;
    // FORMAT JSON rather than the default: `Index Cond` and `Filter` are separate keys on the
    // node, so this does not ride on EXPLAIN's line layout staying put across a major bump.
    let explained: Value = sqlx::query_scalar(&format!(
        "EXPLAIN (FORMAT JSON) {}",
        make_suspended_pull_query(&["flow".to_string()])
    ))
    .bind("test-worker")
    .fetch_one(&mut *conn)
    .await?;

    let mut all = vec![];
    nodes(&explained[0]["Plan"], &mut all);
    let pretty = serde_json::to_string_pretty(&explained)?;
    let scan = all
        .iter()
        .find(|n| n["Index Name"] == "queue_suspended_v2")
        .unwrap_or_else(|| {
            panic!("suspended pull did not scan queue_suspended_v2 on {version}:\n{pretty}")
        });
    // Only `Index Cond` is checked against the index tuple, so that is where the resume test
    // has to land — as a `Filter` it would cost a heap fetch per suspended row. The residual
    // `suspend_until IS NOT NULL` filter is not that: it is always true for rows the partial
    // index holds, and only ever runs on the row LIMIT 1 already fetched.
    let cond = scan["Index Cond"].as_str().unwrap_or_else(|| {
        panic!("no Index Cond on the suspended pull scan on {version}:\n{pretty}")
    });
    assert!(
        cond.contains("CASE WHEN"),
        "resume test is not an index condition on {version}:\n{pretty}"
    );
    assert!(
        !scan["Filter"].as_str().unwrap_or("").contains("CASE WHEN"),
        "resume test fell back to a heap filter on {version}:\n{pretty}"
    );
    Ok(())
}
