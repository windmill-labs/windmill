mod common;

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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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

    #[sqlx::test(fixtures("base", "schedule_push"))]
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
    // try_schedule_next_job: push failure disables schedule
    // -----------------------------------------------------------------------

    #[sqlx::test(fixtures("base", "schedule_push"))]
    async fn test_handle_push_failure_disables_schedule(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Insert a schedule row so try_schedule_next_job can disable it
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
        tx.commit().await?;
        // Should succeed (error is handled internally by disabling schedule)
        assert!(err.is_none());
        assert_eq!(count_queued_jobs(&db).await, 0);

        // Schedule should be disabled with an error
        let (enabled, error): (bool, Option<String>) = sqlx::query_as(
            "SELECT enabled, error FROM schedule WHERE workspace_id = 'test-workspace' AND path = 'f/system/bad_schedule'",
        )
        .fetch_one(&db)
        .await?;
        assert!(!enabled);
        assert!(error.is_some());
        Ok(())
    }
}
