//! Tests for `update_concurrency_counter` — the per-script concurrency limiter
//! at the heart of `apply_concurrency_limit` (windmill-queue/jobs_ee.rs).
//!
//! The structural property under test is the **never-over-admit invariant**:
//! across N concurrent admission attempts on a single concurrency_id with
//! limit L, the number of admits must never exceed L.
//!
//! Run with:
//!   cargo test -p windmill-queue --test concurrency_counter_test \
//!       --features private,enterprise -- --nocapture

#[cfg(all(feature = "private", feature = "enterprise"))]
mod concurrency {
    use serde_json::json;
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;
    use windmill_queue::jobs_ee::update_concurrency_counter;

    fn init_uuids(job_id: Uuid) -> serde_json::Value {
        json!({ job_id.hyphenated().to_string(): {} })
    }

    async fn admitted_in_counter(db: &Pool<Postgres>, key: &str) -> i64 {
        sqlx::query_scalar!(
            "SELECT COALESCE((SELECT COUNT(*) FROM jsonb_object_keys(job_uuids)), 0)
             FROM concurrency_counter WHERE concurrency_id = $1",
            key,
        )
        .fetch_optional(db)
        .await
        .expect("query admitted")
        .flatten()
        .unwrap_or(0)
    }

    /// Stress: N concurrent admit-attempts on a pre-existing key with limit
    /// L < N. Asserts admitted count is exactly L (never over-admits) AND
    /// that the row's job_uuids reflects exactly L entries.
    ///
    /// The row is pre-seeded with an empty `job_uuids` because there is a
    /// known preexisting race when the `concurrency_counter` row does not
    /// exist yet: `SELECT ... FOR UPDATE` on a missing row locks nothing,
    /// so all concurrent transactions see `running=0` and all admit via
    /// `INSERT ... ON CONFLICT DO UPDATE`. That race is inherited from
    /// the legacy code path and is out of scope for this PR. This test
    /// validates the property we *do* preserve: no over-admit once the
    /// row exists.
    #[sqlx::test(migrations = "../migrations")]
    async fn stress_never_over_admits(db: Pool<Postgres>) {
        let key = "stress-never-over-admits".to_string();
        let limit: i32 = 5;
        let workers: usize = 50;

        sqlx::query!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids) VALUES ($1, '{}'::jsonb)",
            key,
        )
        .execute(&db)
        .await
        .expect("seed empty counter row");

        let mut tasks = Vec::with_capacity(workers);
        for _ in 0..workers {
            let db = db.clone();
            let key = key.clone();
            let job_id = Uuid::new_v4();
            tasks.push(tokio::spawn(async move {
                update_concurrency_counter(
                    &db,
                    &job_id,
                    key,
                    init_uuids(job_id),
                    job_id.hyphenated().to_string(),
                    60,
                    limit,
                )
                .await
            }));
        }

        let mut admitted = 0;
        for t in tasks {
            let (within, _) = t.await.expect("join").expect("update_concurrency_counter");
            if within {
                admitted += 1;
            }
        }

        assert_eq!(
            admitted, limit as usize,
            "admit count must equal the limit exactly under contention"
        );

        let in_counter = admitted_in_counter(&db, &key).await;
        assert_eq!(
            in_counter, limit as i64,
            "concurrency_counter.job_uuids must contain exactly `limit` entries"
        );
    }

    /// First-job race: N concurrent attempts when no concurrency_counter row
    /// exists yet, with `limit >= N`. All must admit, and the row must end up
    /// with exactly N entries (no INSERT-ON-CONFLICT lost updates).
    #[sqlx::test(migrations = "../migrations")]
    async fn first_job_race_all_admit(db: Pool<Postgres>) {
        let key = "first-job-race".to_string();
        let workers: usize = 10;
        let limit: i32 = workers as i32;

        let mut tasks = Vec::with_capacity(workers);
        for _ in 0..workers {
            let db = db.clone();
            let key = key.clone();
            let job_id = Uuid::new_v4();
            tasks.push(tokio::spawn(async move {
                update_concurrency_counter(
                    &db,
                    &job_id,
                    key,
                    init_uuids(job_id),
                    job_id.hyphenated().to_string(),
                    60,
                    limit,
                )
                .await
            }));
        }

        let mut admitted = 0;
        for t in tasks {
            let (within, _) = t.await.expect("join").expect("update_concurrency_counter");
            if within {
                admitted += 1;
            }
        }

        assert_eq!(
            admitted, workers,
            "all jobs must admit when limit == workers"
        );
        assert_eq!(admitted_in_counter(&db, &key).await, workers as i64);
    }

    /// Bail path: pre-populate the row with `limit` entries, then issue M
    /// concurrent calls. None must admit. Counter must remain at `limit`.
    #[sqlx::test(migrations = "../migrations")]
    async fn bail_path_no_admits_when_full(db: Pool<Postgres>) {
        let key = "bail-path-full".to_string();
        let limit: i32 = 3;

        let mut filled = serde_json::Map::new();
        for _ in 0..limit {
            filled.insert(Uuid::new_v4().hyphenated().to_string(), json!({}));
        }
        sqlx::query!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids) VALUES ($1, $2)",
            key,
            serde_json::Value::Object(filled),
        )
        .execute(&db)
        .await
        .expect("seed counter");

        let attempts: usize = 20;
        let mut tasks = Vec::with_capacity(attempts);
        for _ in 0..attempts {
            let db = db.clone();
            let key = key.clone();
            let job_id = Uuid::new_v4();
            tasks.push(tokio::spawn(async move {
                update_concurrency_counter(
                    &db,
                    &job_id,
                    key,
                    init_uuids(job_id),
                    job_id.hyphenated().to_string(),
                    60,
                    limit,
                )
                .await
            }));
        }

        for t in tasks {
            let (within, _) = t.await.expect("join").expect("update_concurrency_counter");
            assert!(!within, "no admits expected when row is already at limit");
        }
        assert_eq!(
            admitted_in_counter(&db, &key).await,
            limit as i64,
            "counter must remain unchanged on the bail path"
        );
    }

    /// Idempotent re-pull: when the same `pulled_job_id` is already in
    /// `job_uuids` (retry case), it must not be double-counted toward the
    /// limit. Pre-seed with `limit` entries including `retry_id`; the call
    /// for `retry_id` should admit because the WHERE key <> $2 clause excludes
    /// its own UUID from the count.
    #[sqlx::test(migrations = "../migrations")]
    async fn retry_not_double_counted(db: Pool<Postgres>) {
        let key = "retry-not-double-counted".to_string();
        let limit: i32 = 3;
        let retry_id = Uuid::new_v4();

        let mut filled = serde_json::Map::new();
        filled.insert(retry_id.hyphenated().to_string(), json!({}));
        for _ in 0..(limit - 1) {
            filled.insert(Uuid::new_v4().hyphenated().to_string(), json!({}));
        }
        sqlx::query!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids) VALUES ($1, $2)",
            key,
            serde_json::Value::Object(filled),
        )
        .execute(&db)
        .await
        .expect("seed counter");

        let (within, _) = update_concurrency_counter(
            &db,
            &retry_id,
            key.clone(),
            init_uuids(retry_id),
            retry_id.hyphenated().to_string(),
            60,
            limit,
        )
        .await
        .expect("retry call");

        assert!(within, "retrying an already-tracked job must admit");
    }

    /// `max_ended_at` plumbing: when `concurrency_key` has rows in the window,
    /// the returned `max_ended_at` must equal the max from the table.
    #[sqlx::test(migrations = "../migrations")]
    async fn max_ended_at_returned(db: Pool<Postgres>) {
        let key = "max-ended-at".to_string();
        let limit: i32 = 100;

        // Seed two completed entries in the window. We need a v2_job + v2_job_completed
        // row each because concurrency_key references job_id with a FK.
        // Simpler: insert directly; concurrency_key has no FK on job_id beyond not-null.
        let now = chrono::Utc::now();
        let earlier = now - chrono::Duration::seconds(5);
        for ended_at in [earlier, now] {
            sqlx::query!(
                "INSERT INTO concurrency_key (job_id, key, ended_at) VALUES ($1, $2, $3)",
                Uuid::new_v4(),
                key,
                ended_at,
            )
            .execute(&db)
            .await
            .expect("seed concurrency_key");
        }

        let job_id = Uuid::new_v4();
        let (within, max_ended_at) = update_concurrency_counter(
            &db,
            &job_id,
            key.clone(),
            init_uuids(job_id),
            job_id.hyphenated().to_string(),
            60,
            limit,
        )
        .await
        .expect("call");

        assert!(within);
        let max = max_ended_at.expect("max_ended_at present");
        assert!(
            (max - now).num_milliseconds().abs() < 1000,
            "returned max_ended_at ({max}) must match the latest concurrency_key row ({now})"
        );
    }

    /// Helper: pre-seeded stress with given (limit, workers, time_window_s).
    /// Returns (admitted, in_counter).
    async fn run_stress(
        db: &Pool<Postgres>,
        key: &str,
        limit: i32,
        workers: usize,
        time_window_s: i32,
    ) -> (usize, i64) {
        sqlx::query!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids) VALUES ($1, '{}'::jsonb) ON CONFLICT DO NOTHING",
            key,
        )
        .execute(db)
        .await
        .expect("seed empty counter row");

        let mut tasks = Vec::with_capacity(workers);
        for _ in 0..workers {
            let db = db.clone();
            let key = key.to_string();
            let job_id = Uuid::new_v4();
            tasks.push(tokio::spawn(async move {
                update_concurrency_counter(
                    &db,
                    &job_id,
                    key,
                    init_uuids(job_id),
                    job_id.hyphenated().to_string(),
                    time_window_s,
                    limit,
                )
                .await
            }));
        }

        let mut admitted = 0;
        for t in tasks {
            let (within, _) = t.await.expect("join").expect("update_concurrency_counter");
            if within {
                admitted += 1;
            }
        }
        let in_counter = admitted_in_counter(db, key).await;
        (admitted, in_counter)
    }

    /// Stress matrix across (limit, workers, window) combinations. Every
    /// scenario must admit exactly `min(limit, workers)` and the counter
    /// must agree.
    #[sqlx::test(migrations = "../migrations")]
    async fn stress_matrix(db: Pool<Postgres>) {
        let scenarios: &[(i32, usize, i32)] = &[
            (1, 10, 60),
            (1, 100, 60),
            (2, 50, 60),
            (5, 50, 60),
            (5, 200, 60),
            (10, 200, 60),
            (20, 200, 60),
            (5, 50, 1),
            (5, 50, 3600),
            // workers <= limit: all admit
            (50, 10, 60),
            (50, 50, 60),
        ];
        for (i, (limit, workers, window)) in scenarios.iter().enumerate() {
            let key = format!("stress-matrix-{i}");
            let (admitted, in_counter) = run_stress(&db, &key, *limit, *workers, *window).await;
            let expected = (*limit as usize).min(*workers);
            assert_eq!(
                admitted, expected,
                "scenario limit={limit}, workers={workers}, window={window}: admitted={admitted}, expected={expected}"
            );
            assert_eq!(
                in_counter, expected as i64,
                "scenario limit={limit}, workers={workers}, window={window}: counter={in_counter}, expected={expected}"
            );
        }
    }

    /// Wall-clock benchmark of the over-limit storm. Pre-seeds the row to
    /// `limit` and spawns N workers; all must reject. Reports elapsed.
    /// Useful for comparing legacy FOR-UPDATE-on-every-pull vs the lockless
    /// precheck — same setup, just toggle the function under test by checking
    /// out the legacy file before running.
    #[sqlx::test(migrations = "../migrations")]
    async fn bench_over_limit_storm(db: Pool<Postgres>) {
        let key = "bench-over-limit-storm".to_string();
        let limit: i32 = 5;
        let workers: usize = 200;

        let mut filled = serde_json::Map::new();
        for _ in 0..limit {
            filled.insert(Uuid::new_v4().hyphenated().to_string(), json!({}));
        }
        sqlx::query!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids) VALUES ($1, $2)",
            key,
            serde_json::Value::Object(filled),
        )
        .execute(&db)
        .await
        .expect("seed at limit");

        let start = std::time::Instant::now();
        let mut tasks = Vec::with_capacity(workers);
        for _ in 0..workers {
            let db = db.clone();
            let key = key.clone();
            let job_id = Uuid::new_v4();
            tasks.push(tokio::spawn(async move {
                update_concurrency_counter(
                    &db,
                    &job_id,
                    key,
                    init_uuids(job_id),
                    job_id.hyphenated().to_string(),
                    60,
                    limit,
                )
                .await
            }));
        }
        let mut admitted = 0;
        for t in tasks {
            let (within, _) = t.await.expect("join").expect("call");
            if within {
                admitted += 1;
            }
        }
        let elapsed = start.elapsed();
        eprintln!(
            "BENCH over_limit_storm: {workers} workers, limit={limit}, elapsed={:?}, admitted={admitted}",
            elapsed
        );
        assert_eq!(admitted, 0, "over-limit storm must admit zero");
    }

    /// Long-haul: 500 in-process iterations of the worst storm scenario.
    /// Each iteration uses a fresh key. If any single iteration over-admits,
    /// fail with the offending iteration index.
    ///
    /// This is a single test rather than 500 cargo invocations so the cost
    /// is one DB setup, not 500. Total ~30s.
    #[sqlx::test(migrations = "../migrations")]
    async fn long_haul_stress(db: Pool<Postgres>) {
        let limit: i32 = 2;
        let workers: usize = 100;
        let iterations: usize = 500;
        for i in 0..iterations {
            let key = format!("long-haul-{i}");
            let (admitted, in_counter) = run_stress(&db, &key, limit, workers, 60).await;
            assert_eq!(
                admitted, limit as usize,
                "iter {i}: admitted={admitted}, expected={limit}"
            );
            assert_eq!(
                in_counter, limit as i64,
                "iter {i}: counter={in_counter}, expected={limit}"
            );
        }
    }

    /// Race: workers admit while a separate task removes entries from
    /// `job_uuids` (mimicking complete_job's `job_uuids - $2` delete).
    /// Must not over-admit and must not deadlock.
    ///
    /// Setup: seed the row already at limit (`limit` keys in job_uuids).
    /// Spawn N admit-attempts and a deleter that removes one key every few ms.
    /// Each delete frees a slot, so at most `deletions` admits should succeed.
    /// We assert: admitted <= deletions + 0 (since precheck reads the count
    /// after each delete, never below the previous floor) and the final
    /// counter ≤ limit (never exceeds).
    #[sqlx::test(migrations = "../migrations")]
    async fn race_admit_vs_delete(db: Pool<Postgres>) {
        let key = "race-admit-vs-delete".to_string();
        let limit: i32 = 5;

        let mut seeded_keys = Vec::new();
        let mut filled = serde_json::Map::new();
        for _ in 0..limit {
            let id = Uuid::new_v4().hyphenated().to_string();
            filled.insert(id.clone(), json!({}));
            seeded_keys.push(id);
        }
        sqlx::query!(
            "INSERT INTO concurrency_counter(concurrency_id, job_uuids) VALUES ($1, $2)",
            key,
            serde_json::Value::Object(filled),
        )
        .execute(&db)
        .await
        .expect("seed at limit");

        let workers: usize = 50;
        let mut tasks = Vec::with_capacity(workers + 1);
        for _ in 0..workers {
            let db = db.clone();
            let key = key.clone();
            let job_id = Uuid::new_v4();
            tasks.push(tokio::spawn(async move {
                let r = update_concurrency_counter(
                    &db,
                    &job_id,
                    key,
                    init_uuids(job_id),
                    job_id.hyphenated().to_string(),
                    60,
                    limit,
                )
                .await;
                r.map(|(w, _)| w)
            }));
        }

        // Deleter: removes seeded keys one at a time with small spacing,
        // mimicking complete_job. Each delete creates a slot.
        let deleter_db = db.clone();
        let deleter_key = key.clone();
        let deletions = seeded_keys.len();
        let deleter = tokio::spawn(async move {
            for k in seeded_keys {
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                sqlx::query!(
                    "UPDATE concurrency_counter SET job_uuids = job_uuids - $2 WHERE concurrency_id = $1",
                    deleter_key,
                    k,
                )
                .execute(&deleter_db)
                .await
                .expect("delete entry");
            }
        });

        let mut admitted = 0;
        for t in tasks {
            if t.await.expect("join").expect("call") {
                admitted += 1;
            }
        }
        deleter.await.expect("deleter");

        let in_counter = admitted_in_counter(&db, &key).await;

        // The deleter freed exactly `deletions` slots, but only one at a time
        // and only after Phase 2 admits already had the chance to fill them.
        // Final invariant: counter ≤ limit, and admits ≤ deletions.
        assert!(
            in_counter <= limit as i64,
            "counter ({in_counter}) must never exceed limit ({limit})"
        );
        assert!(
            admitted <= deletions,
            "admitted ({admitted}) must not exceed slots freed ({deletions})"
        );
    }
}
