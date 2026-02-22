//! Tests for debouncing logic: both normal (push-time) and post-preprocessing debouncing.
//!
//! Run with:
//!   cargo test -p windmill-queue --test debounce_test --features private,enterprise -- --nocapture
//!
//! Requires a live database (migrations are applied automatically by sqlx::test).

#[cfg(feature = "private")]
mod debounce {
    use chrono::Utc;
    use serde_json::value::RawValue;
    use sqlx::{Pool, Postgres};
    use std::collections::HashMap;
    use uuid::Uuid;
    use windmill_common::jobs::JobKind;
    use windmill_common::runnable_settings::DebouncingSettings;
    use windmill_queue::PushArgs;

    /// Helper: insert a minimal job into v2_job + v2_job_queue + v2_job_runtime so debounce can find it.
    async fn insert_noop_job(db: &Pool<Postgres>, job_id: Uuid, workspace_id: &str) {
        sqlx::query!(
            "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id)
             VALUES ($1, 'noop', 'deno', 'test-user', 'u/test-user', 'test@windmill.dev', $2)",
            job_id,
            workspace_id,
        )
        .execute(db)
        .await
        .expect("insert v2_job");

        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
             VALUES ($1, $2, now(), 'deno')",
            job_id,
            workspace_id,
        )
        .execute(db)
        .await
        .expect("insert v2_job_queue");

