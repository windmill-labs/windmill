//! Regression tests for the "Jobs waiting in queue" critical alert.
//!
//! ## Requirements
//!
//! - PostgreSQL database running locally
//! - Enterprise features enabled
//!
//! ## Running the tests
//!
//! ```bash
//! cargo test -p windmill-common --test jobs_waiting_alerts --features private,enterprise -- --ignored --nocapture
//! ```

#[cfg(all(feature = "private", feature = "enterprise"))]
mod tests {
    use serde_json::json;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;
    use windmill_common::ee::jobs_waiting_alerts;

    const TAG: &str = "python3";

    /// `jobs_waiting_alerts` bails out unless it can take the shared lock, and a
    /// freshly inserted lock row is never old enough to be claimed on the first
    /// pass. Seed it stale so the body actually runs.
    async fn free_the_lock(db: &Pool<Postgres>) {
        sqlx::query!(
            "INSERT INTO concurrency_locks (id, last_locked_at) VALUES ('jobs_waiting_alerts_lock', NOW() - INTERVAL '1 hour')
             ON CONFLICT (id) DO UPDATE SET last_locked_at = EXCLUDED.last_locked_at",
        )
        .execute(db)
        .await
        .expect("seed stale lock");
    }

    async fn configure_alert(db: &Pool<Postgres>, jobs_num_threshold: i64) {
        let config = json!({
            "alerts": [{
                "name": "queue depth",
                "tags_to_monitor": [TAG],
                "jobs_num_threshold": jobs_num_threshold,
                "alert_time_threshold_seconds": 60,
                "alert_cooldown_seconds": 600,
            }]
        });
        sqlx::query!(
            "INSERT INTO global_settings (name, value) VALUES ('alert_job_queue_waiting', $1)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
            config,
        )
        .execute(db)
        .await
        .expect("seed alert config");
    }

    /// Queue `n` jobs on TAG that have been due for 5 minutes. `gated` mirrors
    /// what the limiter writes when it re-queues a job it could not admit.
    async fn queue_jobs(db: &Pool<Postgres>, n: usize, gated: bool) {
        for _ in 0..n {
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, tag, running, scheduled_for, concurrency_gated)
                 VALUES ($1, 'test-workspace', $2, false, NOW() - INTERVAL '5 minutes', $3)",
                Uuid::new_v4(),
                TAG,
                gated.then_some(true),
            )
            .execute(db)
            .await
            .expect("queue job");
        }
    }

    async fn alert_messages(db: &Pool<Postgres>) -> Vec<String> {
        sqlx::query_scalar!("SELECT message FROM alerts WHERE alert_type = 'critical_error'")
            .fetch_all(db)
            .await
            .expect("read alerts")
    }

    /// Jobs the limiter parked behind their own concurrency gate must not page
    /// on-call: they are waiting by design, not for want of a worker, and no
    /// operator action drains them. Their re-queue timestamps mature in bursts,
    /// so counting them produces a stream of alerts whose count swings wildly
    /// while the fleet sits idle.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn concurrency_gated_backlog_does_not_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        configure_alert(&db, 100).await;
        queue_jobs(&db, 200, true).await;

        jobs_waiting_alerts(&db).await;

        assert!(
            alert_messages(&db).await.is_empty(),
            "a backlog the limiter parked behind a concurrency gate must not raise a critical alert"
        );
    }

    /// The complement: jobs no gate is holding are waiting on capacity and must
    /// page, counted apart from the gated ones so the number can be reconciled
    /// against the much larger raw queue depth.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn jobs_waiting_on_capacity_still_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        configure_alert(&db, 100).await;
        queue_jobs(&db, 150, false).await;
        queue_jobs(&db, 200, true).await;

        jobs_waiting_alerts(&db).await;

        let messages = alert_messages(&db).await;
        assert_eq!(
            messages.len(),
            1,
            "expected exactly one critical alert, got {messages:?}"
        );
        assert!(
            messages[0].contains("150"),
            "alert must count only the jobs actually waiting on capacity: {}",
            messages[0]
        );
        assert!(
            messages[0].contains("200"),
            "alert must report the gated jobs it excluded: {}",
            messages[0]
        );
    }
}
