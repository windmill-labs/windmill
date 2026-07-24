// Integration tests for native single-script retry (no one-step-flow wrapping).
//
// These use the *runtime* sqlx API (`sqlx::query`/`query_as`, not the `!` macros)
// like schedule_push.rs, so they need no `.sqlx` offline cache entry.
mod native_retry {
    use sqlx::{Pool, Postgres};
    use uuid::Uuid;

    use windmill_common::flows::{ConstantDelay, Retry};
    use windmill_common::jobs::{JobKind, JobTriggerKind};
    use windmill_common::runnable_settings::{
        from_handle, insert_rs, ConcurrencySettings, RetrySettings, RunnableSettings,
        RunnableSettingsTrait,
    };
    use windmill_common::scripts::{ScriptHash, ScriptLang};
    use windmill_common::users::username_to_permissioned_as;
    use windmill_queue::jobs::{maybe_enqueue_native_script_retry, MiniCompletedJob};

    const WS: &str = "test-workspace";
    const SCHED: &str = "f/system/test_schedule";
    const SCRIPT: &str = "f/system/test_script";

    fn mini(id: Uuid, parent_job: Option<Uuid>, handle: Option<i64>) -> MiniCompletedJob {
        MiniCompletedJob {
            id,
            workspace_id: WS.to_string(),
            runnable_id: Some(ScriptHash(100001)),
            scheduled_for: chrono::Utc::now(),
            parent_job,
            flow_innermost_root_job: None,
            runnable_path: Some(SCRIPT.to_string()),
            kind: JobKind::Script,
            started_at: Some(chrono::Utc::now()),
            permissioned_as: username_to_permissioned_as("test-user"),
            created_by: "test-user".to_string(),
            script_lang: Some(ScriptLang::Deno),
            permissioned_as_email: "test@windmill.dev".to_string(),
            flow_step_id: None,
            trigger_kind: Some(JobTriggerKind::Schedule),
            trigger: Some(SCHED.to_string()),
            priority: None,
            concurrent_limit: None,
            tag: "deno".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            runnable_settings_handle: handle,
        }
    }

