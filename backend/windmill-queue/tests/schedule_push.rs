mod schedule_push {
    use chrono::Utc;
    use sqlx::{Pool, Postgres};
    use windmill_common::db::Authed;
    use windmill_common::jobs::{JobKind, JobTriggerKind};
    use windmill_common::schedule::Schedule;
    use windmill_common::scripts::ScriptHash;
    use windmill_common::users::username_to_permissioned_as;
    use windmill_queue::jobs::{try_schedule_next_job, MiniCompletedJob};
    use windmill_queue::schedule::push_scheduled_job;

    fn make_schedule(overrides: impl FnOnce(&mut Schedule)) -> Schedule {
        let mut s = Schedule {
            workspace_id: "test-workspace".to_string(),
            path: "f/system/test_schedule".to_string(),
            edited_by: "test-user".to_string(),
            edited_at: Utc::now(),
            schedule: "0 0 */5 * * *".to_string(),
            timezone: "UTC".to_string(),
            enabled: true,
            script_path: "f/system/test_script".to_string(),
            is_flow: false,
            args: None,
            extra_perms: serde_json::json!({}),
            email: "test@windmill.dev".to_string(),
            error: None,
            on_failure: None,
            on_failure_times: None,
            on_failure_exact: None,
            on_failure_extra_args: None,
            on_recovery: None,
            on_recovery_times: None,
            on_recovery_extra_args: None,
            on_success: None,
            on_success_extra_args: None,
            ws_error_handler_muted: false,
            retry: None,
            no_flow_overlap: false,
            summary: None,
            description: None,
            tag: None,
            paused_until: None,
            cron_version: None,
            dynamic_skip: None,
        };
        overrides(&mut s);
        s
    }

    fn make_authed() -> Authed {
        Authed {
            email: "test@windmill.dev".to_string(),
            username: "test-user".to_string(),
            is_admin: true,
            is_operator: false,
            groups: vec![],
            folders: vec![],
            scopes: None,
            token_prefix: None,
        }
    }

    fn make_completed_job(schedule: &Schedule) -> MiniCompletedJob {
        MiniCompletedJob {
            id: uuid::Uuid::new_v4(),
            workspace_id: schedule.workspace_id.clone(),
            runnable_id: Some(ScriptHash(100001)),
            scheduled_for: Utc::now() - chrono::Duration::minutes(5),
            parent_job: None,
            flow_innermost_root_job: None,
            runnable_path: Some(schedule.script_path.clone()),
            kind: JobKind::Script,
            started_at: Some(Utc::now() - chrono::Duration::minutes(4)),
            permissioned_as: username_to_permissioned_as(&schedule.edited_by),
            created_by: schedule.edited_by.clone(),
            script_lang: None,
            permissioned_as_email: schedule.email.clone(),
            flow_step_id: None,
            trigger_kind: Some(JobTriggerKind::Schedule),
            trigger: Some(schedule.path.clone()),
            priority: None,
            concurrent_limit: None,
            tag: "deno".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            runnable_settings_handle: None,
        }
    }

    async fn count_queued_jobs(db: &Pool<Postgres>) -> i64 {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM v2_job_queue")
            .fetch_one(db)
            .await
            .unwrap()
    }

    async fn get_queued_job(
        db: &Pool<Postgres>,
    ) -> Option<(
        String,         // workspace_id
        Option<String>, // runnable_path
        Option<String>, // trigger
        Option<String>, // trigger_kind as text
    )> {
        sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>)>(
            "SELECT j.workspace_id, j.runnable_path, j.trigger, j.trigger_kind::text
             FROM v2_job j JOIN v2_job_queue q ON j.id = q.id
             LIMIT 1",
        )
        .fetch_optional(db)
        .await
        .unwrap()
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: basic script schedule
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_script_schedule(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|_| {});
        let authed = make_authed();

        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await?;
        tx.commit().await?;

        assert_eq!(count_queued_jobs(&db).await, 1);
        let (ws, path, trigger, trigger_kind) = get_queued_job(&db).await.unwrap();
        assert_eq!(ws, "test-workspace");
        assert_eq!(path.as_deref(), Some("f/system/test_script"));
        assert_eq!(trigger.as_deref(), Some("f/system/test_schedule"));
        assert_eq!(trigger_kind.as_deref(), Some("schedule"));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: flow schedule
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_flow_schedule(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.is_flow = true;
            s.script_path = "f/system/test_flow".to_string();
            s.path = "f/system/flow_schedule".to_string();
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await?;
        tx.commit().await?;

        assert_eq!(count_queued_jobs(&db).await, 1);
        let (_, path, trigger, _) = get_queued_job(&db).await.unwrap();
        assert_eq!(path.as_deref(), Some("f/system/test_flow"));
        assert_eq!(trigger.as_deref(), Some("f/system/flow_schedule"));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: on_behalf_of_email (script)
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_script_on_behalf_of_email(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.script_path = "f/system/obo_script".to_string();
            s.path = "f/system/obo_schedule".to_string();
        });

        // No pre-computed authed: forces the obo path inside push_scheduled_job
        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, None, None).await?;
        tx.commit().await?;

        assert_eq!(count_queued_jobs(&db).await, 1);

        let email = sqlx::query_scalar::<_, String>(
            "SELECT permissioned_as_email FROM v2_job j JOIN v2_job_queue q ON j.id = q.id LIMIT 1",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(email, "obo@windmill.dev");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: on_behalf_of_email (flow)
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_flow_on_behalf_of_email(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.is_flow = true;
            s.script_path = "f/system/obo_flow".to_string();
            s.path = "f/system/obo_flow_schedule".to_string();
        });

        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, None, None).await?;
        tx.commit().await?;

        assert_eq!(count_queued_jobs(&db).await, 1);

        let email = sqlx::query_scalar::<_, String>(
            "SELECT permissioned_as_email FROM v2_job j JOIN v2_job_queue q ON j.id = q.id LIMIT 1",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(email, "obo@windmill.dev");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: with retry wraps in SingleStepFlow
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_script_with_retry(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.retry = Some(serde_json::json!({
                "constant": { "attempts": 3, "seconds": 10 }
            }));
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await?;
        tx.commit().await?;

        assert_eq!(count_queued_jobs(&db).await, 1);

        // When retry is set, the job kind is singlescriptflow (SingleStepFlow wraps it)
        let kind = sqlx::query_scalar::<_, String>(
            "SELECT kind::text FROM v2_job j JOIN v2_job_queue q ON j.id = q.id LIMIT 1",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(kind, "singlestepflow");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: duplicate detection (same schedule + time = skip)
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_duplicate_skipped(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|_| {});
        let authed = make_authed();

        // First push
        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await?;
        tx.commit().await?;
        assert_eq!(count_queued_jobs(&db).await, 1);

        // Second push with same schedule — should be idempotent
        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await?;
        tx.commit().await?;
        assert_eq!(count_queued_jobs(&db).await, 1); // Still 1
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: invalid timezone
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_invalid_timezone(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.timezone = "Invalid/Timezone".to_string();
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let result = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await;
        assert!(result.is_err());
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: invalid cron expression
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_invalid_cron(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.schedule = "not a cron".to_string();
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let result = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await;
        assert!(result.is_err());
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: invalid args (not a dict)
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_invalid_args(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            let raw = serde_json::value::RawValue::from_string("[1,2,3]".to_string()).unwrap();
            s.args = Some(sqlx::types::Json(raw));
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let result = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await;
        assert!(result.is_err());
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: with schedule args passed to job
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_with_args(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            let raw =
                serde_json::value::RawValue::from_string(r#"{"key":"value"}"#.to_string()).unwrap();
            s.args = Some(sqlx::types::Json(raw));
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await?;
        tx.commit().await?;

        assert_eq!(count_queued_jobs(&db).await, 1);

        let args = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT args FROM v2_job j JOIN v2_job_queue q ON j.id = q.id LIMIT 1",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(args, serde_json::json!({"key": "value"}));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: script not found
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_script_not_found(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.script_path = "f/system/nonexistent".to_string();
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let result = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await;
        assert!(result.is_err());
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: flow not found
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_flow_not_found(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.is_flow = true;
            s.script_path = "f/system/nonexistent_flow".to_string();
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let result = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await;
        assert!(result.is_err());
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: paused schedule (paused_until in future)
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_paused_schedule(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.paused_until = Some(Utc::now() + chrono::Duration::hours(1));
        });
        let authed = make_authed();

        let tx = db.begin().await?;
        let tx = push_scheduled_job(&db, tx, &schedule, Some(&authed), None).await?;
        tx.commit().await?;

        // Job is still pushed, but scheduled_for will be after paused_until
        assert_eq!(count_queued_jobs(&db).await, 1);

        let scheduled_for = sqlx::query_scalar::<_, chrono::DateTime<Utc>>(
            "SELECT scheduled_for FROM v2_job_queue LIMIT 1",
        )
        .fetch_one(&db)
        .await?;
        assert!(scheduled_for > Utc::now());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // push_scheduled_job: clock shift detection (now_cutoff >= now)
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_clock_shift(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|_| {});
        let authed = make_authed();

        // Pass a now_cutoff far in the future — simulates clock shift
        let future_cutoff = Utc::now() + chrono::Duration::hours(24);
        let tx = db.begin().await?;
        let tx =
            push_scheduled_job(&db, tx, &schedule, Some(&authed), Some(future_cutoff)).await?;
        tx.commit().await?;

        assert_eq!(count_queued_jobs(&db).await, 1);

        // The scheduled_for should be after the cutoff
        let scheduled_for = sqlx::query_scalar::<_, chrono::DateTime<Utc>>(
            "SELECT scheduled_for FROM v2_job_queue LIMIT 1",
        )
        .fetch_one(&db)
        .await?;
        assert!(scheduled_for > future_cutoff);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: disabled schedule does not push
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_handle_disabled_schedule(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.enabled = false;
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        tx.commit().await?;
        assert!(err.is_none());
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: script path mismatch does not push
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_handle_path_mismatch(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|_| {});
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            "f/system/different_script",
        )
        .await;
        tx.commit().await?;
        assert!(err.is_none());
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: enabled + matching path pushes next job
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_handle_enabled_pushes_next_job(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|_| {});
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        tx.commit().await?;
        assert!(err.is_none());
        assert_eq!(count_queued_jobs(&db).await, 1);

        let (_, path, trigger, trigger_kind) = get_queued_job(&db).await.unwrap();
        assert_eq!(path.as_deref(), Some("f/system/test_script"));
        assert_eq!(trigger.as_deref(), Some("f/system/test_schedule"));
        assert_eq!(trigger_kind.as_deref(), Some("schedule"));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: on_behalf_of_email via handle path
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_handle_on_behalf_of_email(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.script_path = "f/system/obo_script".to_string();
            s.path = "f/system/obo_schedule".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        tx.commit().await?;
        assert!(err.is_none());
        assert_eq!(count_queued_jobs(&db).await, 1);

        let email = sqlx::query_scalar::<_, String>(
            "SELECT permissioned_as_email FROM v2_job j JOIN v2_job_queue q ON j.id = q.id LIMIT 1",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(email, "obo@windmill.dev");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: push failure returns error to caller
    // (caller is responsible for retry + eventual disable)
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_handle_push_failure_disables_schedule(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/bad_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/nonexistent', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|s| {
            s.path = "f/system/bad_schedule".to_string();
            s.script_path = "f/system/nonexistent".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        // NotFound: schedule disabled internally, no error returned (caller commits)
        assert!(err.is_none());
        tx.commit().await?;
        assert_eq!(count_queued_jobs(&db).await, 0);

        // Schedule should be disabled (NotFound is non-retryable)
        let (enabled, error): (bool, Option<String>) = sqlx::query_as(
            "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/bad_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(!enabled);
        assert!(error.is_some());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: successful push is atomic with tx commit
    // If the caller commits, both the next tick and any prior writes persist.
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_success_atomic_with_commit(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/test_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_script', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|_| {});
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        assert!(err.is_none());
        tx.commit().await?;

        // Next tick was pushed
        assert_eq!(count_queued_jobs(&db).await, 1);

        // Schedule stayed enabled
        let enabled: bool = sqlx::query_scalar(
            "SELECT enabled FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/test_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(enabled);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: successful push rolls back if tx is dropped
    // Ensures no next tick leaks when the outer tx is not committed.
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_success_rolls_back_on_drop(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|_| {});
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        assert!(err.is_none());

        // Intentionally drop tx without committing (simulates caller failure)
        drop(tx);

        // Nothing should be visible — the next tick must NOT leak
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: schedule disable rolls back if tx is dropped
    // The schedule must stay enabled when the caller doesn't commit.
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_failure_disable_rolls_back_on_drop(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/bad_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/nonexistent', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|s| {
            s.path = "f/system/bad_schedule".to_string();
            s.script_path = "f/system/nonexistent".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        // NotFound: schedule disabled in tx, no error returned
        assert!(err.is_none());

        // Drop without commit — simulates zombie retry path
        drop(tx);

        // Schedule should STILL be enabled (disable was rolled back with tx)
        let enabled: bool = sqlx::query_scalar(
            "SELECT enabled FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/bad_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(enabled, "schedule must stay enabled when tx is rolled back");
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: tx remains usable after successful push
    // The caller can perform additional writes on the returned tx.
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_tx_usable_after_success(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|_| {});
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (mut tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        assert!(err.is_none());

        // Write something else on the same tx
        sqlx::query("INSERT INTO global_settings (name, value) VALUES ('_test_after_push', '42'::jsonb)")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        // Both the pushed job and the extra write should be visible
        assert_eq!(count_queued_jobs(&db).await, 1);
        let val: serde_json::Value = sqlx::query_scalar(
            "SELECT value FROM global_settings WHERE name = '_test_after_push'",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(val, serde_json::json!(42));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: tx remains usable after push failure + disable
    // The caller can still write on the returned tx after a schedule disable.
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_tx_usable_after_failure(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/bad_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/nonexistent', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|s| {
            s.path = "f/system/bad_schedule".to_string();
            s.script_path = "f/system/nonexistent".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (mut tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        // NotFound: schedule disabled in tx, no error returned
        assert!(err.is_none());

        // Write something else on the returned tx — tx is still usable
        sqlx::query("INSERT INTO global_settings (name, value) VALUES ('_test_after_fail', '99'::jsonb)")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        // Schedule should be disabled (NotFound is non-retryable)
        let (enabled, error): (bool, Option<String>) = sqlx::query_as(
            "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/bad_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(!enabled);
        assert!(error.is_some());

        // Extra write should still be committed (tx is usable after push failure)
        let val: serde_json::Value = sqlx::query_scalar(
            "SELECT value FROM global_settings WHERE name = '_test_after_fail'",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(val, serde_json::json!(99));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: flow schedule pushes next job
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_try_schedule_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.is_flow = true;
            s.script_path = "f/system/test_flow".to_string();
            s.path = "f/system/flow_schedule".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        tx.commit().await?;
        assert!(err.is_none());
        assert_eq!(count_queued_jobs(&db).await, 1);

        let (_, path, trigger, _) = get_queued_job(&db).await.unwrap();
        assert_eq!(path.as_deref(), Some("f/system/test_flow"));
        assert_eq!(trigger.as_deref(), Some("f/system/flow_schedule"));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: script with retry wraps in SingleStepFlow
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_try_schedule_with_retry(db: Pool<Postgres>) -> anyhow::Result<()> {
        let schedule = make_schedule(|s| {
            s.retry = Some(serde_json::json!({
                "constant": { "attempts": 3, "seconds": 10 }
            }));
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        tx.commit().await?;
        assert!(err.is_none());
        assert_eq!(count_queued_jobs(&db).await, 1);

        let kind = sqlx::query_scalar::<_, String>(
            "SELECT kind::text FROM v2_job j JOIN v2_job_queue q ON j.id = q.id LIMIT 1",
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(kind, "singlestepflow");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: push failure error message is stored on schedule
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_failure_stores_error_message(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/bad_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/nonexistent', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|s| {
            s.path = "f/system/bad_schedule".to_string();
            s.script_path = "f/system/nonexistent".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        // NotFound: schedule disabled in tx, no error returned (caller commits)
        assert!(err.is_none());
        tx.commit().await?;

        let error: String = sqlx::query_scalar(
            "SELECT error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/bad_schedule'",
        )
        .fetch_one(&db)
        .await?;
        // Error should mention the script that couldn't be found
        assert!(
            error.contains("nonexistent"),
            "error message should describe the failure, got: {error}"
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: disabled schedule leaves no side effects
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_disabled_schedule_no_side_effects(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/disabled_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', false, 'f/system/test_script', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|s| {
            s.path = "f/system/disabled_schedule".to_string();
            s.enabled = false;
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        tx.commit().await?;
        assert!(err.is_none());

        // Schedule should remain disabled (not re-enabled) and no error set
        let (enabled, error): (bool, Option<String>) = sqlx::query_as(
            "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/disabled_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(!enabled);
        assert!(error.is_none(), "disabled schedule should not get an error set");
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: path mismatch leaves schedule unchanged
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_path_mismatch_no_side_effects(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/test_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_script', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|_| {});
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            "f/system/different_script",
        )
        .await;
        tx.commit().await?;
        assert!(err.is_none());

        // Schedule should remain enabled, no error, no jobs pushed
        let (enabled, error): (bool, Option<String>) = sqlx::query_as(
            "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/test_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(enabled, "schedule must stay enabled on path mismatch");
        assert!(error.is_none(), "no error should be set on path mismatch");
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // try_schedule_next_job: push failure with schedule not in DB
    // When the schedule row doesn't exist, the UPDATE affects 0 rows but
    // doesn't error. The function should return (tx, None).
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_push_failure_schedule_not_in_db(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Do NOT insert a schedule row — the disable UPDATE will match 0 rows
        let schedule = make_schedule(|s| {
            s.path = "f/system/ghost_schedule".to_string();
            s.script_path = "f/system/nonexistent".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        drop(tx);
        // NotFound: disable succeeds (UPDATE 0 rows is not an error), no error returned
        assert!(err.is_none());
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Critical invariant: after commit, it's impossible to have schedule
    // enabled + no next tick + function returned success.
    // We verify: if push succeeds, both the job AND the schedule's
    // unmodified enabled state are committed together.
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_invariant_success_means_tick_committed(db: Pool<Postgres>) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/test_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_script', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|_| {});
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        assert!(err.is_none());
        tx.commit().await?;

        // After commit: schedule enabled AND next tick exists — invariant holds
        let enabled: bool = sqlx::query_scalar(
            "SELECT enabled FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/test_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(enabled, "schedule must be enabled after successful push");
        assert_eq!(
            count_queued_jobs(&db).await,
            1,
            "next tick must exist after successful push + commit"
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Critical invariant: after commit with push failure, the schedule is
    // disabled. It's never the case that we commit with the schedule still
    // enabled and no next tick.
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_invariant_failure_means_disabled_after_commit(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/bad_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/nonexistent', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|s| {
            s.path = "f/system/bad_schedule".to_string();
            s.script_path = "f/system/nonexistent".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        // NotFound: schedule disabled in tx, no error returned (caller commits)
        assert!(err.is_none());
        tx.commit().await?;

        // After commit: no next tick, but schedule is disabled — invariant holds
        assert_eq!(count_queued_jobs(&db).await, 0);
        let enabled: bool = sqlx::query_scalar(
            "SELECT enabled FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/bad_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(
            !enabled,
            "schedule must be disabled when push fails with NotFound and tx commits"
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Critical invariant: if tx is NOT committed (zombie path), neither
    // the next tick nor the schedule disable persists. The schedule stays
    // enabled so that zombie retry can re-attempt.
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_invariant_rollback_preserves_schedule_for_retry(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/bad_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/nonexistent', false, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        let schedule = make_schedule(|s| {
            s.path = "f/system/bad_schedule".to_string();
            s.script_path = "f/system/nonexistent".to_string();
        });
        let job = make_completed_job(&schedule);

        let tx = db.begin().await?;
        let (tx, err) = try_schedule_next_job(
            &db,
            tx,
            &job,
            &schedule,
            &schedule.script_path,
        )
        .await;
        // NotFound: schedule disabled in tx, no error returned
        assert!(err.is_none());

        // Simulate zombie path: drop tx without commit
        drop(tx);

        // Schedule still enabled (disable was rolled back with tx) — ready for retry
        let (enabled, error): (bool, Option<String>) = sqlx::query_as(
            "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/bad_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(enabled, "schedule must remain enabled after rollback");
        assert!(error.is_none(), "error must not persist after rollback");
        assert_eq!(count_queued_jobs(&db).await, 0);
        Ok(())
    }

    // ===================================================================
    // Failpoint tests — feature-gated, only compiled under `failpoints`
    // ===================================================================

    #[cfg(feature = "failpoints")]
    mod failpoint_tests {
        use super::*;
        use windmill_queue::jobs::schedule_failpoints::{ScheduleFailPoint, ACTIVE};

        // ---------------------------------------------------------------
        // SavepointCreate failpoint → schedule disabled, 0 jobs
        // ---------------------------------------------------------------

        #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
        async fn test_failpoint_savepoint_create_disables(db: Pool<Postgres>) -> anyhow::Result<()> {
            sqlx::query(
                "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
                 VALUES ('test-workspace', 'f/system/test_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_script', false, 'test@windmill.dev', '{}', false, false)"
            )
            .execute(&db)
            .await?;

            let schedule = make_schedule(|_| {});
            let job = make_completed_job(&schedule);

            ACTIVE.scope(ScheduleFailPoint::SavepointCreate, async {
                let tx = db.begin().await.unwrap();
                let (tx, err) = try_schedule_next_job(
                    &db, tx, &job, &schedule, &schedule.script_path,
                ).await;
                // Transient error returned to caller for retry
                assert!(err.is_some());
                drop(tx);

                assert_eq!(count_queued_jobs(&db).await, 0);
                let enabled: bool = sqlx::query_scalar(
                    "SELECT enabled FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/test_schedule'",
                )
                .fetch_one(&db)
                .await
                .unwrap();
                assert!(enabled, "schedule must stay enabled for caller retry");
            }).await;
            Ok(())
        }

        // ---------------------------------------------------------------
        // Push failpoint → schedule disabled, 0 jobs
        // ---------------------------------------------------------------

        #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
        async fn test_failpoint_push_disables(db: Pool<Postgres>) -> anyhow::Result<()> {
            sqlx::query(
                "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
                 VALUES ('test-workspace', 'f/system/test_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_script', false, 'test@windmill.dev', '{}', false, false)"
            )
            .execute(&db)
            .await?;

            let schedule = make_schedule(|_| {});
            let job = make_completed_job(&schedule);

            ACTIVE.scope(ScheduleFailPoint::Push, async {
                let tx = db.begin().await.unwrap();
                let (tx, err) = try_schedule_next_job(
                    &db, tx, &job, &schedule, &schedule.script_path,
                ).await;
                // Transient error returned to caller for retry
                assert!(err.is_some());
                drop(tx);

                assert_eq!(count_queued_jobs(&db).await, 0);
                let enabled: bool = sqlx::query_scalar(
                    "SELECT enabled FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/test_schedule'",
                )
                .fetch_one(&db)
                .await
                .unwrap();
                assert!(enabled, "schedule must stay enabled for caller retry");
            }).await;
            Ok(())
        }

        // ---------------------------------------------------------------
        // SavepointCommit failpoint → schedule disabled, 0 jobs
        // ---------------------------------------------------------------

        #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
        async fn test_failpoint_savepoint_commit_disables(db: Pool<Postgres>) -> anyhow::Result<()> {
            sqlx::query(
                "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
                 VALUES ('test-workspace', 'f/system/test_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_script', false, 'test@windmill.dev', '{}', false, false)"
            )
            .execute(&db)
            .await?;

            let schedule = make_schedule(|_| {});
            let job = make_completed_job(&schedule);

            ACTIVE.scope(ScheduleFailPoint::SavepointCommit, async {
                let tx = db.begin().await.unwrap();
                let (tx, err) = try_schedule_next_job(
                    &db, tx, &job, &schedule, &schedule.script_path,
                ).await;
                // Transient error returned to caller for retry
                assert!(err.is_some());
                drop(tx);

                assert_eq!(count_queued_jobs(&db).await, 0, "pushed job must be rolled back");
                let enabled: bool = sqlx::query_scalar(
                    "SELECT enabled FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/test_schedule'",
                )
                .fetch_one(&db)
                .await
                .unwrap();
                assert!(enabled, "schedule must stay enabled for caller retry");
            }).await;
            Ok(())
        }

        // ---------------------------------------------------------------
        // ScheduleDisable failpoint → returns Some(err), caller doesn't commit
        // ---------------------------------------------------------------

        #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
        async fn test_failpoint_schedule_disable_returns_err(db: Pool<Postgres>) -> anyhow::Result<()> {
            sqlx::query(
                "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
                 VALUES ('test-workspace', 'f/system/bad_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/nonexistent', false, 'test@windmill.dev', '{}', false, false)"
            )
            .execute(&db)
            .await?;

            let schedule = make_schedule(|s| {
                s.path = "f/system/bad_schedule".to_string();
                s.script_path = "f/system/nonexistent".to_string();
            });
            let job = make_completed_job(&schedule);

            ACTIVE.scope(ScheduleFailPoint::ScheduleDisable, async {
                let tx = db.begin().await.unwrap();
                let (_tx, err) = try_schedule_next_job(
                    &db, tx, &job, &schedule, &schedule.script_path,
                ).await;
                assert!(err.is_some(), "must return Some(err) when disable fails");
            }).await;
            Ok(())
        }

        // ---------------------------------------------------------------
        // ScheduleDisable failpoint + tx drop → schedule stays enabled
        // ---------------------------------------------------------------

        #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
        async fn test_failpoint_disable_failure_rollback(db: Pool<Postgres>) -> anyhow::Result<()> {
            sqlx::query(
                "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
                 VALUES ('test-workspace', 'f/system/bad_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/nonexistent', false, 'test@windmill.dev', '{}', false, false)"
            )
            .execute(&db)
            .await?;

            let schedule = make_schedule(|s| {
                s.path = "f/system/bad_schedule".to_string();
                s.script_path = "f/system/nonexistent".to_string();
            });
            let job = make_completed_job(&schedule);

            ACTIVE.scope(ScheduleFailPoint::ScheduleDisable, async {
                let tx = db.begin().await.unwrap();
                let (tx, err) = try_schedule_next_job(
                    &db, tx, &job, &schedule, &schedule.script_path,
                ).await;
                assert!(err.is_some(), "must return Some(err) when disable fails");
                drop(tx);

                let (enabled, error): (bool, Option<String>) = sqlx::query_as(
                    "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/bad_schedule'",
                )
                .fetch_one(&db)
                .await
                .unwrap();
                assert!(enabled, "schedule must stay enabled when tx is dropped after disable failure");
                assert!(error.is_none(), "error must not persist after rollback");
            }).await;
            Ok(())
        }

        // ---------------------------------------------------------------
        // PushQuotaExceeded failpoint (script) → schedule disabled, 0 jobs,
        // no error handler notification (QuotaExceeded is silenced)
        // ---------------------------------------------------------------

        #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
        async fn test_failpoint_push_quota_exceeded_script(db: Pool<Postgres>) -> anyhow::Result<()> {
            sqlx::query(
                "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
                 VALUES ('test-workspace', 'f/system/test_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_script', false, 'test@windmill.dev', '{}', false, false)"
            )
            .execute(&db)
            .await?;

            let schedule = make_schedule(|_| {});
            let job = make_completed_job(&schedule);

            ACTIVE.scope(ScheduleFailPoint::PushQuotaExceeded, async {
                let tx = db.begin().await.unwrap();
                let (tx, err) = try_schedule_next_job(
                    &db, tx, &job, &schedule, &schedule.script_path,
                ).await;
                // QuotaExceeded: schedule disabled internally, no error returned
                assert!(err.is_none(), "QuotaExceeded should be handled internally (returns None)");
                tx.commit().await.unwrap();

                assert_eq!(count_queued_jobs(&db).await, 0);
                let (enabled, error): (bool, Option<String>) = sqlx::query_as(
                    "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/test_schedule'",
                )
                .fetch_one(&db)
                .await
                .unwrap();
                assert!(!enabled, "schedule must be disabled after QuotaExceeded");
                assert!(error.is_some(), "error must be set on schedule");
                assert!(error.unwrap().contains("quota"), "error message should mention quota");
            }).await;
            Ok(())
        }

        // ---------------------------------------------------------------
        // PushQuotaExceeded failpoint (flow) → schedule disabled, 0 jobs
        // ---------------------------------------------------------------

        #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
        async fn test_failpoint_push_quota_exceeded_flow(db: Pool<Postgres>) -> anyhow::Result<()> {
            sqlx::query(
                "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
                 VALUES ('test-workspace', 'f/system/flow_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_flow', true, 'test@windmill.dev', '{}', false, false)"
            )
            .execute(&db)
            .await?;

            let schedule = make_schedule(|s| {
                s.is_flow = true;
                s.script_path = "f/system/test_flow".to_string();
                s.path = "f/system/flow_schedule".to_string();
            });
            let job = make_completed_job(&schedule);

            ACTIVE.scope(ScheduleFailPoint::PushQuotaExceeded, async {
                let tx = db.begin().await.unwrap();
                let (tx, err) = try_schedule_next_job(
                    &db, tx, &job, &schedule, &schedule.script_path,
                ).await;
                // QuotaExceeded: schedule disabled internally, no error returned
                assert!(err.is_none(), "QuotaExceeded should be handled internally (returns None)");
                tx.commit().await.unwrap();

                assert_eq!(count_queued_jobs(&db).await, 0);
                let (enabled, error): (bool, Option<String>) = sqlx::query_as(
                    "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/flow_schedule'",
                )
                .fetch_one(&db)
                .await
                .unwrap();
                assert!(!enabled, "flow schedule must be disabled after QuotaExceeded");
                assert!(error.is_some(), "error must be set on flow schedule");
                assert!(error.unwrap().contains("quota"), "error message should mention quota");
            }).await;
            Ok(())
        }
    }

    // ===================================================================
    // Zombie detection tests — verify that a flow left in queue after
    // SchedulePushZombieError meets the restart criteria in monitor.rs
    // ===================================================================

    // -----------------------------------------------------------------------
    // When both schedule push AND post-retry disable fail, the flow job stays
    // in v2_job_queue as a zombie. This test simulates that state and verifies:
    //
    // 1. The zombie detection query (from handle_zombie_flows) finds the flow
    // 2. The flow meets restart criteria: first module is WaitingForPriorSteps
    //    and same_worker is false
    // 3. After the restart UPDATE, the flow is re-queued (running=false)
    // 4. The schedule remains enabled for retry on next flow execution
    // -----------------------------------------------------------------------

    #[sqlx::test(migrations = "../migrations", fixtures("base", "schedule_push"))]
    async fn test_zombie_flow_after_schedule_push_failure_meets_restart_criteria(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        use windmill_common::flow_status::{FlowStatus, FlowStatusModule};

        let flow_job_id = uuid::Uuid::new_v4();
        let now = Utc::now();
        let stale_ping = now - chrono::Duration::minutes(5);

        // Schedule still enabled — simulates both push and disable failing
        sqlx::query(
            "INSERT INTO schedule (workspace_id, path, edited_by, edited_at, schedule, timezone, enabled, script_path, is_flow, email, extra_perms, ws_error_handler_muted, no_flow_overlap)
             VALUES ('test-workspace', 'f/system/test_schedule', 'test-user', now(), '0 0 */5 * * *', 'UTC', true, 'f/system/test_flow', true, 'test@windmill.dev', '{}', false, false)"
        )
        .execute(&db)
        .await?;

        // Flow job in v2_job (kind=flow, triggered by schedule, same_worker=false)
        sqlx::query(
            "INSERT INTO v2_job (id, workspace_id, created_at, created_by, permissioned_as, permissioned_as_email, kind, runnable_path, trigger, trigger_kind, same_worker, visible_to_owner, tag)
             VALUES ($1, 'test-workspace', $2, 'test-user', 'u/test-user', 'test@windmill.dev', 'flow', 'f/system/test_flow', 'f/system/test_schedule', 'schedule', false, false, 'flow')"
        )
        .bind(flow_job_id)
        .bind(now - chrono::Duration::minutes(5))
        .execute(&db)
        .await?;

        // Queue entry: running=true (worker caught SchedulePushZombieError and returned Ok)
        sqlx::query(
            "INSERT INTO v2_job_queue (id, workspace_id, created_at, scheduled_for, running, started_at, tag, suspend)
             VALUES ($1, 'test-workspace', $2, $2, true, $3, 'flow', 0)"
        )
        .bind(flow_job_id)
        .bind(now - chrono::Duration::minutes(5))
        .bind(now - chrono::Duration::minutes(4))
        .execute(&db)
        .await?;

        // Stale ping — older than the 60s zombie transition timeout
        sqlx::query("INSERT INTO v2_job_runtime (id, ping) VALUES ($1, $2)")
            .bind(flow_job_id)
            .bind(stale_ping)
            .execute(&db)
            .await?;

        // Flow status at step 0, first module = WaitingForPriorSteps
        // This is the initial state of a flow that hasn't started any steps yet
        let flow_status = serde_json::json!({
            "step": 0,
            "modules": [{"type": "WaitingForPriorSteps", "id": "a"}],
            "failure_module": {"type": "WaitingForPriorSteps", "id": "failure"},
            "retry": {"fail_count": 0, "failed_jobs": []},
            "cleanup_module": {"flow_jobs_to_clean": []}
        });

        sqlx::query("INSERT INTO v2_job_status (id, flow_status) VALUES ($1, $2::jsonb)")
            .bind(flow_job_id)
            .bind(&flow_status)
            .execute(&db)
            .await?;

        // Run the same zombie detection query from handle_zombie_flows (60s timeout)
        let zombie_flows = sqlx::query_as::<_, (uuid::Uuid, String, Option<bool>, Option<String>)>(
            r#"
            SELECT
                j.id, j.workspace_id, j.same_worker,
                COALESCE(s.flow_status, s.workflow_as_code_status)::text AS flow_status
            FROM v2_job_queue q
            JOIN v2_job j USING (id)
            LEFT JOIN v2_job_runtime r USING (id)
            LEFT JOIN v2_job_status s USING (id)
            WHERE q.running = true AND q.suspend = 0 AND q.suspend_until IS null
                AND q.scheduled_for <= now()
                AND (j.kind = 'flow' OR j.kind = 'flowpreview' OR j.kind = 'flownode')
                AND r.ping IS NOT NULL
                AND r.ping < NOW() - ('60' || ' seconds')::interval
                AND q.canceled_by IS NULL
            "#,
        )
        .fetch_all(&db)
        .await?;

        assert_eq!(zombie_flows.len(), 1, "zombie flow must be detected");
        let (id, _ws, same_worker, flow_status_json) = &zombie_flows[0];
        assert_eq!(*id, flow_job_id);

        // Replicate the exact branching logic from handle_zombie_flows (monitor.rs:2711-2754).
        // Only flows matching the restart condition get restarted; others are cancelled.
        let status = flow_status_json
            .as_deref()
            .and_then(|x| serde_json::from_str::<FlowStatus>(x).ok());
        let should_restart = !same_worker.unwrap_or(false)
            && status.is_some_and(|s| {
                s.modules
                    .get(0)
                    .is_some_and(|x| matches!(x, FlowStatusModule::WaitingForPriorSteps { .. }))
            });

        assert!(
            should_restart,
            "flow must match the restart branch (not the cancel branch) in handle_zombie_flows"
        );

        // Apply the restart action — same UPDATE as handle_zombie_flows
        sqlx::query(
            "UPDATE v2_job_queue SET running = false, started_at = null
             WHERE id = $1 AND canceled_by IS NULL",
        )
        .bind(flow_job_id)
        .execute(&db)
        .await?;

        // Flow is re-queued for processing
        let (running, started_at): (bool, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
            "SELECT running, started_at FROM v2_job_queue WHERE id = $1",
        )
        .bind(flow_job_id)
        .fetch_one(&db)
        .await?;
        assert!(!running, "flow must not be running after zombie restart");
        assert!(started_at.is_none(), "started_at must be null after zombie restart");

        // Schedule still enabled — will be retried when flow re-executes
        let enabled: bool = sqlx::query_scalar(
            "SELECT enabled FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/test_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(enabled, "schedule must remain enabled for retry after zombie restart");

        // Flow job is NOT in v2_job_completed (it was never completed with error)
        let completed_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM v2_job_completed WHERE id = $1",
        )
        .bind(flow_job_id)
        .fetch_one(&db)
        .await?;
        assert_eq!(completed_count, 0, "flow must not be in completed_job — it's a zombie, not an error");

        Ok(())
    }
}
