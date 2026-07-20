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
    use windmill_common::ee::concurrency_gate_alerts;

    /// `key|limit|window` as the limiter stamps it: one job per 5s, so the gate
    /// admits 17280/day.
    const SLOW_GATE: &str = "test-workspace/u/user/sync|1|5";
    const STALE_GATE: &str = "test-workspace/u/user/stale|1|5";

    async fn free_the_lock(db: &Pool<Postgres>) {
        sqlx::query!(
            "INSERT INTO concurrency_locks (id, last_locked_at) VALUES ('concurrency_gate_alerts_lock', NOW() - INTERVAL '1 hour')
             ON CONFLICT (id) DO UPDATE SET last_locked_at = EXCLUDED.last_locked_at",
        )
        .execute(db)
        .await
        .expect("seed stale lock");
    }

    /// Queue `n` jobs parked under `gate`, created `created_hours_ago` ago, with
    /// the gate mark written `mark_mins_ago` ago. Inserted as one set-returning
    /// statement: these fixtures run to tens of thousands of rows.
    async fn queue_gated_full(
        db: &Pool<Postgres>,
        n: i64,
        gate: &str,
        created_hours_ago: i64,
        mark_mins_ago: i64,
        canceled: bool,
    ) {
        // A non-delayed job reaches the gate when it is created, so first_gated
        // tracks created_at here. The delayed-arrival case sets them apart via
        // queue_gated_delayed.
        queue_gated_full_delayed(db, n, gate, created_hours_ago, created_hours_ago, mark_mins_ago, canceled)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn queue_gated_full_delayed(
        db: &Pool<Postgres>,
        n: i64,
        gate: &str,
        created_hours_ago: i64,
        first_gated_hours_ago: i64,
        mark_mins_ago: i64,
        canceled: bool,
    ) {
        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, tag, running, scheduled_for,
                                       created_at, concurrency_gated_at, concurrency_first_gated_at,
                                       concurrency_gate_id, canceled_by)
             SELECT gen_random_uuid(), 'test-workspace', 'python3', false, NOW(),
                    NOW() - INTERVAL '1 hour' * $2::bigint,
                    NOW() - INTERVAL '1 minute' * $4::bigint,
                    NOW() - INTERVAL '1 hour' * $3::bigint,
                    $5,
                    CASE WHEN $6 THEN 'canceller' ELSE NULL END
             FROM generate_series(1, $1::bigint)",
            n,
            created_hours_ago,
            first_gated_hours_ago,
            mark_mins_ago,
            gate,
            canceled,
        )
        .execute(db)
        .await
        .expect("queue gated jobs");
    }

    async fn queue_gated(db: &Pool<Postgres>, n: i64, gate: &str, created_hours_ago: i64) {
        queue_gated_full(db, n, gate, created_hours_ago, 1, false).await
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
            messages[0].contains("17280") && messages[0].contains("40000")
                && messages[0].contains("720"),
            "alert must name the backlog and both ceilings it exceeds: {}",
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

    /// Arrivals must outrun the gate, not merely exist. A backlog draining at the
    /// ceiling still leaves recent survivors queued, and treating any of them as
    /// growth reports a shrinking queue as unbounded.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn shrinking_backlog_does_not_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        queue_gated(&db, 40_000, SLOW_GATE, 48).await;
        // One arrival a day against a 17280/day ceiling: the queue is draining.
        queue_gated(&db, 1, SLOW_GATE, 0).await;

        concurrency_gate_alerts(&db).await;

        assert!(
            alert_messages(&db).await.is_empty(),
            "a backlog shrinking at the gate ceiling must not alert"
        );
    }

    /// Cancelled jobs bypass the limiter and stale marks outlive the gate that
    /// wrote them, so neither is held by the gate they still name.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn cancelled_and_stale_marks_are_not_counted(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        queue_gated_full(&db, 40_000, SLOW_GATE, 0, 1, true).await;
        queue_gated_full(&db, 40_000, STALE_GATE, 0, 600, false).await;

        concurrency_gate_alerts(&db).await;

        assert!(
            alert_messages(&db).await.is_empty(),
            "cancelled jobs and marks from a gate that stopped refreshing must not alert"
        );
    }

    /// A gate that drains completely vanishes from the aggregation, so recovery
    /// has to be swept rather than keyed off the gates still present -- otherwise
    /// its alert stays open forever.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn a_drained_gate_recovers(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        queue_gated(&db, 40_000, SLOW_GATE, 0).await;
        concurrency_gate_alerts(&db).await;
        assert_eq!(alert_messages(&db).await.len(), 1, "expected the initial alert");

        sqlx::query!("DELETE FROM v2_job_queue")
            .execute(&db)
            .await
            .expect("drain the gate");
        free_the_lock(&db).await;
        concurrency_gate_alerts(&db).await;

        let recovery_workspace: Option<String> = sqlx::query_scalar!(
            "SELECT workspace_id FROM alerts
             WHERE alert_type = 'recovered_critical_error' ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(&db)
        .await
        .expect("read recovery alert")
        .flatten();
        assert_eq!(
            recovery_workspace.as_deref(),
            Some("test-workspace"),
            "recovery must stay scoped to the workspace the alert was raised for"
        );

        let unresolved: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM healthchecks
             WHERE check_type LIKE 'concurrency_gate_alert_%' AND healthy = false"
        )
        .fetch_one(&db)
        .await
        .expect("read healthchecks")
        .unwrap_or(0);
        assert_eq!(unresolved, 0, "a fully drained gate must have its alert recovered");
    }

    /// The window governs how soon the alert fires, so a gate taking on less than
    /// it admits over that window is keeping up and must stay silent even though
    /// a long backlog sits in front of it.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn arrivals_under_the_window_ceiling_do_not_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        queue_gated(&db, 40_000, SLOW_GATE, 48).await;
        // 700 in the last hour against a 720/hour ceiling: still keeping up.
        queue_gated(&db, 700, SLOW_GATE, 0).await;

        concurrency_gate_alerts(&db).await;

        assert!(
            alert_messages(&db).await.is_empty(),
            "arrivals below the gate ceiling for the window must not alert"
        );
    }

    /// A job can be pushed with a future scheduled_for and only reach the gate
    /// long after it was created. Its arrival must be measured from when the
    /// limiter first parked it, not its creation time -- otherwise a burst of
    /// long-scheduled jobs coming due looks like no arrivals at all and the alert
    /// stays silent through a real backlog.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn jobs_delayed_before_reaching_the_gate_still_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        // Created 3 days ago, but only reached the gate within the last hour and
        // still arriving faster than the 720/hour ceiling.
        queue_gated_full_delayed(&db, 40_000, SLOW_GATE, 72, 0, 1, false).await;

        concurrency_gate_alerts(&db).await;

        assert_eq!(
            alert_messages(&db).await.len(),
            1,
            "arrivals gauged from creation would miss long-delayed jobs reaching the gate"
        );
    }
}
