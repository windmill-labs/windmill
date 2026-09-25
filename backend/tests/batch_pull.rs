//! Pins what `pull_batch` promises a caller serving many waiting workers from one claim: every
//! claimed job is marked running under exactly one of those workers, concurrent batches never
//! claim the same job, the queues are walked in the same order a single pull walks them, and
//! the batch peek keeps the ordered index scan the single pull relies on.

use std::collections::HashSet;

use serde_json::Value;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::worker::{make_batch_pull_query, PullQueue};
use windmill_queue::{pull_batch, PulledJobResult};

const W_ID: &str = "test-workspace";

async fn queue_job(db: &Pool<Postgres>, tag: &str, suspend: Option<i32>) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, created_at, permissioned_as, \
            permissioned_as_email, kind, tag, args, visible_to_owner) \
         VALUES ($1, $2, 'test-user', now(), 'u/test-user', 'test@windmill.dev', 'noop', $3, \
            '{}', true)",
    )
    .bind(id)
    .bind(W_ID)
    .bind(tag)
    .execute(db)
    .await?;
    // A suspended flow sits running with `suspend_until` set; `suspend = 0` makes it resumable.
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag, suspend, \
            suspend_until) \
         VALUES ($1, $2, now() - interval '1 second', $3, $4, COALESCE($5, 0), \
            CASE WHEN $5::int IS NULL THEN NULL ELSE now() + interval '1 day' END)",
    )
    .bind(id)
    .bind(W_ID)
    .bind(suspend.is_some())
    .bind(tag)
    .bind(suspend)
    .execute(db)
    .await?;
    sqlx::query("INSERT INTO v2_job_runtime (id) VALUES ($1)")
        .bind(id)
        .execute(db)
        .await?;
    Ok(id)
}

fn names(prefix: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("{prefix}-{i}")).collect()
}

fn job_id(res: &PulledJobResult) -> Uuid {
    res.job
        .as_ref()
        .expect("an admitted result carries its job")
        .id
}

#[sqlx::test(fixtures("base"))]
async fn batch_pull_assigns_each_job_to_one_waiting_worker(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    for _ in 0..30 {
        queue_job(&db, "batch", None).await?;
    }
    let groups = vec![vec!["batch".to_string()]];

    // Three batches race over one queue; a name listed twice still gets one job.
    let mut a = names("a", 8);
    a.push("a-0".to_string());
    let (b, c) = (names("b", 8), names("c", 8));
    let (ra, rb, rc) = tokio::join!(
        pull_batch(&db, &groups, &a, false),
        pull_batch(&db, &groups, &b, false),
        pull_batch(&db, &groups, &c, false),
    );
    let claimed: Vec<(String, PulledJobResult)> = [ra?, rb?, rc?].into_iter().flatten().collect();
    assert_eq!(
        claimed.len(),
        24,
        "30 queued jobs cover all 24 distinct workers"
    );
    let workers: HashSet<&String> = claimed.iter().map(|(w, _)| w).collect();
    let jobs: HashSet<Uuid> = claimed.iter().map(|(_, r)| job_id(r)).collect();
    assert_eq!(workers.len(), 24, "no worker is handed two jobs");
    assert_eq!(jobs.len(), 24, "no job is handed to two workers");

    for (worker, res) in &claimed {
        let (running, row_worker): (bool, Option<String>) =
            sqlx::query_as("SELECT running, worker FROM v2_job_queue WHERE id = $1")
                .bind(job_id(res))
                .fetch_one(&db)
                .await?;
        assert!(running);
        assert_eq!(
            row_worker.as_ref(),
            Some(worker),
            "job is running under its worker"
        );
    }

    // The result is the wire type a remote caller hands to its worker.
    let (_, first) = &claimed[0];
    let round_trip: PulledJobResult = serde_json::from_str(&serde_json::to_string(first)?)?;
    assert_eq!(job_id(&round_trip), job_id(first));

    // Six jobs remain for ten workers: four go without.
    let rest = pull_batch(&db, &groups, &names("d", 10), false).await?;
    assert_eq!(rest.len(), 6);
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn batch_pull_walks_suspended_then_priority_groups(db: Pool<Postgres>) -> anyhow::Result<()> {
    let resumable = queue_job(&db, "low", Some(0)).await?;
    let mut high = HashSet::new();
    for _ in 0..3 {
        high.insert(queue_job(&db, "high", None).await?);
        queue_job(&db, "low", None).await?;
    }
    let groups = vec![vec!["high".to_string()], vec!["low".to_string()]];

    let claimed = pull_batch(&db, &groups, &names("w", 5), true).await?;
    assert_eq!(claimed.len(), 5);
    let suspended: Vec<Uuid> = claimed
        .iter()
        .filter(|(_, r)| r.suspended)
        .map(|(_, r)| job_id(r))
        .collect();
    assert_eq!(
        suspended,
        vec![resumable],
        "the resumable flow is claimed first"
    );
    let ready: HashSet<Uuid> = claimed
        .iter()
        .filter(|(_, r)| !r.suspended)
        .map(|(_, r)| job_id(r))
        .collect();
    assert!(
        high.is_subset(&ready),
        "the high group drains before the low one"
    );
    assert_eq!(ready.len(), 4);
    Ok(())
}

// A job over its concurrency limit must go back to the queue rather than sit claimed, and the
// worker it was claimed for must still be served from the rest of the queue.
#[cfg(feature = "enterprise")]
#[sqlx::test(fixtures("base"))]
async fn batch_pull_requeues_over_limit_job_and_serves_its_worker(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    let limited = queue_job(&db, "batch", None).await?;
    sqlx::query(
        "UPDATE v2_job SET kind = 'script', runnable_path = 'f/test/limited', \
            concurrent_limit = 1, concurrency_time_window_s = 0 WHERE id = $1",
    )
    .bind(limited)
    .execute(&db)
    .await?;
    // Claimed first, so the bounce happens before the free job is reached.
    sqlx::query("UPDATE v2_job_queue SET priority = 10 WHERE id = $1")
        .bind(limited)
        .execute(&db)
        .await?;
    sqlx::query("INSERT INTO concurrency_key (key, job_id) VALUES ('limited-key', $1)")
        .bind(limited)
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO concurrency_counter (concurrency_id, job_uuids) \
         VALUES ('limited-key', jsonb_build_object(gen_random_uuid()::text, '{}'::jsonb))",
    )
    .execute(&db)
    .await?;
    let free = queue_job(&db, "batch", None).await?;

    let groups = vec![vec!["batch".to_string()]];
    let claimed = pull_batch(&db, &groups, &names("w", 1), false).await?;
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].0, "w-0");
    assert_eq!(
        job_id(&claimed[0].1),
        free,
        "the worker is served the job under no limit"
    );

    let (running, rescheduled): (bool, bool) =
        sqlx::query_as("SELECT running, scheduled_for > now() FROM v2_job_queue WHERE id = $1")
            .bind(limited)
            .fetch_one(&db)
            .await?;
    assert!(!running, "the over-limit job is back in the queue");
    assert!(rescheduled, "the over-limit job waits for a free slot");
    Ok(())
}

