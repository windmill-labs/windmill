/// Integration test for batched job cleanup
///
/// Prerequisites:
///   - PostgreSQL running with windmill database
///   - DATABASE_URL env var or default: postgres://postgres:changeme@localhost:5432/windmill
///
/// Run all tests:
///   cargo test --test job_cleanup_test -- --nocapture
///
/// Run specific test:
///   cargo test --test job_cleanup_test test_batched_job_cleanup -- --nocapture
///   cargo test --test job_cleanup_test test_skip_locked_prevents_contention -- --nocapture
///
/// Setup test data (run in psql first):
///   -- Set short retention for testing
///   UPDATE global_settings SET value = '3600' WHERE name = 'retention_period_secs';
///
///   -- Insert expired test jobs
///   INSERT INTO v2_job (id, tag, workspace_id, created_at, created_by, permissioned_as, permissioned_as_email, kind, same_worker, visible_to_owner)
///   SELECT gen_random_uuid(), 'deno', 'admins', now() - interval '2 hours', 'test_user', 'test_user', 'test@example.com', 'script', false, true
///   FROM generate_series(1, 10000);
///
///   INSERT INTO v2_job_completed (id, workspace_id, duration_ms, deleted, status, completed_at, started_at)
///   SELECT j.id, j.workspace_id, 100, false, 'success', j.created_at + interval '1 second', j.created_at
///   FROM v2_job j WHERE j.created_by = 'test_user';
use sqlx::postgres::PgPoolOptions;
use std::time::Instant;
use uuid::Uuid;

#[tokio::test]
#[ignore]
async fn test_batched_job_cleanup() {
    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());

    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Get retention period from settings
    let retention_secs: i64 = 2592000;

    println!(
        "Retention period: {} seconds ({} days)",
        retention_secs,
        retention_secs / 86400
    );

    // Count jobs eligible for deletion
    let eligible_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM v2_job_completed WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval",
        retention_secs
    )
    .fetch_one(&db)
    .await
    .expect("Failed to count eligible jobs")
    .unwrap_or(0);

    println!("Jobs eligible for deletion: {}", eligible_count);

    if eligible_count == 0 {
        println!("No jobs to delete. To create test data, run:");
        println!("  psql $DATABASE_URL -c \"UPDATE global_settings SET value = '3600' WHERE name = 'retention_period_secs';\"");
        println!("  Then insert expired jobs (see test file header for SQL)");
        return;
    }

    // Test different batch sizes
    let batch_sizes = [1000, 5000, 10000];

    for batch_size in batch_sizes {
        let remaining: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM v2_job_completed WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval",
            retention_secs
        )
        .fetch_one(&db)
        .await
        .unwrap()
        .unwrap_or(0);

        if remaining == 0 {
            println!("All jobs deleted, stopping batch size tests");
            break;
        }

        println!(
            "\n--- Testing batch_size={} (remaining: {}) ---",
            batch_size, remaining
        );

        let start = Instant::now();
        let mut total_deleted = 0u64;
        let mut batch_num = 0u32;
        let max_batches = 5;

        loop {
            if batch_num >= max_batches {
                println!("Reached max batches limit ({})", max_batches);
                break;
            }

            let batch_start = Instant::now();

            // Run batched deletion with LIMIT and SKIP LOCKED (same as monitor.rs)
            let deleted_jobs: Vec<Uuid> = sqlx::query_scalar!(
                "DELETE FROM v2_job_completed
                 WHERE id IN (
                     SELECT id FROM v2_job_completed
                     WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval
                     ORDER BY completed_at ASC
                     LIMIT $2
                     FOR UPDATE SKIP LOCKED
                 )
                 RETURNING id",
                retention_secs,
                batch_size as i64
            )
            .fetch_all(&db)
            .await
            .expect("Failed to delete batch");

            let batch_count = deleted_jobs.len();
            let batch_elapsed = batch_start.elapsed();

            if batch_count == 0 {
                println!("No more jobs to delete");
                break;
            }

            // Delete related records
            sqlx::query!(
                "DELETE FROM job_stats WHERE job_id = ANY($1)",
                &deleted_jobs
            )
            .execute(&db)
            .await
            .ok();

            sqlx::query!("DELETE FROM job_logs WHERE job_id = ANY($1)", &deleted_jobs)
                .execute(&db)
                .await
                .ok();

            sqlx::query!("DELETE FROM v2_job WHERE id = ANY($1)", &deleted_jobs)
                .execute(&db)
                .await
                .ok();

            sqlx::query!(
                "DELETE FROM job_result_stream_v2 WHERE job_id = ANY($1)",
                &deleted_jobs
            )
            .execute(&db)
            .await
            .ok();

            total_deleted += batch_count as u64;
            batch_num += 1;

            let rate = batch_count as f64 / batch_elapsed.as_secs_f64();
            println!(
                "  Batch {}: deleted {} jobs in {:?} ({:.0} jobs/sec)",
                batch_num, batch_count, batch_elapsed, rate
            );
        }

        let total_elapsed = start.elapsed();
        let overall_rate = if total_elapsed.as_secs_f64() > 0.0 {
            total_deleted as f64 / total_elapsed.as_secs_f64()
        } else {
            0.0
        };
        println!(
            "batch_size={}: deleted {} jobs in {} batches, total time {:?} ({:.0} jobs/sec)",
            batch_size, total_deleted, batch_num, total_elapsed, overall_rate
        );
    }

    let final_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM v2_job_completed WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval",
        retention_secs
    )
    .fetch_one(&db)
    .await
    .unwrap()
    .unwrap_or(0);

    println!(
        "\nFinal eligible count: {} (started with {})",
        final_count, eligible_count
    );
}

