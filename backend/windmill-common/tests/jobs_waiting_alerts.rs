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
    const BUSY_GATE: &str = "test-workspace/u/user/busy";
    const FREED_GATE: &str = "test-workspace/u/user/freed";

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

    /// Queue `n` jobs on TAG that have been due for 5 minutes. `gate` mirrors
    /// the stamp the limiter writes when it re-queues a job it could not admit:
    /// the key it was parked under and how long ago. `None` is a job no gate has
    /// parked.
    async fn queue_jobs(db: &Pool<Postgres>, n: usize, gate: Option<(&str, i64)>) {
        for _ in 0..n {
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, tag, running, scheduled_for,
                                           concurrency_gated_at, concurrency_gated_key)
                 VALUES ($1, 'test-workspace', $2, false, NOW() - INTERVAL '5 minutes',
                         CASE WHEN $3::bigint IS NULL THEN NULL
                              ELSE NOW() - INTERVAL '1 second' * $3::bigint END,
                         $4)",
                Uuid::new_v4(),
                TAG,
                gate.map(|(_, secs)| secs),
                gate.map(|(key, _)| key),
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
    /// on-call: they wait by design, not for want of a worker, and no operator
    /// action drains them. Their re-queue timestamps mature in bursts, so
    /// counting them alerts repeatedly while the fleet sits idle.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn concurrency_gated_backlog_does_not_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        configure_alert(&db, 100).await;
        queue_jobs(&db, 200, Some((BUSY_GATE, 1))).await;

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
        queue_jobs(&db, 150, None).await;
        queue_jobs(&db, 200, Some((BUSY_GATE, 1))).await;

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

    /// A mark is only evidence while the limiter keeps refreshing it. Once a gate
    /// frees, nothing clears the marks it left, so if workers then disappear the
    /// backlog is genuinely starved while still carrying them. Marks that stopped
    /// advancing must not be trusted, or the outage stays silent forever.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn stale_gate_marks_do_not_suppress_the_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        configure_alert(&db, 100).await;
        // Marked well beyond alert_time_threshold_seconds and never refreshed.
        queue_jobs(&db, 200, Some((FREED_GATE, 3600))).await;

        jobs_waiting_alerts(&db).await;

        let messages = alert_messages(&db).await;
        assert_eq!(
            messages.len(),
            1,
            "a backlog whose gate marks stopped advancing is waiting on workers and \
             must alert, got {messages:?}"
        );
        assert!(
            messages[0].contains("200"),
            "every job must be counted once its marks go stale: {}",
            messages[0]
        );
    }

    /// Gates on a tag free independently, so freshness cannot be judged tag-wide:
    /// one key still being refreshed would vouch for stale marks left by another
    /// that has since freed. The freed key's backlog is unpulled and starving
    /// while the busy key keeps producing fresh marks beside it.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn a_live_gate_does_not_vouch_for_another_gates_stale_marks(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        configure_alert(&db, 100).await;
        queue_jobs(&db, 200, Some((BUSY_GATE, 1))).await;
        queue_jobs(&db, 150, Some((FREED_GATE, 3600))).await;

        jobs_waiting_alerts(&db).await;

        let messages = alert_messages(&db).await;
        assert_eq!(
            messages.len(),
            1,
            "the freed gate's backlog is waiting on workers and must alert, got {messages:?}"
        );
        assert!(
            messages[0].contains("150"),
            "only the freed gate's jobs count; the live gate still holds its own: {}",
            messages[0]
        );
    }
}