/// Depth-first walk of an `EXPLAIN (FORMAT JSON)` plan tree.
fn nodes(plan: &Value, out: &mut Vec<Value>) {
    out.push(plan.clone());
    for child in plan["Plans"].as_array().unwrap_or(&vec![]) {
        nodes(child, out);
    }
}

// The batch LIMIT is a bind parameter, so a cached generic plan cannot see it and the planner
// falls back to guessing a fraction of the matching rows. On a deep backlog, a guess that tips
// it into a bitmap scan plus sort reads every queued job of the tag on each pull.
#[sqlx::test(fixtures("base"))]
async fn batch_pull_generic_plan_walks_the_queue_index(db: Pool<Postgres>) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, running, tag)
         SELECT gen_random_uuid(), 'test-workspace', now() - make_interval(secs => i % 1000),
                false, CASE WHEN i % 10 = 0 THEN 'python3' ELSE 'deno' END
         FROM generate_series(1, 200000) i",
    )
    .execute(&db)
    .await?;
    sqlx::query("ANALYZE v2_job_queue").execute(&db).await?;

    let mut conn = db.acquire().await?;
    sqlx::query("SET plan_cache_mode = force_generic_plan")
        .execute(&mut *conn)
        .await?;
    let query = make_batch_pull_query(&["python3".to_string()], PullQueue::Ready);
    sqlx::query(&format!("PREPARE batch_pull(text[]) AS {query}"))
        .execute(&mut *conn)
        .await?;
    let explained: Value =
        sqlx::query_scalar("EXPLAIN (FORMAT JSON) EXECUTE batch_pull(ARRAY['w-0', 'w-1'])")
            .fetch_one(&mut *conn)
            .await?;

    let mut all = vec![];
    nodes(&explained[0]["Plan"], &mut all);
    let pretty = serde_json::to_string_pretty(&explained)?;
    assert!(
        all.iter()
            .any(|n| n["Node Type"] == "Index Scan" && n["Index Name"] == "queue_sort_v2"),
        "batch peek does not walk queue_sort_v2 in order:\n{pretty}"
    );
    assert!(
        !all.iter().any(|n| n["Node Type"] == "Sort"),
        "batch peek sorts the backlog:\n{pretty}"
    );
    Ok(())
}