    async fn count_retries(db: &Pool<Postgres>, root: Uuid) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM v2_job WHERE parent_job = $1")
            .bind(root)
            .fetch_one(db)
            .await
            .unwrap()
    }

    /// The queued retry with the given attempt number, if any: (id, kind, parent_job, backoff_s, handle).
    async fn retry_by_attempt(
        db: &Pool<Postgres>,
        root: Uuid,
        attempt: i64,
    ) -> Option<(Uuid, String, Option<Uuid>, f64, Option<i64>)> {
        sqlx::query_as::<_, (Uuid, String, Option<Uuid>, Option<f64>, Option<i64>)>(
            "SELECT j.id, j.kind::text, j.parent_job,
                    EXTRACT(EPOCH FROM (q.scheduled_for - now()))::float8,
                    q.runnable_settings_handle
             FROM v2_job j
             JOIN v2_job_queue q ON q.id = j.id
             JOIN native_retry_attempt nra ON nra.job_id = j.id
             WHERE j.parent_job = $1 AND nra.attempt = $2",
        )
        .bind(root)
        .bind(attempt)
        .fetch_optional(db)
        .await
        .unwrap()
        .map(|(id, kind, parent, backoff, handle)| {
            (id, kind, parent, backoff.unwrap_or(0.0), handle)
        })
    }

    fn no_result() -> Option<Box<serde_json::value::RawValue>> {
        None
    }

    // attempt0 -> retry1 -> retry2 -> exhausted, with crash-replay idempotency.
    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn chains_attempts_and_is_idempotent(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Policy: 2 constant attempts, 1s apart.
        let retry = Retry {
            constant: ConstantDelay { attempts: 2, seconds: 1 },
            exponential: Default::default(),
            retry_if: None,
        };
        let handle = insert_rs(
            RunnableSettings {
                debouncing_settings: None,
                concurrency_settings: None,
                retry_settings: RetrySettings::from(&retry).insert_cached(&db).await?,
            },
            &db,
        )
        .await?;
        assert!(handle.is_some(), "a retry policy must produce a handle");

        // attempt 0 (the schedule root); maybe_enqueue tolerates the absent queue row (counter -> 0).
        let root_id = Uuid::new_v4();
        let root = mini(root_id, None, handle);

        // First failure -> retry 1.
        assert!(
            maybe_enqueue_native_script_retry(&db, &root, &None, &no_result).await?,
            "first failure enqueues a retry"
        );
        let (r1_id, kind, parent, backoff, r1_handle) = retry_by_attempt(&db, root_id, 1)
            .await
            .expect("retry attempt 1 exists");
        assert_eq!(
            kind, "script",
            "retry is a native Script, not a singlestepflow"
        );
        assert_eq!(parent, Some(root_id), "retry links to the chain root");
        assert!(
            r1_handle.is_some(),
            "retry carries the policy for further chaining"
        );
        assert!(
            backoff > 0.0 && backoff <= 3.0,
            "constant 1s backoff, got {backoff}s"
        );

        // Crash-replay: the SAME completion again must not double-enqueue, and must
        // still report pending (so schedule handlers stay deferred). Regression for P1.
        assert!(
            maybe_enqueue_native_script_retry(&db, &root, &None, &no_result).await?,
            "replay still reports the retry as pending"
        );
        assert_eq!(
            count_retries(&db, root_id).await,
            1,
            "no double retry on crash replay"
        );

        // retry 1 fails -> retry 2 (still within attempts = 2).
        let r1 = mini(r1_id, Some(root_id), r1_handle);
        assert!(maybe_enqueue_native_script_retry(&db, &r1, &None, &no_result).await?);
        assert_eq!(count_retries(&db, root_id).await, 2);

        // retry 2 fails -> attempts exhausted, no retry 3.
        let (r2_id, _, _, _, r2_handle) = retry_by_attempt(&db, root_id, 2)
            .await
            .expect("retry attempt 2 exists");
        let r2 = mini(r2_id, Some(root_id), r2_handle);
        assert!(
            !maybe_enqueue_native_script_retry(&db, &r2, &None, &no_result).await?,
            "exhausted policy does not enqueue"
        );
        assert_eq!(
            count_retries(&db, root_id).await,
            2,
            "no retry past max attempts"
        );
        Ok(())
    }

    // Cancellation always wins over a pending retry.
    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn canceled_job_does_not_retry(db: Pool<Postgres>) -> anyhow::Result<()> {
        let retry = Retry {
            constant: ConstantDelay { attempts: 3, seconds: 1 },
            exponential: Default::default(),
            retry_if: None,
        };
        let handle = insert_rs(
            RunnableSettings {
                debouncing_settings: None,
                concurrency_settings: None,
                retry_settings: RetrySettings::from(&retry).insert_cached(&db).await?,
            },
            &db,
        )
        .await?;
        let root_id = Uuid::new_v4();
        let root = mini(root_id, None, handle);
        let canceled = Some(windmill_queue::jobs::CanceledBy {
            username: Some("test-user".to_string()),
            reason: Some("manual".to_string()),
        });
        assert!(!maybe_enqueue_native_script_retry(&db, &root, &canceled, &no_result).await?);
        assert_eq!(
            count_retries(&db, root_id).await,
            0,
            "canceled job must not retry"
        );
        Ok(())
    }

    // A concurrency-limited script that also retries must carry its concurrency
    // settings into each retry, otherwise the retry runs unbounded. Regression: P1.
    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn retry_preserves_concurrency_settings(db: Pool<Postgres>) -> anyhow::Result<()> {
        let retry = Retry {
            constant: ConstantDelay { attempts: 1, seconds: 0 },
            exponential: Default::default(),
            retry_if: None,
        };
        let concurrency = ConcurrencySettings {
            concurrency_key: Some("f/system/test_script".to_string()),
            concurrent_limit: Some(1),
            concurrency_time_window_s: Some(60),
        };
        let handle = insert_rs(
            RunnableSettings {
                debouncing_settings: None,
                concurrency_settings: concurrency.insert_cached(&db).await?,
                retry_settings: RetrySettings::from(&retry).insert_cached(&db).await?,
            },
            &db,
        )
        .await?;

        let root_id = Uuid::new_v4();
        let root = mini(root_id, None, handle);
        assert!(maybe_enqueue_native_script_retry(&db, &root, &None, &no_result).await?);

        let (_id, _kind, _parent, _backoff, r1_handle) = retry_by_attempt(&db, root_id, 1)
            .await
            .expect("retry attempt 1 exists");
        // The retry's own handle must resolve to the same concurrency settings, not
        // just the retry policy — otherwise it would run with no concurrency_key.
        let rs = from_handle(r1_handle, &db).await?;
        let resolved = ConcurrencySettings::get(
            rs.concurrency_settings
                .expect("retry must carry concurrency settings forward"),
            &db,
        )
        .await?;
        assert_eq!(
            resolved.concurrency_key.as_deref(),
            Some("f/system/test_script")
        );
        assert_eq!(resolved.concurrent_limit, Some(1));
        assert!(
            rs.retry_settings.is_some(),
            "retry policy is still carried for further chaining"
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Per-occurrence terminal status (drives on_failure_times / on_recovery).
    // Mirrors the exact query in windmill-ee-private jobs_ee::apply_schedule_handlers.
    // ------------------------------------------------------------------
    // `is_retry` marks the seeded job as a native retry attempt (the explicit
    // native_retry_attempt marker), the same signal `apply_schedule_handlers`
    // keys off. Handlers / WAC inline children are seeded without it.
    async fn seed_job(
        db: &Pool<Postgres>,
        id: Uuid,
        parent: Option<Uuid>,
        is_retry: bool,
        status: &str,
    ) {
        sqlx::query(
            "INSERT INTO v2_job (id, workspace_id, kind, runnable_path, trigger_kind, trigger, parent_job)
             VALUES ($1, $2, 'script', $3, 'schedule', $4, $5)",
        )
        .bind(id)
        .bind(WS)
        .bind(SCRIPT)
        .bind(SCHED)
        .bind(parent)
        .execute(db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, status)
             VALUES ($1, $2, 1, $3::job_status)",
        )
        .bind(id)
        .bind(WS)
        .bind(status)
        .execute(db)
        .await
        .unwrap();
        if is_retry {
            sqlx::query("INSERT INTO native_retry_attempt (job_id, attempt) VALUES ($1, 1)")
                .bind(id)
                .execute(db)
                .await
                .unwrap();
        }
    }

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn per_occurrence_status_counts_recovered_as_success(db: Pool<Postgres>) {
        // A native retry attempt is marked (native_retry_attempt); handler and WAC
        // inline children are parented Script children WITHOUT the marker.
        // a: root fails, a marked retry succeeds   -> RECOVERED -> success
        // b: root fails, retry also fails          -> failure
        // c: root succeeds directly                -> success
        // e: root fails, only its on_failure HANDLER (unmarked) succeeds
        //    -> failure: handler child must NOT count as a recovery
        // f: root fails, only a WAC inline child (unmarked) succeeds
        //    -> failure: WAC inline child must NOT count as a recovery
        // d: the current occurrence (excluded by `j.id != $4`)
        let (a, b, c, e, f, d) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        seed_job(&db, a, None, false, "failure").await;
        seed_job(&db, Uuid::new_v4(), Some(a), true, "success").await; // a's retry attempt (marked)
        seed_job(&db, b, None, false, "failure").await;
        seed_job(&db, Uuid::new_v4(), Some(b), true, "failure").await; // b's failed retry
        seed_job(&db, c, None, false, "success").await;
        seed_job(&db, e, None, false, "failure").await;
        seed_job(&db, Uuid::new_v4(), Some(e), false, "success").await; // e's handler child (unmarked)
        seed_job(&db, f, None, false, "failure").await;
        seed_job(&db, Uuid::new_v4(), Some(f), false, "success").await; // f's WAC inline child (unmarked)
        seed_job(&db, d, None, false, "failure").await; // current occurrence

        // Exact expression from jobs_ee::apply_schedule_handlers: the EXISTS counts
        // only marked native retry children, so neither handler nor WAC inline
        // children count as a recovery.
        let rows = sqlx::query_as::<_, (Uuid, bool)>(
            "SELECT j.id, (status = 'success' OR EXISTS (
                    SELECT 1 FROM native_retry_attempt nra
                    JOIN v2_job jc ON jc.id = nra.job_id
                    JOIN v2_job_completed cc ON cc.id = nra.job_id
                    WHERE jc.parent_job = j.id AND cc.status = 'success'
                ))
             FROM v2_job j JOIN v2_job_completed USING (id)
             WHERE j.workspace_id = $1 AND trigger_kind = 'schedule' AND trigger = $2
                AND parent_job IS NULL AND runnable_path = $3 AND j.id != $4
             ORDER BY created_at DESC",
        )
        .bind(WS)
        .bind(SCHED)
        .bind(SCRIPT)
        .bind(d)
        .fetch_all(&db)
        .await
        .unwrap();

        let status: std::collections::HashMap<Uuid, bool> = rows.into_iter().collect();
        // Retries/handlers/WAC children (parent_job set) are NOT occurrences, and the
        // current one is excluded: exactly the five roots a, b, c, e, f remain.
        assert_eq!(
            status.len(),
            5,
            "child jobs excluded from occurrence counting; current excluded"
        );
        assert_eq!(
            status[&a], true,
            "recovered occurrence (same-runnable retry succeeded) = success"
        );
        assert_eq!(
            status[&b], false,
            "all-attempts-failed occurrence = failure"
        );
        assert_eq!(status[&c], true, "direct success");
        assert_eq!(
            status[&e], false,
            "on_failure handler success must NOT count as a recovery"
        );
        assert_eq!(
            status[&f], false,
            "WAC inline child success must NOT count as a recovery"
        );
    }

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn succeeding_retry_resolves_the_attempts_it_superseded(db: Pool<Postgres>) {
        // a: root fails, its marked retry succeeds -> both resolve, status stays 'failure'
        // a_sibling: an unmarked child of the SAME root that failed on its own (a WAC
        //   inline job or an SDK-launched child) -> must stay unresolved. `root` is
        //   `parent_job.unwrap_or(id)`, so sharing a parent proves nothing about chain
        //   membership; resolving this would hide an unhandled failure.
        // b: an unrelated failure -> must stay unresolved
        // f: root fails, an unmarked child succeeds -> must not trigger any resolution
        let (a, a_retry, a_sibling, b, f, f_child) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        seed_job(&db, a, None, false, "failure").await;
        seed_job(&db, a_retry, Some(a), true, "failure").await;
        seed_job(&db, a_sibling, Some(a), false, "failure").await;
        seed_job(&db, b, None, false, "failure").await;
        seed_job(&db, f, None, false, "failure").await;
        seed_job(&db, f_child, Some(f), false, "success").await;

        // A human resolved the first attempt already: the automatic pass must not
        // overwrite their note.
        sqlx::query("INSERT INTO job_resolution (job_id, workspace_id, resolved_by, note) VALUES ($1, $2, 'ruben', 'flaky upstream')")
            .bind(a)
            .bind(WS)
            .execute(&db)
            .await
            .unwrap();

        let succeeded = Uuid::new_v4();
        seed_job(&db, succeeded, Some(a), true, "success").await;
        windmill_queue::jobs::resolve_superseded_retry_attempts(&db, a, WS, succeeded, None)
            .await
            .unwrap();
        // The unmarked child must not be able to resolve its own chain.
        windmill_queue::jobs::resolve_superseded_retry_attempts(&db, f, WS, f_child, None)
            .await
            .unwrap();

        let resolutions = sqlx::query_as::<_, (Uuid, Option<String>, Option<String>)>(
            "SELECT job_id, resolved_by, note FROM job_resolution WHERE workspace_id = $1",
        )
        .bind(WS)
        .fetch_all(&db)
        .await
        .unwrap();
        let by_id: std::collections::HashMap<_, _> = resolutions
            .into_iter()
            .map(|(id, by, note)| (id, (by, note)))
            .collect();

        assert_eq!(
            by_id.get(&a),
            Some(&(
                Some("ruben".to_string()),
                Some("flaky upstream".to_string())
            )),
            "an existing human resolution must survive the automatic pass"
        );
        assert_eq!(
            by_id.get(&a_retry),
            Some(&(None, None)),
            "the superseded attempt resolves automatically (resolved_by NULL)"
        );
        assert!(
            !by_id.contains_key(&a_sibling),
            "an unmarked failed child sharing the root is not part of the chain"
        );
        assert!(
            !by_id.contains_key(&b),
            "an unrelated failure must not be resolved"
        );
        assert!(
            !by_id.contains_key(&f),
            "an unmarked child succeeding must not resolve its parent"
        );

        // The helper writes through a privileged pool, so it must verify its own arguments
        // rather than trust the caller: a job that did not succeed resolves nothing.
        let (g, g_attempt) = (Uuid::new_v4(), Uuid::new_v4());
        seed_job(&db, g, None, false, "failure").await;
        seed_job(&db, g_attempt, Some(g), true, "failure").await;
        windmill_queue::jobs::resolve_superseded_retry_attempts(&db, g, WS, g_attempt, None)
            .await
            .unwrap();
        let g_resolved: i64 =
            sqlx::query_scalar("SELECT count(*) FROM job_resolution WHERE job_id IN ($1, $2)")
                .bind(g)
                .bind(g_attempt)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(g_resolved, 0, "a failed attempt must not resolve its chain");

        // Resolution is orthogonal: the run is still recorded as a failure.
        let status: String =
            sqlx::query_scalar("SELECT status::text FROM v2_job_completed WHERE id = $1")
                .bind(a_retry)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(status, "failure", "resolving must not change job status");
    }
}
