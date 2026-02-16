/// Integration benchmark comparing old (in-memory UUID) vs new (jobs_pending_deletion table)
/// job cleanup approaches.
///
/// Both methods scope their candidate selection to a specific `tag` so that each run
/// operates on an isolated set of jobs. This ensures fair comparison regardless of other
/// expired jobs in the database.
///
/// Prerequisites:
///   - PostgreSQL running with windmill database (migrations applied)
///   - DATABASE_URL env var or default: postgres://postgres:changeme@localhost:5432/windmill
///
/// Run comparison benchmark:
///   cargo test -p windmill-queue --test job_cleanup_test test_compare_cleanup_methods -- --nocapture --ignored
///
/// Run individual tests:
///   cargo test -p windmill-queue --test job_cleanup_test test_old_method -- --nocapture --ignored
///   cargo test -p windmill-queue --test job_cleanup_test test_new_method -- --nocapture --ignored
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::time::Instant;
use uuid::Uuid;

const DEFAULT_DB_URL: &str = "postgres://postgres:changeme@localhost:5432/windmill";
const DEFAULT_JOB_COUNT: usize = 50_000;

async fn connect() -> Pool<Postgres> {
    let url = std::env::var("DATABASE_URL").unwrap_or(DEFAULT_DB_URL.to_string());
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("Failed to connect to database")
}

async fn insert_test_jobs(db: &Pool<Postgres>, count: usize, tag: &str) {
    for chunk in (0..count).collect::<Vec<_>>().chunks(1000) {
        let chunk_ids: Vec<Uuid> = chunk.iter().map(|_| Uuid::new_v4()).collect();

        for id in &chunk_ids {
            sqlx::query!(
                "INSERT INTO v2_job (id, tag, workspace_id, created_at, created_by, permissioned_as, permissioned_as_email, kind, same_worker, visible_to_owner)
                 VALUES ($1, $2, 'admins', now() - interval '30 minutes', $3, $3, 'bench@test.com', 'script', false, true)",
                id,
                tag,
                "bench_user"
            )
            .execute(db)
            .await
            .expect("Failed to insert v2_job");
        }

        for id in &chunk_ids {
            sqlx::query!(
                "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, deleted, status, completed_at, started_at)
                 VALUES ($1, 'admins', 100, false, 'success', now() - interval '30 minutes', now() - interval '30 minutes')",
                id
            )
            .execute(db)
            .await
            .expect("Failed to insert v2_job_completed");
        }

        for id in &chunk_ids {
            sqlx::query!(
                "INSERT INTO job_stats (job_id, workspace_id) VALUES ($1, 'admins')
                 ON CONFLICT DO NOTHING",
                id,
            )
            .execute(db)
            .await
            .ok();
        }

        for id in &chunk_ids {
            sqlx::query!(
                "INSERT INTO job_logs (job_id, workspace_id, logs) VALUES ($1, 'admins', 'test log output')
                 ON CONFLICT DO NOTHING",
                id,
            )
            .execute(db)
            .await
            .ok();
        }
    }
}

async fn cleanup_test_data(db: &Pool<Postgres>, tag: &str) {
    sqlx::query!("DELETE FROM jobs_pending_deletion")
        .execute(db)
        .await
        .ok();
    sqlx::query!(
        "DELETE FROM job_stats WHERE job_id IN (SELECT id FROM v2_job WHERE created_by = $1 AND tag = $2)",
        "bench_user",
        tag
    )
    .execute(db)
    .await
    .ok();
    sqlx::query!(
        "DELETE FROM job_logs WHERE job_id IN (SELECT id FROM v2_job WHERE created_by = $1 AND tag = $2)",
        "bench_user",
        tag
    )
    .execute(db)
    .await
    .ok();
    sqlx::query!(
        "DELETE FROM v2_job_completed WHERE id IN (SELECT id FROM v2_job WHERE created_by = $1 AND tag = $2)",
        "bench_user",
        tag
    )
    .execute(db)
    .await
    .ok();
    sqlx::query!(
        "DELETE FROM v2_job WHERE created_by = $1 AND tag = $2",
        "bench_user",
        tag
    )
    .execute(db)
    .await
    .ok();
}