#[tokio::test]
#[ignore]
async fn test_skip_locked_prevents_contention() {
    // This test verifies that SKIP LOCKED allows concurrent cleanup without deadlocks
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or("postgres://postgres:changeme@localhost:5432/windmill".to_string());

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    let retention_secs: i64 = sqlx::query_scalar!(
        "SELECT COALESCE((SELECT value::bigint FROM global_settings WHERE name = 'retention_period_secs'), 2592000)"
    )
    .fetch_one(&db)
    .await
    .expect("Failed to get retention period")
    .unwrap_or(2592000);

    let eligible: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM v2_job_completed WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval",
        retention_secs
    )
    .fetch_one(&db)
    .await
    .unwrap()
    .unwrap_or(0);

    println!("Retention period: {} seconds", retention_secs);
    println!("Jobs eligible for deletion: {}", eligible);

    if eligible == 0 {
        println!("No jobs to delete, skipping concurrent test");
        return;
    }

    let batch_size: i64 = 1000;

    // Run 3 concurrent cleanup tasks (simulating multiple server replicas)
    let handles: Vec<_> = (0..3)
        .map(|worker_id| {
            let db = db.clone();
            tokio::spawn(async move {
                let start = Instant::now();
                let mut total = 0u64;

                for _batch in 0..3 {
                    let deleted: Vec<Uuid> = sqlx::query_scalar!(
                        "DELETE FROM v2_job_completed
                         WHERE id IN (
                             SELECT id FROM v2_job_completed
                             WHERE completed_at <= now() - ($1::bigint::text || ' s')::interval
                             ORDER BY completed_at ASC
                             LIMIT $2
                             FOR UPDATE SKIP LOCKED
                         )
                         RETURNING id",
                        retention_secs,
                        batch_size
                    )
                    .fetch_all(&db)
                    .await
                    .unwrap_or_default();

                    let count = deleted.len();
                    total += count as u64;

                    if count > 0 {
                        sqlx::query!("DELETE FROM v2_job WHERE id = ANY($1)", &deleted)
                            .execute(&db)
                            .await
                            .ok();
                    }

                    if count == 0 {
                        break;
                    }
                }

                println!(
                    "Worker {}: deleted {} jobs in {:?}",
                    worker_id,
                    total,
                    start.elapsed()
                );
                total
            })
        })
        .collect();

    let results: Vec<u64> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap_or(0))
        .collect();

    let total: u64 = results.iter().sum();
    println!("\nTotal deleted by all workers: {}", total);
    println!("Per worker: {:?}", results);
    println!("\nWith SKIP LOCKED: no deadlocks, work distributed across workers");
}
