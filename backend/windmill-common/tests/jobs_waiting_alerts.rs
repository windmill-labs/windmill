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
    const GATE_KEY: &str = "test-workspace/u/user/gated_script";

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

    /// A concurrency gate as the push path lays it out: the limit hangs off
    /// `runnable_settings_handle` -> `concurrency_settings`, and the live
    /// admission count is the number of job uuids in `concurrency_counter`.
    /// `running` is how many slots are currently taken, so `running == limit`
    /// is a saturated gate.
    async fn seed_gate(db: &Pool<Postgres>, key: &str, handle: i64, limit: i32, running: usize) {
        let job_uuids: serde_json::Value = (0..running)
            .map(|_| (Uuid::new_v4().hyphenated().to_string(), json!({})))
            .collect::<serde_json::Map<_, _>>()
            .into();

        sqlx::query!(
            "INSERT INTO concurrency_settings (hash, concurrency_key, concurrent_limit, concurrency_time_window_s)
             VALUES ($1, $2, $3, 5)",
            handle,
            key,
            limit,
        )
        .execute(db)
        .await
        .expect("seed concurrency settings");

        sqlx::query!(
            "INSERT INTO runnable_settings (hash, concurrency_settings) VALUES ($1, $1)",
            handle,
        )
        .execute(db)
        .await
        .expect("seed runnable settings");

        sqlx::query!(
            "INSERT INTO concurrency_counter (concurrency_id, job_uuids) VALUES ($1, $2)",
            key,
            job_uuids,
        )
        .execute(db)
        .await
        .expect("seed concurrency counter");
    }

    /// Queue `n` jobs on TAG that have been due for 5 minutes. When `gate` is
    /// set, each job also gets the `concurrency_key` row that
    /// `insert_concurrency_key` writes at push time for any runnable carrying a
    /// concurrent_limit, plus the settings handle that carries the limit.
    async fn queue_jobs(db: &Pool<Postgres>, n: usize, gate: Option<(&str, i64)>) {
        for _ in 0..n {
            let id = Uuid::new_v4();
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, tag, running, scheduled_for, runnable_settings_handle)
                 VALUES ($1, 'test-workspace', $2, false, NOW() - INTERVAL '5 minutes', $3)",
                id,
                TAG,
                gate.map(|(_, handle)| handle),
            )
            .execute(db)
            .await
            .expect("queue job");

            if let Some((key, _)) = gate {
                sqlx::query!(
                    "INSERT INTO concurrency_key (job_id, key) VALUES ($1, $2)",
                    id,
                    key,
                )
                .execute(db)
                .await
                .expect("gate job");
            }
        }
    }

    async fn alert_messages(db: &Pool<Postgres>) -> Vec<String> {
        sqlx::query_scalar!("SELECT message FROM alerts WHERE alert_type = 'critical_error'")
            .fetch_all(db)
            .await
            .expect("read alerts")
    }

    /// Jobs parked by their own workspace's concurrency gate must not page
    /// on-call: they are waiting by design, not for want of a worker, and no
    /// operator action drains them. Their re-queue timestamps mature in bursts,
    /// so counting them produces a stream of alerts whose count swings wildly
    /// while the fleet sits idle.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn concurrency_gated_backlog_does_not_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        configure_alert(&db, 100).await;
        seed_gate(&db, GATE_KEY, 1, 1, 1).await;
        queue_jobs(&db, 200, Some((GATE_KEY, 1))).await;

        jobs_waiting_alerts(&db).await;

        assert!(
            alert_messages(&db).await.is_empty(),
            "a backlog held entirely behind a saturated concurrency gate must not raise a critical alert"
        );
    }

    /// The exclusion keys off the gate being *full*, not off the job merely
    /// carrying a concurrent_limit -- every such job gets a `concurrency_key`
    /// row at push time, so excluding on that row alone would stop this alert
    /// from ever firing for a rate-limited runnable. Half the backlog here sits
    /// under a gate with free slots: it is starving for workers like the plain
    /// jobs beside it and must be counted. Excluding it drops the count under
    /// the threshold and the alert goes silent.
    #[ignore = "requires database setup - run with --ignored flag"]
    #[sqlx::test(migrations = "../migrations")]
    async fn jobs_waiting_on_capacity_still_alert(db: Pool<Postgres>) {
        free_the_lock(&db).await;
        configure_alert(&db, 150).await;
        seed_gate(&db, GATE_KEY, 1, 10, 2).await;
        queue_jobs(&db, 100, None).await;
        queue_jobs(&db, 100, Some((GATE_KEY, 1))).await;

        jobs_waiting_alerts(&db).await;

        let messages = alert_messages(&db).await;
        assert_eq!(
            messages.len(),
            1,
            "expected exactly one critical alert, got {messages:?}"
        );
        assert!(
            messages[0].contains("200"),
            "every job waiting on capacity must be counted, gated or not: {}",
            messages[0]
        );
    }
}