/// Old method: single transaction, DELETE RETURNING into Vec<Uuid>, then ANY($1) cascades.
/// Scoped to jobs with the given `tag` for isolation.
async fn run_old_method(
    db: &Pool<Postgres>,
    retention_secs: i64,
    batch_size: i64,
    tag: &str,
) -> (usize, std::time::Duration) {
    let start = Instant::now();
    let mut total = 0usize;

    loop {
        let batch_start = Instant::now();
        let mut tx = db.begin().await.unwrap();

        let active_roots: Vec<Uuid> = sqlx::query_scalar!(
            "SELECT q.id FROM v2_job_queue q
             JOIN v2_job j ON j.id = q.id
             WHERE j.parent_job IS NULL
               AND j.created_at <= now() - ($1::bigint::text || ' s')::interval",
            retention_secs
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap();

        let deleted_jobs: Vec<Uuid> = sqlx::query_scalar!(
            "DELETE FROM v2_job_completed
             WHERE id IN (
                 SELECT jc.id FROM v2_job_completed jc
                 JOIN v2_job j ON j.id = jc.id
                 WHERE jc.completed_at <= now() - ($1::bigint::text || ' s')::interval
                   AND COALESCE(j.root_job, j.flow_innermost_root_job, jc.id) != ALL($3)
                   AND j.tag = $4
                 ORDER BY jc.completed_at ASC
                 LIMIT $2
                 FOR UPDATE OF jc SKIP LOCKED
             )
             RETURNING id",
            retention_secs,
            batch_size,
            &active_roots,
            tag
        )
        .fetch_all(&mut *tx)
        .await
        .unwrap();

        let count = deleted_jobs.len();
        if count == 0 {
            tx.commit().await.unwrap();
            break;
        }

        sqlx::query!("DELETE FROM job_stats WHERE job_id = ANY($1)", &deleted_jobs)
            .execute(&mut *tx)
            .await
            .ok();
        sqlx::query!("DELETE FROM job_logs WHERE job_id = ANY($1)", &deleted_jobs)
            .execute(&mut *tx)
            .await
            .ok();
        sqlx::query!("DELETE FROM v2_job WHERE id = ANY($1)", &deleted_jobs)
            .execute(&mut *tx)
            .await
            .ok();
        sqlx::query!(
            "DELETE FROM job_result_stream_v2 WHERE job_id = ANY($1)",
            &deleted_jobs
        )
        .execute(&mut *tx)
        .await
        .ok();

        tx.commit().await.unwrap();
        total += count;

        let rate = count as f64 / batch_start.elapsed().as_secs_f64();
        println!(
            "  [old] batch: {} jobs in {:?} ({:.0} jobs/sec)",
            count,
            batch_start.elapsed(),
            rate
        );
    }

    (total, start.elapsed())
}

/// New method: INSERT INTO jobs_pending_deletion, then per-table DELETE USING joins.
/// Scoped to jobs with the given `tag` for isolation.
async fn run_new_method(
    db: &Pool<Postgres>,
    retention_secs: i64,
    batch_size: i64,
    tag: &str,
) -> (usize, std::time::Duration) {
    let start = Instant::now();
    let mut total = 0usize;
    let instance = "bench";

    loop {
        let batch_start = Instant::now();

        let active_roots: Vec<Uuid> = sqlx::query_scalar!(
            "SELECT q.id FROM v2_job_queue q
             JOIN v2_job j ON j.id = q.id
             WHERE j.parent_job IS NULL
               AND j.created_at <= now() - ($1::bigint::text || ' s')::interval",
            retention_secs
        )
        .fetch_all(db)
        .await
        .unwrap();

        let result = sqlx::query!(
            "INSERT INTO jobs_pending_deletion (id, marked_by)
             SELECT jc.id, $5 FROM v2_job_completed jc
             JOIN v2_job j ON j.id = jc.id
             WHERE jc.completed_at <= now() - ($1::bigint::text || ' s')::interval
               AND COALESCE(j.root_job, j.flow_innermost_root_job, jc.id) != ALL($3)
               AND j.tag = $4
             ORDER BY jc.completed_at ASC
             LIMIT $2
             ON CONFLICT DO NOTHING",
            retention_secs,
            batch_size,
            &active_roots,
            tag,
            instance
        )
        .execute(db)
        .await
        .unwrap();

        let count = result.rows_affected() as usize;
        if count == 0 {
            break;
        }

        sqlx::query!(
            "DELETE FROM job_stats USING jobs_pending_deletion d
             WHERE job_stats.job_id = d.id AND d.marked_by = $1",
            instance
        )
        .execute(db)
        .await
        .ok();
        sqlx::query!(
            "DELETE FROM job_logs USING jobs_pending_deletion d
             WHERE job_logs.job_id = d.id AND d.marked_by = $1",
            instance
        )
        .execute(db)
        .await
        .ok();
        sqlx::query!(
            "DELETE FROM v2_job_completed USING jobs_pending_deletion d
             WHERE v2_job_completed.id = d.id AND d.marked_by = $1",
            instance
        )
        .execute(db)
        .await
        .ok();
        sqlx::query!(
            "DELETE FROM v2_job USING jobs_pending_deletion d
             WHERE v2_job.id = d.id AND d.marked_by = $1",
            instance
        )
        .execute(db)
        .await
        .ok();
        sqlx::query!(
            "DELETE FROM job_result_stream_v2 USING jobs_pending_deletion d
             WHERE job_result_stream_v2.job_id = d.id AND d.marked_by = $1",
            instance
        )
        .execute(db)
        .await
        .ok();
        sqlx::query!(
            "DELETE FROM jobs_pending_deletion WHERE marked_by = $1",
            instance
        )
        .execute(db)
        .await
        .ok();

        total += count;

        let rate = count as f64 / batch_start.elapsed().as_secs_f64();
        println!(
            "  [new] batch: {} jobs in {:?} ({:.0} jobs/sec)",
            count,
            batch_start.elapsed(),
            rate
        );
    }

    (total, start.elapsed())
}

#[tokio::test]
#[ignore]
async fn test_compare_cleanup_methods() {
    let db = connect().await;
    let job_count = std::env::var("BENCH_JOB_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_JOB_COUNT);

    let retention_secs: i64 = 600;
    let batch_sizes: &[i64] = &[1000, 5000, 10000, 20000];

    for &batch_size in batch_sizes {
        println!(
            "\n========== batch_size={}, job_count={} ==========",
            batch_size, job_count
        );

        let tag_old = format!("bench_old_{}", batch_size);
        let tag_new = format!("bench_new_{}", batch_size);

        // Clean slate for both tags
        cleanup_test_data(&db, &tag_old).await;
        cleanup_test_data(&db, &tag_new).await;

        // --- Old method ---
        println!("[old] Inserting {} test jobs...", job_count);
        insert_test_jobs(&db, job_count, &tag_old).await;
        println!("[old] Running old method (in-memory UUID, single tx)...");
        let (old_total, old_dur) = run_old_method(&db, retention_secs, batch_size, &tag_old).await;
        let old_rate = if old_dur.as_secs_f64() > 0.0 {
            old_total as f64 / old_dur.as_secs_f64()
        } else {
            0.0
        };
        println!(
            "[old] Deleted {} jobs in {:?} ({:.0} jobs/sec)",
            old_total, old_dur, old_rate
        );
        cleanup_test_data(&db, &tag_old).await;

        // --- New method ---
        println!("[new] Inserting {} test jobs...", job_count);
        insert_test_jobs(&db, job_count, &tag_new).await;
        println!("[new] Running new method (jobs_pending_deletion table)...");
        let (new_total, new_dur) = run_new_method(&db, retention_secs, batch_size, &tag_new).await;
        let new_rate = if new_dur.as_secs_f64() > 0.0 {
            new_total as f64 / new_dur.as_secs_f64()
        } else {
            0.0
        };
        println!(
            "[new] Deleted {} jobs in {:?} ({:.0} jobs/sec)",
            new_total, new_dur, new_rate
        );
        cleanup_test_data(&db, &tag_new).await;

        println!("\n--- Summary (batch_size={}) ---", batch_size);
        println!(
            "  Old: {} jobs in {:?} ({:.0} jobs/sec)",
            old_total, old_dur, old_rate
        );
        println!(
            "  New: {} jobs in {:?} ({:.0} jobs/sec)",
            new_total, new_dur, new_rate
        );
        if old_total != new_total {
            println!(
                "  WARNING: job counts differ (old={}, new={})",
                old_total, new_total
            );
        }
        if old_dur.as_secs_f64() > 0.0 && new_dur.as_secs_f64() > 0.0 {
            let speedup = old_dur.as_secs_f64() / new_dur.as_secs_f64();
            println!("  Speedup: {:.2}x", speedup);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_old_method() {
    let db = connect().await;
    let retention_secs: i64 = 600;
    let batch_size: i64 = 10000;
    let job_count = 10_000;
    let tag = "bench_old_solo";

    cleanup_test_data(&db, tag).await;
    println!("Inserting {} test jobs...", job_count);
    insert_test_jobs(&db, job_count, tag).await;
    println!("Running old method...");
    let (total, dur) = run_old_method(&db, retention_secs, batch_size, tag).await;
    println!(
        "Deleted {} jobs in {:?} ({:.0} jobs/sec)",
        total,
        dur,
        total as f64 / dur.as_secs_f64()
    );
    cleanup_test_data(&db, tag).await;
}

#[tokio::test]
#[ignore]
async fn test_new_method() {
    let db = connect().await;
    let retention_secs: i64 = 600;
    let batch_size: i64 = 10000;
    let job_count = 10_000;
    let tag = "bench_new_solo";

    cleanup_test_data(&db, tag).await;
    println!("Inserting {} test jobs...", job_count);
    insert_test_jobs(&db, job_count, tag).await;
    println!("Running new method...");
    let (total, dur) = run_new_method(&db, retention_secs, batch_size, tag).await;
    println!(
        "Deleted {} jobs in {:?} ({:.0} jobs/sec)",
        total,
        dur,
        total as f64 / dur.as_secs_f64()
    );
    cleanup_test_data(&db, tag).await;
}

#[tokio::test]
#[ignore]
async fn test_crash_recovery() {
    let db = connect().await;
    let retention_secs: i64 = 600;
    let tag = "bench_crash_recovery";
    let instance = "bench";

    cleanup_test_data(&db, tag).await;
    println!("Inserting 1000 test jobs...");
    insert_test_jobs(&db, 1000, tag).await;

    // Simulate: mark jobs but don't delete them (mimics crash after step 1)
    let active_roots: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT q.id FROM v2_job_queue q
         JOIN v2_job j ON j.id = q.id
         WHERE j.parent_job IS NULL
           AND j.created_at <= now() - ($1::bigint::text || ' s')::interval",
        retention_secs
    )
    .fetch_all(&db)
    .await
    .unwrap();

    let marked = sqlx::query!(
        "INSERT INTO jobs_pending_deletion (id, marked_by)
         SELECT jc.id, $4 FROM v2_job_completed jc
         JOIN v2_job j ON j.id = jc.id
         WHERE jc.completed_at <= now() - ($1::bigint::text || ' s')::interval
           AND COALESCE(j.root_job, j.flow_innermost_root_job, jc.id) != ALL($2)
           AND j.tag = $3
         ORDER BY jc.completed_at ASC
         LIMIT 1000
         ON CONFLICT DO NOTHING",
        retention_secs,
        &active_roots,
        tag,
        instance
    )
    .execute(&db)
    .await
    .unwrap();

    let marked_count = marked.rows_affected();
    println!(
        "Marked {} jobs (simulating crash before deletion)",
        marked_count
    );
    assert!(marked_count > 0, "Should have marked some jobs");

    // Verify pending_deletion has rows
    let pending: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM jobs_pending_deletion")
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(0);
    assert!(pending > 0, "jobs_pending_deletion should have rows");

    // Now run the new method — it should detect leftovers and resume
    println!("Running new method (should resume from crash)...");
    let (total, dur) = run_new_method(&db, retention_secs, 1000, tag).await;
    println!("Recovered and deleted {} jobs in {:?}", total, dur);

    // Verify cleanup
    let remaining: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM jobs_pending_deletion")
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(0);
    assert_eq!(
        remaining, 0,
        "jobs_pending_deletion should be empty after recovery"
    );

    cleanup_test_data(&db, tag).await;
}