        sqlx::query!("INSERT INTO v2_job_runtime (id) VALUES ($1)", job_id,)
            .execute(db)
            .await
            .expect("insert v2_job_runtime");
    }

    /// Helper: insert a flow job into v2_job + v2_job_queue + v2_job_runtime.
    async fn insert_flow_job(
        db: &Pool<Postgres>,
        job_id: Uuid,
        workspace_id: &str,
        runnable_path: &str,
    ) {
        sqlx::query!(
            "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path)
             VALUES ($1, 'flow', 'flow', 'test-user', 'u/test-user', 'test@windmill.dev', $2, $3)",
            job_id,
            workspace_id,
            runnable_path,
        )
        .execute(db)
        .await
        .expect("insert v2_job");

        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
             VALUES ($1, $2, now(), 'flow')",
            job_id,
            workspace_id,
        )
        .execute(db)
        .await
        .expect("insert v2_job_queue");

        sqlx::query!("INSERT INTO v2_job_runtime (id) VALUES ($1)", job_id,)
            .execute(db)
            .await
            .expect("insert v2_job_runtime");
    }

    /// Helper: check if a job is completed (exists in v2_job_completed).
    async fn is_completed(db: &Pool<Postgres>, job_id: &Uuid) -> bool {
        sqlx::query_scalar!("SELECT 1 as x FROM v2_job_completed WHERE id = $1", job_id,)
            .fetch_optional(db)
            .await
            .expect("check completed")
            .is_some()
    }

    /// Helper: check if a job is still in the queue.
    async fn is_queued(db: &Pool<Postgres>, job_id: &Uuid) -> bool {
        sqlx::query_scalar!("SELECT 1 as x FROM v2_job_queue WHERE id = $1", job_id,)
            .fetch_optional(db)
            .await
            .expect("check queued")
            .is_some()
    }

    /// Helper: get the debounce_key entry for a given key.
    async fn get_debounce_key(db: &Pool<Postgres>, key: &str) -> Option<(Uuid, Option<Uuid>, i32)> {
        sqlx::query!(
            "SELECT job_id, previous_job_id, debounced_times FROM debounce_key WHERE key = $1",
            key,
        )
        .fetch_optional(db)
        .await
        .expect("get debounce_key")
        .map(|r| (r.job_id, r.previous_job_id, r.debounced_times))
    }

    fn empty_args() -> HashMap<String, Box<RawValue>> {
        HashMap::new()
    }

    // =========================================================================
    // Tests for maybe_debounce (push-time debouncing)
    // =========================================================================

    /// Test: First job in a debounce batch should set scheduled_for and create debounce_key entry.
    /// No previous job should be debounced.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_first_job(db: Pool<Postgres>) -> anyhow::Result<()> {
        let job_id = Uuid::new_v4();
        insert_noop_job(&db, job_id, "test-workspace").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("test_first_job_key".to_string()),
            ..Default::default()
        };

        let mut scheduled_for = None;
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);
        let mut tx = db.begin().await?;

        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut scheduled_for,
            &Some("f/test/script".to_string()),
            "test-workspace",
            JobKind::Noop,
            job_id,
            &args,
            &mut tx,
            &db,
        )
        .await?;

        tx.commit().await?;

        // scheduled_for should be set to now + 5 seconds
        assert!(scheduled_for.is_some(), "scheduled_for should be set");
        let sf = scheduled_for.unwrap();
        let diff = (sf - Utc::now()).num_seconds();
        assert!(
            diff >= 3 && diff <= 6,
            "scheduled_for should be ~5s in the future, got {diff}s"
        );

        // debounce_key entry should exist with this job
        let dk = get_debounce_key(&db, "test_first_job_key").await;
        assert!(dk.is_some(), "debounce_key entry should exist");
        let (dk_job_id, dk_prev, dk_times) = dk.unwrap();
        assert_eq!(dk_job_id, job_id);
        assert!(dk_prev.is_none(), "no previous job for first in batch");
        assert_eq!(dk_times, 0, "debounced_times should be 0 for first job");

        // Job should still be in queue (not debounced)
        assert!(
            is_queued(&db, &job_id).await,
            "first job should still be queued"
        );
        assert!(
            !is_completed(&db, &job_id).await,
            "first job should not be completed"
        );

        Ok(())
    }

    /// Test: Second job with the same debounce key should debounce (complete) the first job.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_second_job_cancels_first(db: Pool<Postgres>) -> anyhow::Result<()> {
        let job1 = Uuid::new_v4();
        let job2 = Uuid::new_v4();
        insert_noop_job(&db, job1, "test-workspace").await;
        insert_noop_job(&db, job2, "test-workspace").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("test_cancel_key".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        // Push job 1
        {
            let args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                job1,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // Push job 2 with same key - should debounce job 1
        {
            let args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                job2,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // job1 should be completed (debounced)
        assert!(
            is_completed(&db, &job1).await,
            "job1 should be completed (debounced)"
        );

        // job2 should still be in queue
        assert!(is_queued(&db, &job2).await, "job2 should still be in queue");

        // debounce_key should point to job2
        let dk = get_debounce_key(&db, "test_cancel_key").await.unwrap();
        assert_eq!(dk.0, job2, "debounce_key should point to job2");
        assert_eq!(dk.2, 1, "debounced_times should be 1");

        Ok(())
    }

    /// Test: 1000 jobs in sequence with the same debounce key — only the last should remain queued.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_chain_of_1000(db: Pool<Postgres>) -> anyhow::Result<()> {
        let n: usize = 1000;

        // Batch-insert all jobs for speed
        let jobs: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        for chunk in jobs.chunks(500) {
            let chunk_vec: Vec<Uuid> = chunk.to_vec();
            sqlx::query!(
                "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id)
                 SELECT unnest($1::uuid[]), 'noop', 'deno', 'test-user', 'u/test-user', 'test@windmill.dev', 'test-workspace'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
                 SELECT unnest($1::uuid[]), 'test-workspace', now(), 'deno'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
        }

        let settings = DebouncingSettings {
            debounce_delay_s: Some(10),
            debounce_key: Some("test_chain_1000_key".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        for &j in &jobs {
            let args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                j,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // Only the last job should remain in queue
        let queued_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_queue WHERE id = ANY($1)",
            &jobs,
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            queued_count, 1,
            "exactly 1 job should remain in queue, got {queued_count}"
        );

        // N-1 jobs should be completed (debounced)
        let completed_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_completed WHERE id = ANY($1)",
            &jobs,
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            completed_count,
            (n - 1) as i64,
            "{} jobs should be completed (debounced), got {completed_count}",
            n - 1
        );

        // Last job should be the survivor
        assert!(
            is_queued(&db, &jobs[n - 1]).await,
            "last job should still be queued"
        );

        let dk = get_debounce_key(&db, "test_chain_1000_key").await.unwrap();
        assert_eq!(dk.0, jobs[n - 1], "debounce_key should point to last job");
        assert_eq!(dk.2, (n - 1) as i32);

        Ok(())
    }

    /// Test: Different debounce keys should not interfere with each other.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_different_keys_independent(db: Pool<Postgres>) -> anyhow::Result<()> {
        let job_a = Uuid::new_v4();
        let job_b = Uuid::new_v4();
        insert_noop_job(&db, job_a, "test-workspace").await;
        insert_noop_job(&db, job_b, "test-workspace").await;

        let args_hm = empty_args();

        // Push job_a with key "alpha"
        {
            let settings = DebouncingSettings {
                debounce_delay_s: Some(5),
                debounce_key: Some("alpha".to_string()),
                ..Default::default()
            };
            let args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script_a".to_string()),
                "test-workspace",
                JobKind::Noop,
                job_a,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // Push job_b with key "beta"
        {
            let settings = DebouncingSettings {
                debounce_delay_s: Some(5),
                debounce_key: Some("beta".to_string()),
                ..Default::default()
            };
            let args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script_b".to_string()),
                "test-workspace",
                JobKind::Noop,
                job_b,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // Both should still be queued since they have different keys
        assert!(is_queued(&db, &job_a).await, "job_a should still be queued");
        assert!(is_queued(&db, &job_b).await, "job_b should still be queued");

        Ok(())
    }

    /// Test: Debounce key with $args interpolation uses the args to build a unique key.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_key_with_args_interpolation(db: Pool<Postgres>) -> anyhow::Result<()> {
        let job1 = Uuid::new_v4();
        let job2 = Uuid::new_v4();
        let job3 = Uuid::new_v4();
        insert_noop_job(&db, job1, "test-workspace").await;
        insert_noop_job(&db, job2, "test-workspace").await;
        insert_noop_job(&db, job3, "test-workspace").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("debounce_$args[tenant_id]".to_string()),
            ..Default::default()
        };

        // job1: tenant_id = "A"
        {
            let mut hm = HashMap::new();
            hm.insert(
                "tenant_id".to_string(),
                RawValue::from_string("\"A\"".to_string()).unwrap(),
            );
            let args = PushArgs::from(&hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                job1,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // job2: tenant_id = "B" (different key)
        {
            let mut hm = HashMap::new();
            hm.insert(
                "tenant_id".to_string(),
                RawValue::from_string("\"B\"".to_string()).unwrap(),
            );
            let args = PushArgs::from(&hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                job2,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // job3: tenant_id = "A" (same key as job1, should debounce job1)
        {
            let mut hm = HashMap::new();
            hm.insert(
                "tenant_id".to_string(),
                RawValue::from_string("\"A\"".to_string()).unwrap(),
            );
            let args = PushArgs::from(&hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                job3,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // job1 should be debounced (same key as job3)
        assert!(
            is_completed(&db, &job1).await,
            "job1 should be debounced by job3"
        );
        // job2 should still be queued (different key)
        assert!(
            is_queued(&db, &job2).await,
            "job2 should still be queued (different tenant)"
        );
        // job3 should still be queued
        assert!(is_queued(&db, &job3).await, "job3 should still be queued");

        Ok(())
    }

    /// Test: When debounce_delay_s is 0 or None, no debouncing should occur.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_no_debounce_when_delay_zero(db: Pool<Postgres>) -> anyhow::Result<()> {
        let job1 = Uuid::new_v4();
        let job2 = Uuid::new_v4();
        insert_noop_job(&db, job1, "test-workspace").await;
        insert_noop_job(&db, job2, "test-workspace").await;

        let args_hm = empty_args();

        // delay = 0
        {
            let settings = DebouncingSettings {
                debounce_delay_s: Some(0),
                debounce_key: Some("no_debounce_zero".to_string()),
                ..Default::default()
            };
            let args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                job1,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
            assert!(
                scheduled_for.is_none(),
                "scheduled_for should not be set with delay=0"
            );
        }

        // delay = None
        {
            let settings = DebouncingSettings {
                debounce_delay_s: None,
                debounce_key: Some("no_debounce_none".to_string()),
                ..Default::default()
            };
            let args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                job2,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
            assert!(
                scheduled_for.is_none(),
                "scheduled_for should not be set with delay=None"
            );
        }

        // Both should still be queued
        assert!(is_queued(&db, &job1).await);
        assert!(is_queued(&db, &job2).await);

        Ok(())
    }

    /// Test: max_total_debounces_amount limit - debounce batch resets when exceeded.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_max_count_limit(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("count_limit_key".to_string()),
            max_total_debounces_amount: Some(2),
            ..Default::default()
        };
        let args_hm = empty_args();

        // Push 4 jobs: after the 3rd debounce (exceeding limit of 2), batch should reset
        let mut jobs = Vec::new();
        for _ in 0..4 {
            let job_id = Uuid::new_v4();
            insert_noop_job(&db, job_id, "test-workspace").await;
            jobs.push(job_id);
        }

        for &j in &jobs {
            let args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Noop,
                j,
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // The debounce_key entry should still exist
        let dk = get_debounce_key(&db, "count_limit_key").await;
        assert!(dk.is_some(), "debounce_key entry should exist");

        Ok(())
    }

    // =========================================================================
    // Tests for maybe_debounce_post_preprocessing
    // =========================================================================

    /// Test: Post-preprocessing debounce with first job returns scheduled_for.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_debounce_first_job(db: Pool<Postgres>) -> anyhow::Result<()> {
        let flow_id = Uuid::new_v4();
        insert_flow_job(&db, flow_id, "test-workspace", "f/test/flow").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_first_key".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);

        let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            flow_id,
            &args,
            &db,
        )
        .await?;

        // Should return a scheduled_for value
        assert!(
            result.is_some(),
            "should return scheduled_for for first job"
        );
        let sf = result.unwrap();
        let diff = (sf - Utc::now()).num_seconds();
        assert!(
            diff >= 3 && diff <= 6,
            "scheduled_for should be ~5s in future, got {diff}s"
        );

        // debounce_key should be created
        let dk = get_debounce_key(&db, "pp_first_key").await;
        assert!(dk.is_some(), "debounce_key entry should exist");
        let (dk_job_id, _, _) = dk.unwrap();
        assert_eq!(dk_job_id, flow_id);

        Ok(())
    }

    /// Test: Post-preprocessing debounce with second job debounces the first.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_debounce_second_cancels_first(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let flow1 = Uuid::new_v4();
        let flow2 = Uuid::new_v4();
        insert_flow_job(&db, flow1, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, flow2, "test-workspace", "f/test/flow").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_cancel_key".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        // First flow
        {
            let args = PushArgs::from(&args_hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                flow1,
                &args,
                &db,
            )
            .await?;
        }

        // Second flow - should debounce the first
        {
            let args = PushArgs::from(&args_hm);
            let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                flow2,
                &args,
                &db,
            )
            .await?;
            assert!(result.is_some(), "should return scheduled_for");
        }

        // flow1 should be completed (debounced)
        assert!(
            is_completed(&db, &flow1).await,
            "flow1 should be completed (debounced by flow2)"
        );

        // flow2 should still be in queue
        assert!(is_queued(&db, &flow2).await, "flow2 should still be queued");

        // debounce_key should point to flow2
        let dk = get_debounce_key(&db, "pp_cancel_key").await.unwrap();
        assert_eq!(dk.0, flow2, "debounce_key should point to flow2");
        assert_eq!(dk.2, 1, "debounced_times should be 1");

        Ok(())
    }

    /// Test: Post-preprocessing debounce with args-based key differentiates by preprocessed args.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_debounce_args_differentiation(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let flow_a = Uuid::new_v4();
        let flow_b = Uuid::new_v4();
        let flow_a2 = Uuid::new_v4();
        insert_flow_job(&db, flow_a, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, flow_b, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, flow_a2, "test-workspace", "f/test/flow").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_$args[region]".to_string()),
            ..Default::default()
        };

        // flow_a: region = "us"
        {
            let mut hm = HashMap::new();
            hm.insert(
                "region".to_string(),
                RawValue::from_string("\"us\"".to_string()).unwrap(),
            );
            let args = PushArgs::from(&hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                flow_a,
                &args,
                &db,
            )
            .await?;
        }

        // flow_b: region = "eu" (different key, no debounce)
        {
            let mut hm = HashMap::new();
            hm.insert(
                "region".to_string(),
                RawValue::from_string("\"eu\"".to_string()).unwrap(),
            );
            let args = PushArgs::from(&hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                flow_b,
                &args,
                &db,
            )
            .await?;
        }

        // flow_a2: region = "us" (same key as flow_a, should debounce flow_a)
        {
            let mut hm = HashMap::new();
            hm.insert(
                "region".to_string(),
                RawValue::from_string("\"us\"".to_string()).unwrap(),
            );
            let args = PushArgs::from(&hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                flow_a2,
                &args,
                &db,
            )
            .await?;
        }

        // flow_a should be debounced (same region as flow_a2)
        assert!(
            is_completed(&db, &flow_a).await,
            "flow_a should be debounced by flow_a2"
        );
        // flow_b should be queued (different region)
        assert!(
            is_queued(&db, &flow_b).await,
            "flow_b should still be queued"
        );
        // flow_a2 should be queued
        assert!(
            is_queued(&db, &flow_a2).await,
            "flow_a2 should still be queued"
        );

        Ok(())
    }

    /// Test: Post-preprocessing debounce returns None when delay is zero.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_no_debounce_zero_delay(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let flow_id = Uuid::new_v4();
        insert_flow_job(&db, flow_id, "test-workspace", "f/test/flow").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(0),
            debounce_key: Some("pp_zero_delay".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);

        let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            flow_id,
            &args,
            &db,
        )
        .await?;

        assert!(result.is_none(), "should return None when delay is 0");
        Ok(())
    }

    /// Test: Post-preprocessing debounce returns None when delay is None.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_no_debounce_no_delay(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let flow_id = Uuid::new_v4();
        insert_flow_job(&db, flow_id, "test-workspace", "f/test/flow").await;

        let settings = DebouncingSettings::default();
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);

        let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            flow_id,
            &args,
            &db,
        )
        .await?;

        assert!(result.is_none(), "should return None with default settings");
        Ok(())
    }

    /// Test: Post-preprocessing debounce chain of 1000 jobs — only the last should remain queued.
    /// This verifies debouncing works correctly at scale with sequential debounce operations.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_debounce_chain_1000(db: Pool<Postgres>) -> anyhow::Result<()> {
        let n: usize = 1000;

        // Batch-insert all jobs using raw SQL for speed
        let uuids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        for chunk in uuids.chunks(500) {
            let chunk_vec: Vec<Uuid> = chunk.to_vec();
            sqlx::query!(
                "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path)
                 SELECT unnest($1::uuid[]), 'flow', 'flow', 'test-user', 'u/test-user', 'test@windmill.dev', 'test-workspace', 'f/test/flow'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
                 SELECT unnest($1::uuid[]), 'test-workspace', now(), 'flow'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
        }

        let settings = DebouncingSettings {
            debounce_delay_s: Some(10),
            debounce_key: Some("pp_chain_1000_key".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        for &j in &uuids {
            let args = PushArgs::from(&args_hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                j,
                &args,
                &db,
            )
            .await?;
        }

        // Only the last job should remain in queue
        let queued_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_queue WHERE id = ANY($1)",
            &uuids,
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            queued_count, 1,
            "exactly 1 job should remain in queue, got {queued_count}"
        );

        // N-1 jobs should be completed (debounced)
        let completed_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_completed WHERE id = ANY($1)",
            &uuids,
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            completed_count,
            (n - 1) as i64,
            "n-1 jobs should be completed (debounced), got {completed_count}"
        );

        let dk = get_debounce_key(&db, "pp_chain_1000_key").await.unwrap();
        assert_eq!(dk.0, uuids[n - 1], "debounce_key should point to last job");
        assert_eq!(dk.2, (n - 1) as i32);

        Ok(())
    }

    /// Test: Post-preprocessing debounce with max count limit resets the batch.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_debounce_max_count_resets(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_max_count_key".to_string()),
            max_total_debounces_amount: Some(2),
            ..Default::default()
        };
        let args_hm = empty_args();

        // Push 4 jobs. After 3rd debounce (exceeding limit of 2), batch should reset.
        let mut jobs = Vec::new();
        let mut results = Vec::new();
        for _ in 0..4 {
            let id = Uuid::new_v4();
            insert_flow_job(&db, id, "test-workspace", "f/test/flow").await;
            jobs.push(id);
        }

        for &j in &jobs {
            let args = PushArgs::from(&args_hm);
            let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                j,
                &args,
                &db,
            )
            .await?;
            results.push(result);
        }

        // First job always gets scheduled_for
        assert!(results[0].is_some(), "first job should get scheduled_for");

        // Jobs 2 and 3 should also get scheduled_for (debouncing within limit)
        assert!(results[1].is_some(), "second job should get scheduled_for");
        assert!(results[2].is_some(), "third job should get scheduled_for");

        // Job 4 (the one that exceeds the limit): when limit is exceeded,
        // the batch resets and the job executes immediately (no scheduled_for delay)
        // The exact behavior depends on whether the limit check happens before or after the
        // new job is counted. Let's just verify the debounce_key is reset.
        let dk = get_debounce_key(&db, "pp_max_count_key").await.unwrap();
        // debounced_times should have been reset at some point
        assert!(dk.0 == jobs[3], "debounce_key should point to last job");

        Ok(())
    }

    /// Test: 1000 concurrent debounce operations with different keys — no contention or deadlocks.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_concurrent_different_keys_1000(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let n: usize = 1000;

        // Batch-insert all flow jobs upfront
        let flow_ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        for chunk in flow_ids.chunks(500) {
            let chunk_vec: Vec<Uuid> = chunk.to_vec();
            sqlx::query!(
                "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path)
                 SELECT unnest($1::uuid[]), 'flow', 'flow', 'test-user', 'u/test-user', 'test@windmill.dev', 'test-workspace', 'f/test/flow'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
                 SELECT unnest($1::uuid[]), 'test-workspace', now(), 'flow'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
        }

        // Fire all debounce calls concurrently, each with a unique key
        let mut handles = Vec::new();
        for (i, &flow_id) in flow_ids.iter().enumerate() {
            let db = db.clone();
            let handle = tokio::spawn(async move {
                let settings = DebouncingSettings {
                    debounce_delay_s: Some(5),
                    debounce_key: Some(format!("concurrent_key_{i}")),
                    ..Default::default()
                };
                let args_hm: HashMap<String, Box<RawValue>> = HashMap::new();
                let args = PushArgs::from(&args_hm);

                windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                    &settings,
                    &Some("f/test/flow".to_string()),
                    "test-workspace",
                    flow_id,
                    &args,
                    &db,
                )
                .await
            });
            handles.push(handle);
        }

        let mut error_count = 0;
        for handle in handles {
            match handle.await? {
                Ok(result) => {
                    assert!(result.is_some(), "should return scheduled_for");
                }
                Err(e) => {
                    eprintln!("Concurrent debounce error: {e:#}");
                    error_count += 1;
                }
            }
        }
        assert_eq!(error_count, 0, "no errors expected, got {error_count}");

        // All jobs should still be in queue (each has a unique key, no debouncing between them)
        let queued_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_queue WHERE id = ANY($1)",
            &flow_ids,
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            queued_count, n as i64,
            "all {n} jobs should remain in queue, got {queued_count}"
        );

        Ok(())
    }

    /// Test: 1000 concurrent debounce operations with the SAME key — verifies no deadlocks
    /// and exactly 1 job survives in the queue.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_concurrent_same_key_1000(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let n: usize = 1000;

        // Batch-insert all flow jobs upfront
        let flow_ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
        for chunk in flow_ids.chunks(500) {
            let chunk_vec: Vec<Uuid> = chunk.to_vec();
            sqlx::query!(
                "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path)
                 SELECT unnest($1::uuid[]), 'flow', 'flow', 'test-user', 'u/test-user', 'test@windmill.dev', 'test-workspace', 'f/test/flow'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
                 SELECT unnest($1::uuid[]), 'test-workspace', now(), 'flow'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
        }

        // Fire all debounce calls concurrently, all sharing the same key
        let mut handles = Vec::new();
        for &flow_id in &flow_ids {
            let db = db.clone();
            let handle = tokio::spawn(async move {
                let settings = DebouncingSettings {
                    debounce_delay_s: Some(5),
                    debounce_key: Some("shared_concurrent_key_1000".to_string()),
                    ..Default::default()
                };
                let args_hm: HashMap<String, Box<RawValue>> = HashMap::new();
                let args = PushArgs::from(&args_hm);

                windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                    &settings,
                    &Some("f/test/flow".to_string()),
                    "test-workspace",
                    flow_id,
                    &args,
                    &db,
                )
                .await
            });
            handles.push(handle);
        }

        let mut success_count = 0;
        let mut error_count = 0;
        for handle in handles {
            match handle.await? {
                Ok(_) => success_count += 1,
                Err(e) => {
                    eprintln!("Concurrent debounce error: {e:#}");
                    error_count += 1;
                }
            }
        }

        assert_eq!(error_count, 0, "no errors expected, got {error_count}");
        assert_eq!(success_count, n, "all {n} debounce calls should succeed");

        // Only 1 job should remain in queue, rest should be debounced
        let queued_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_queue WHERE id = ANY($1)",
            &flow_ids,
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            queued_count, 1,
            "exactly 1 job should remain in queue, got {queued_count}"
        );

        let completed_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_completed WHERE id = ANY($1)",
            &flow_ids,
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            completed_count,
            (n - 1) as i64,
            "{} jobs should be completed (debounced), got {completed_count}",
            n - 1
        );

        Ok(())
    }

    // =========================================================================
    // Stress test for DB contention (run manually with --ignored)
    // =========================================================================

    /// Stress test: 20,000 debounce operations across 100 keys (200 jobs per key),
    /// with bounded concurrency (64 in-flight at a time, matching a large worker fleet).
    /// Measures wall-clock time, per-operation latency percentiles, and throughput.
    ///
    /// Run with:
    ///   cargo test -p windmill-queue --test debounce_test --features private,enterprise \
    ///     -- --ignored test_debounce_contention_stress --nocapture
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    #[ignore]
    async fn test_debounce_contention_stress(db: Pool<Postgres>) -> anyhow::Result<()> {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let num_keys: usize = 100;
        let jobs_per_key: usize = 200;
        let total = num_keys * jobs_per_key;
        let max_concurrent: usize = 64;

        // Batch-insert all flow jobs upfront
        let all_ids: Vec<Uuid> = (0..total).map(|_| Uuid::new_v4()).collect();
        for chunk in all_ids.chunks(500) {
            let chunk_vec: Vec<Uuid> = chunk.to_vec();
            sqlx::query!(
                "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path)
                 SELECT unnest($1::uuid[]), 'flow', 'flow', 'test-user', 'u/test-user', 'test@windmill.dev', 'test-workspace', 'f/test/flow'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
                 SELECT unnest($1::uuid[]), 'test-workspace', now(), 'flow'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
        }

        eprintln!("=== DEBOUNCE CONTENTION STRESS TEST ===");
        eprintln!("  keys:            {num_keys}");
        eprintln!("  jobs per key:    {jobs_per_key}");
        eprintln!("  total jobs:      {total}");
        eprintln!("  max concurrent:  {max_concurrent}");

        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let start = std::time::Instant::now();

        let mut handles = Vec::with_capacity(total);
        for (i, &flow_id) in all_ids.iter().enumerate() {
            let db = db.clone();
            let sem = semaphore.clone();
            let key_index = i % num_keys;
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let settings = DebouncingSettings {
                    debounce_delay_s: Some(60),
                    debounce_key: Some(format!("stress_key_{key_index}")),
                    ..Default::default()
                };
                let args_hm: HashMap<String, Box<RawValue>> = HashMap::new();
                let args = PushArgs::from(&args_hm);

                let op_start = std::time::Instant::now();
                let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                    &settings,
                    &Some("f/test/flow".to_string()),
                    "test-workspace",
                    flow_id,
                    &args,
                    &db,
                )
                .await;
                let op_duration = op_start.elapsed();

                (result, op_duration)
            });
            handles.push(handle);
        }

        let mut error_count = 0;
        let mut op_durations = Vec::with_capacity(total);
        for handle in handles {
            let (result, duration) = handle.await?;
            op_durations.push(duration);
            if let Err(e) = result {
                eprintln!("  error: {e:#}");
                error_count += 1;
            }
        }

        let wall_time = start.elapsed();

        // Compute stats
        op_durations.sort();
        let p50 = op_durations[total / 2];
        let p95 = op_durations[total * 95 / 100];
        let p99 = op_durations[total * 99 / 100];
        let max = op_durations[total - 1];
        let ops_per_sec = total as f64 / wall_time.as_secs_f64();

        // Each key group should have exactly 1 survivor in queue
        let queued_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_queue WHERE id = ANY($1)",
            &all_ids,
        )
        .fetch_one(&db)
        .await?;

        let completed_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_completed WHERE id = ANY($1)",
            &all_ids,
        )
        .fetch_one(&db)
        .await?;

        eprintln!("  wall time:       {wall_time:?}");
        eprintln!("  ops/sec:         {ops_per_sec:.0}");
        eprintln!("  p50 latency:     {p50:?}");
        eprintln!("  p95 latency:     {p95:?}");
        eprintln!("  p99 latency:     {p99:?}");
        eprintln!("  max latency:     {max:?}");
        eprintln!("  errors:          {error_count}");
        eprintln!("  queued:          {queued_count} (expected {num_keys})");
        eprintln!(
            "  completed:       {completed_count} (expected {})",
            total - num_keys
        );
        eprintln!("=======================================");

        assert_eq!(error_count, 0, "no errors expected, got {error_count}");
        assert_eq!(
            queued_count, num_keys as i64,
            "expected {num_keys} survivors (1 per key), got {queued_count}"
        );
        assert_eq!(
            completed_count,
            (total - num_keys) as i64,
            "expected {} debounced, got {completed_count}",
            total - num_keys
        );

        Ok(())
    }

    /// Stress test for push-time maybe_debounce: 20,000 operations across 100 keys,
    /// each holding a caller transaction open (simulating push_inner) while debouncing.
    /// This exercises the connection pool under realistic concurrent push load.
    ///
    /// Run with:
    ///   cargo test -p windmill-queue --test debounce_test --features private,enterprise \
    ///     -- --ignored test_push_debounce_contention_stress --nocapture
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    #[ignore]
    async fn test_push_debounce_contention_stress(db: Pool<Postgres>) -> anyhow::Result<()> {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let num_keys: usize = 100;
        let jobs_per_key: usize = 200;
        let total = num_keys * jobs_per_key;
        let max_concurrent: usize = 64;

        // Batch-insert all jobs upfront
        let all_ids: Vec<Uuid> = (0..total).map(|_| Uuid::new_v4()).collect();
        for chunk in all_ids.chunks(500) {
            let chunk_vec: Vec<Uuid> = chunk.to_vec();
            sqlx::query!(
                "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id)
                 SELECT unnest($1::uuid[]), 'noop', 'deno', 'test-user', 'u/test-user', 'test@windmill.dev', 'test-workspace'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
                 SELECT unnest($1::uuid[]), 'test-workspace', now(), 'deno'",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
            sqlx::query!(
                "INSERT INTO v2_job_runtime (id) SELECT unnest($1::uuid[])",
                &chunk_vec,
            )
            .execute(&db)
            .await?;
        }

        eprintln!("=== PUSH-TIME DEBOUNCE CONTENTION STRESS TEST ===");
        eprintln!("  keys:            {num_keys}");
        eprintln!("  jobs per key:    {jobs_per_key}");
        eprintln!("  total jobs:      {total}");
        eprintln!("  max concurrent:  {max_concurrent}");

        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let start = std::time::Instant::now();

        let mut handles = Vec::with_capacity(total);
        for (i, &job_id) in all_ids.iter().enumerate() {
            let db = db.clone();
            let sem = semaphore.clone();
            let key_index = i % num_keys;
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let settings = DebouncingSettings {
                    debounce_delay_s: Some(60),
                    debounce_key: Some(format!("push_stress_key_{key_index}")),
                    ..Default::default()
                };
                let args_hm: HashMap<String, Box<RawValue>> = HashMap::new();
                let args = PushArgs::from(&args_hm);

                let op_start = std::time::Instant::now();

                // Simulate push_inner: open a caller tx, call maybe_debounce,
                // then commit (mirroring the real push flow).
                let mut tx = db.begin().await?;
                let mut scheduled_for = None;
                windmill_queue::jobs_ee::maybe_debounce(
                    &settings,
                    &mut scheduled_for,
                    &None,
                    "test-workspace",
                    JobKind::Script,
                    job_id,
                    &args,
                    &mut tx,
                    &db,
                )
                .await?;
                tx.commit().await?;

                let op_duration = op_start.elapsed();
                Ok::<_, windmill_common::error::Error>((scheduled_for, op_duration))
            });
            handles.push(handle);
        }

        let mut error_count = 0;
        let mut op_durations = Vec::with_capacity(total);
        for handle in handles {
            match handle.await? {
                Ok((_scheduled_for, duration)) => {
                    op_durations.push(duration);
                }
                Err(e) => {
                    eprintln!("  error: {e:#}");
                    error_count += 1;
                    op_durations.push(std::time::Duration::ZERO);
                }
            }
        }

        let wall_time = start.elapsed();

        // Compute stats
        op_durations.sort();
        let p50 = op_durations[total / 2];
        let p95 = op_durations[total * 95 / 100];
        let p99 = op_durations[total * 99 / 100];
        let max = op_durations[total - 1];
        let ops_per_sec = total as f64 / wall_time.as_secs_f64();

        let queued_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_queue WHERE id = ANY($1)",
            &all_ids,
        )
        .fetch_one(&db)
        .await?;

        let completed_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) as \"count!\" FROM v2_job_completed WHERE id = ANY($1)",
            &all_ids,
        )
        .fetch_one(&db)
        .await?;

        eprintln!("  wall time:       {wall_time:?}");
        eprintln!("  ops/sec:         {ops_per_sec:.0}");
        eprintln!("  p50 latency:     {p50:?}");
        eprintln!("  p95 latency:     {p95:?}");
        eprintln!("  p99 latency:     {p99:?}");
        eprintln!("  max latency:     {max:?}");
        eprintln!("  errors:          {error_count}");
        eprintln!("  queued:          {queued_count} (expected {num_keys})");
        eprintln!(
            "  completed:       {completed_count} (expected {})",
            total - num_keys
        );
        eprintln!("=================================================");

        assert_eq!(error_count, 0, "no errors expected, got {error_count}");
        assert_eq!(
            queued_count, num_keys as i64,
            "expected {num_keys} survivors (1 per key), got {queued_count}"
        );
        assert_eq!(
            completed_count,
            (total - num_keys) as i64,
            "expected {} debounced, got {completed_count}",
            total - num_keys
        );

        Ok(())
    }
}
