//! Regression tests for the oversubscribed concurrency gate alert.
//!
//! ## Requirements
//!
//! - PostgreSQL database running locally
//! - Enterprise features enabled
//!
//! ## Running the tests
//!
//! ```bash
//! cargo test -p windmill-common --test concurrency_gate_alerts --features private,enterprise -- --ignored --nocapture
//! ```

#[cfg(all(feature = "private", feature = "enterprise"))]
mod tests {
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;
    use windmill_common::ee::concurrency_gate_alerts;

    /// `key|limit|window` as the limiter stamps it: one job per 5s, so the gate
    /// admits 17280/day.
    const SLOW_GATE: &str = "test-workspace/u/user/sync|1|5";

    async fn free_the_lock(db: &Pool<Postgres>) {
        sqlx::query!(
            "INSERT INTO concurrency_locks (id, last_locked_at) VALUES ('concurrency_gate_alerts_lock', NOW() - INTERVAL '1 hour')
             ON CONFLICT (id) DO UPDATE SET last_locked_at = EXCLUDED.last_locked_at",
        )
        .execute(db)
        .await
        .expect("seed stale lock");
    }

    /// Queue `n` jobs parked under `gate`, created `created_hours_ago` ago.
    async fn queue_gated(db: &Pool<Postgres>, n: usize, gate: &str, created_hours_ago: i64) {
        for _ in 0..n {
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, tag, running, scheduled_for,
                                           created_at, concurrency_gated_at, concurrency_gate_id)
                 VALUES ($1, 'test-workspace', 'python3', false, NOW(),
                         NOW() - INTERVAL '1 hour' * $2::bigint, NOW(), $3)",
                Uuid::new_v4(),
                created_hours_ago,
                gate,
            )
            .execute(db)
            .await
            .expect("queue gated job");
        }
    }

    async fn alert_messages(db: &Pool<Postgres>) -> Vec<String> {
        sqlx::query_scalar!("SELECT message FROM alerts WHERE alert_type = 'critical_error'")
            .fetch_all(db)
            .await
            .expect("read alerts")
    }

    /// A gate taking in more than it can admit accumulates forever, so those jobs
    /// will never run. The queue-depth alert deliberately ignores them because no
    /// operator drains a tenant's own rate limit -- but the tenant can act, and
    /// nothing else would ever tell them.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn oversubscribed_gate_alerts(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        // 40k queued against a 17280/day ceiling, still arriving.
        queue_gated(&db, 40_000, SLOW_GATE, 0).await;

        concurrency_gate_alerts(&db).await;

        let messages = alert_messages(&db).await;
        assert_eq!(
            messages.len(),
            1,
            "a gate that cannot drain its backlog must alert, got {messages:?}"
        );
        assert!(
            messages[0].contains("17280") && messages[0].contains("40000"),
            "alert must name the backlog and the ceiling it exceeds: {}",
            messages[0]
        );
    }

    /// A backlog that is merely large is slow, not broken. Alerting on one that
    /// has stopped growing would repeat the mistake the queue-depth alert made.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn draining_backlog_does_not_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        // Same depth, but every job predates the growth window: nothing new arrived.
        queue_gated(&db, 40_000, SLOW_GATE, 48).await;

        concurrency_gate_alerts(&db).await;

        assert!(
            alert_messages(&db).await.is_empty(),
            "a backlog that has stopped growing is draining and must not alert"
        );
    }
}
