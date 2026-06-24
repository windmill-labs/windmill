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
        insert_rs, RetrySettings, RunnableSettings, RunnableSettingsTrait,
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
             FROM v2_job j JOIN v2_job_queue q ON q.id = j.id
             WHERE j.parent_job = $1 AND (q.extras->>'retry_attempt')::bigint = $2",
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

    // ------------------------------------------------------------------
    // Per-occurrence terminal status (drives on_failure_times / on_recovery).
    // Mirrors the exact query in windmill-ee-private jobs_ee::apply_schedule_handlers.
    // ------------------------------------------------------------------
    async fn seed_job(
        db: &Pool<Postgres>,
        id: Uuid,
        parent: Option<Uuid>,
        runnable_id: i64,
        flow_innermost: Option<Uuid>,
        status: &str,
    ) {
        sqlx::query(
            "INSERT INTO v2_job (id, workspace_id, kind, runnable_path, runnable_id, trigger_kind, trigger, parent_job, flow_innermost_root_job)
             VALUES ($1, $2, 'script', $3, $4, 'schedule', $5, $6, $7)",
        )
        .bind(id)
        .bind(WS)
        .bind(SCRIPT)
        .bind(runnable_id)
        .bind(SCHED)
        .bind(parent)
        .bind(flow_innermost)
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
    }

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn per_occurrence_status_counts_recovered_as_success(db: Pool<Postgres>) {
        // runnable 1 = the scheduled script. A native retry re-runs runnable 1 with
        // no flow_innermost_root_job; handlers run a different runnable; WAC inline
        // children re-run runnable 1 but carry a flow_innermost_root_job.
        // a: root fails, a same-runnable retry succeeds        -> RECOVERED -> success
        // b: root fails, retry also fails                      -> failure
        // c: root succeeds directly                            -> success
        // e: root fails, only its on_failure HANDLER (other runnable) succeeds
        //    -> failure: handler child must NOT count as a recovery
        // f: root fails, only a WAC inline child (same runnable, flow_innermost set)
        //    succeeds -> failure: WAC inline child must NOT count as a recovery
        // d: the current occurrence (excluded by `j.id != $4`)
        let (a, b, c, e, f, d) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        seed_job(&db, a, None, 1, None, "failure").await;
        seed_job(&db, Uuid::new_v4(), Some(a), 1, None, "success").await; // a's same-runnable retry
        seed_job(&db, b, None, 1, None, "failure").await;
        seed_job(&db, Uuid::new_v4(), Some(b), 1, None, "failure").await; // b's failed retry
        seed_job(&db, c, None, 1, None, "success").await;
        seed_job(&db, e, None, 1, None, "failure").await;
        seed_job(&db, Uuid::new_v4(), Some(e), 999, None, "success").await; // e's handler (other runnable)
        seed_job(&db, f, None, 1, None, "failure").await;
        seed_job(&db, Uuid::new_v4(), Some(f), 1, Some(f), "success").await; // f's WAC inline child
        seed_job(&db, d, None, 1, None, "failure").await; // current occurrence

        // Exact expression from jobs_ee::apply_schedule_handlers: the EXISTS is
        // scoped to same-runnable children with no flow_innermost_root_job, so
        // neither handler (other runnable) nor WAC inline (flow_innermost) children
        // count as a recovery — only native retry attempts do.
        let rows = sqlx::query_as::<_, (Uuid, bool)>(
            "SELECT j.id, (status = 'success' OR EXISTS (
                    SELECT 1 FROM v2_job_completed cc JOIN v2_job jc ON jc.id = cc.id
                    WHERE jc.parent_job = j.id AND jc.runnable_id = j.runnable_id
                        AND jc.flow_innermost_root_job IS NULL AND cc.status = 'success'
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
}
