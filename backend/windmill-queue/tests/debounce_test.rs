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
    // Edge case tests: timing, limits, batch behavior, scheduled_for
    // =========================================================================

    /// Test: scheduled_for is set to approximately now + delay_seconds.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_push_scheduled_for_value(db: Pool<Postgres>) -> anyhow::Result<()> {
        let job_id = Uuid::new_v4();
        insert_noop_job(&db, job_id, "test-workspace").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(30),
            debounce_key: Some("scheduled_for_test".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);

        let before = Utc::now();
        let mut scheduled_for = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut scheduled_for,
            &None,
            "test-workspace",
            JobKind::Noop,
            job_id,
            &args,
            &mut tx,
        )
        .await?;
        tx.commit().await?;
        let after = Utc::now();

        let sf = scheduled_for.expect("scheduled_for should be set");
        let expected_min = before + chrono::Duration::seconds(30);
        let expected_max = after + chrono::Duration::seconds(30);
        assert!(
            sf >= expected_min && sf <= expected_max,
            "scheduled_for ({sf}) should be between {expected_min} and {expected_max}"
        );

        Ok(())
    }

    /// Test: post-preprocessing scheduled_for is set to approximately now + delay_seconds.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_scheduled_for_value(db: Pool<Postgres>) -> anyhow::Result<()> {
        let flow_id = Uuid::new_v4();
        insert_flow_job(&db, flow_id, "test-workspace", "f/test/flow").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(45),
            debounce_key: Some("pp_scheduled_for_test".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);

        let before = Utc::now();
        let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            flow_id,
            &args,
            &db,
        )
        .await?;
        let after = Utc::now();

        let sf = result.expect("should return scheduled_for");
        let expected_min = before + chrono::Duration::seconds(45);
        let expected_max = after + chrono::Duration::seconds(45);
        assert!(
            sf >= expected_min && sf <= expected_max,
            "scheduled_for ({sf}) should be between {expected_min} and {expected_max}"
        );

        Ok(())
    }

    /// Test: push-time does NOT set scheduled_for if one is already provided (uses .or()).
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_push_preserves_existing_scheduled_for(db: Pool<Postgres>) -> anyhow::Result<()> {
        let job_id = Uuid::new_v4();
        insert_noop_job(&db, job_id, "test-workspace").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(30),
            debounce_key: Some("preserve_sf_test".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);

        let preset = Utc::now() + chrono::Duration::seconds(999);
        let mut scheduled_for = Some(preset);
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut scheduled_for,
            &None,
            "test-workspace",
            JobKind::Noop,
            job_id,
            &args,
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        assert_eq!(
            scheduled_for,
            Some(preset),
            "existing scheduled_for should be preserved"
        );

        Ok(())
    }

    /// Test: max_total_debouncing_time causes batch reset when exceeded.
    /// Uses direct DB manipulation to set first_started_at in the past.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_max_time_exceeded(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_time_limit_key".to_string()),
            max_total_debouncing_time: Some(10), // 10 seconds max
            ..Default::default()
        };
        let args_hm = empty_args();

        // Job 1: first in batch
        let job1 = Uuid::new_v4();
        insert_flow_job(&db, job1, "test-workspace", "f/test/flow").await;
        let args = PushArgs::from(&args_hm);
        let r1 = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job1,
            &args,
            &db,
        )
        .await?;
        assert!(r1.is_some(), "first job should get scheduled_for");

        // Force first_started_at to 20 seconds ago to simulate time exceeding the limit
        sqlx::query!(
            "UPDATE debounce_key SET first_started_at = now() - interval '20 seconds' WHERE key = $1",
            "pp_time_limit_key"
        )
        .execute(&db)
        .await?;

        // Job 2: should trigger time limit exceeded → batch reset, no debouncing
        let job2 = Uuid::new_v4();
        insert_flow_job(&db, job2, "test-workspace", "f/test/flow").await;
        let args = PushArgs::from(&args_hm);
        let r2 = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job2,
            &args,
            &db,
        )
        .await?;
        // When limit is exceeded, the function resets and returns None (execute immediately)
        assert!(
            r2.is_none(),
            "should return None when time limit is exceeded"
        );

        // Verify the batch was reset: debounced_times should be 0
        let dk = get_debounce_key(&db, "pp_time_limit_key").await.unwrap();
        assert_eq!(dk.2, 0, "debounced_times should be reset to 0");

        // Job 1 should NOT be completed (time limit reset skips debouncing the previous job)
        assert!(
            is_queued(&db, &job1).await,
            "job1 should still be queued (time limit reset doesn't debounce)"
        );

        Ok(())
    }

    /// Test: push-time max_total_debouncing_time causes batch reset when exceeded.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_push_max_time_exceeded(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("push_time_limit_key".to_string()),
            max_total_debouncing_time: Some(10),
            ..Default::default()
        };
        let args_hm = empty_args();

        // Job 1: first in batch
        let job1 = Uuid::new_v4();
        insert_noop_job(&db, job1, "test-workspace").await;
        let args = PushArgs::from(&args_hm);
        let mut sf = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut sf,
            &None,
            "test-workspace",
            JobKind::Noop,
            job1,
            &args,
            &mut tx,
        )
        .await?;
        tx.commit().await?;
        assert!(sf.is_some(), "first job should get scheduled_for");

        // Force first_started_at to 20 seconds ago
        sqlx::query!(
            "UPDATE debounce_key SET first_started_at = now() - interval '20 seconds' WHERE key = $1",
            "push_time_limit_key"
        )
        .execute(&db)
        .await?;

        // Job 2: should trigger time limit exceeded
        let job2 = Uuid::new_v4();
        insert_noop_job(&db, job2, "test-workspace").await;
        let args = PushArgs::from(&args_hm);
        let mut sf2 = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut sf2,
            &None,
            "test-workspace",
            JobKind::Noop,
            job2,
            &args,
            &mut tx,
        )
        .await?;
        tx.commit().await?;
        // scheduled_for is still set (push-time doesn't clear it on limit exceed)
        // but the batch should be reset
        let dk = get_debounce_key(&db, "push_time_limit_key").await.unwrap();
        assert_eq!(dk.2, 0, "debounced_times should be reset to 0");

        Ok(())
    }

    /// Test: max_count boundary — at exactly the limit, debouncing still works.
    /// One over the limit triggers reset.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_max_count_exact_boundary(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_count_boundary_key".to_string()),
            max_total_debounces_amount: Some(3),
            ..Default::default()
        };
        let args_hm = empty_args();

        let mut jobs = Vec::new();
        let mut results = Vec::new();
        // Push 5 jobs: job 1 (no debounce), jobs 2-4 (debounce, count 1-3), job 5 (count 4 > limit 3 → reset)
        for _ in 0..5 {
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

        // Jobs 1-4 should return Some (scheduled_for) — debouncing within limit
        for (i, r) in results.iter().enumerate().take(4) {
            assert!(
                r.is_some(),
                "job {} should get scheduled_for (within limit)",
                i + 1
            );
        }

        // Job 5 (debounced_times=4, exceeds limit=3) should return None (batch reset)
        assert!(
            results[4].is_none(),
            "job 5 should return None (limit exceeded, batch reset)"
        );

        // After reset, debounced_times should be 0
        let dk = get_debounce_key(&db, "pp_count_boundary_key")
            .await
            .unwrap();
        assert_eq!(dk.2, 0, "debounced_times should be reset to 0 after limit");

        Ok(())
    }

    /// Test: after a max_count reset, a new batch starts fresh and debouncing works again.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_max_count_reset_new_batch(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_reset_cycle_key".to_string()),
            max_total_debounces_amount: Some(2),
            ..Default::default()
        };
        let args_hm = empty_args();

        // Limit check is `debounced_times > max`, so with max=2 we need 4 jobs
        // to trigger reset (debounced_times=3 on the 4th job, 3>2=true).
        // Cycle 1: jobs 1-4 (job 4 exceeds limit → reset)
        let mut cycle1 = Vec::new();
        for _ in 0..4 {
            let id = Uuid::new_v4();
            insert_flow_job(&db, id, "test-workspace", "f/test/flow").await;
            cycle1.push(id);
        }
        let mut cycle1_results = Vec::new();
        for &j in &cycle1 {
            let args = PushArgs::from(&args_hm);
            let r = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                j,
                &args,
                &db,
            )
            .await?;
            cycle1_results.push(r);
        }
        assert!(cycle1_results[0].is_some(), "cycle1 job1 scheduled");
        assert!(cycle1_results[1].is_some(), "cycle1 job2 scheduled");
        assert!(
            cycle1_results[2].is_some(),
            "cycle1 job3 scheduled (at limit)"
        );
        assert!(
            cycle1_results[3].is_none(),
            "cycle1 job4 should reset (over limit)"
        );

        // Verify debounced_times is reset to 0
        let dk = get_debounce_key(&db, "pp_reset_cycle_key").await.unwrap();
        assert_eq!(dk.2, 0, "debounced_times should be 0 after reset");

        // Cycle 2: jobs 5-8 (new batch, should debounce independently)
        let mut cycle2 = Vec::new();
        for _ in 0..4 {
            let id = Uuid::new_v4();
            insert_flow_job(&db, id, "test-workspace", "f/test/flow").await;
            cycle2.push(id);
        }
        let mut cycle2_results = Vec::new();
        for &j in &cycle2 {
            let args = PushArgs::from(&args_hm);
            let r = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                j,
                &args,
                &db,
            )
            .await?;
            cycle2_results.push(r);
        }
        // After cycle 1 reset, debounced_times=0. Cycle 2's first job hits ON CONFLICT
        // and increments to 1 (unlike cycle 1's first job which was a fresh insert at 0).
        // So cycle 2 reaches the limit one job sooner:
        //   job5: dt=1, job6: dt=2, job7: dt=3 (>2 → reset), job8: dt=1
        assert!(cycle2_results[0].is_some(), "cycle2 job1 scheduled (dt=1)");
        assert!(cycle2_results[1].is_some(), "cycle2 job2 scheduled (dt=2)");
        assert!(
            cycle2_results[2].is_none(),
            "cycle2 job3 should reset (dt=3 > 2)"
        );
        assert!(
            cycle2_results[3].is_some(),
            "cycle2 job4 scheduled (fresh after reset, dt=1)"
        );

        Ok(())
    }

    /// Test: combined max_count AND max_time — whichever triggers first resets the batch.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_combined_count_and_time_limits(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_combined_limits_key".to_string()),
            max_total_debounces_amount: Some(100), // high count limit
            max_total_debouncing_time: Some(10),   // low time limit
            ..Default::default()
        };
        let args_hm = empty_args();

        // Job 1: start batch
        let job1 = Uuid::new_v4();
        insert_flow_job(&db, job1, "test-workspace", "f/test/flow").await;
        let args = PushArgs::from(&args_hm);
        let r1 = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job1,
            &args,
            &db,
        )
        .await?;
        assert!(r1.is_some(), "first job should get scheduled_for");

        // Force time to exceed limit (count is still 1, well under 100)
        sqlx::query!(
            "UPDATE debounce_key SET first_started_at = now() - interval '20 seconds' WHERE key = $1",
            "pp_combined_limits_key"
        )
        .execute(&db)
        .await?;

        // Job 2: time limit should trigger even though count is low
        let job2 = Uuid::new_v4();
        insert_flow_job(&db, job2, "test-workspace", "f/test/flow").await;
        let args = PushArgs::from(&args_hm);
        let r2 = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job2,
            &args,
            &db,
        )
        .await?;
        assert!(
            r2.is_none(),
            "time limit should trigger reset even with low count"
        );

        Ok(())
    }

    /// Test: debounce batch IDs are consistent within a batch.
    /// All jobs in the same debounce batch should share the same batch number.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_batch_id_consistency(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_batch_id_test".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        let mut jobs = Vec::new();
        for _ in 0..5 {
            let id = Uuid::new_v4();
            insert_flow_job(&db, id, "test-workspace", "f/test/flow").await;
            jobs.push(id);
        }

        for &j in &jobs {
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

        // All jobs should have the same debounce_batch
        let batches: Vec<i64> = sqlx::query_scalar!(
            "SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = ANY($1) ORDER BY debounce_batch",
            &jobs,
        )
        .fetch_all(&db)
        .await?;

        assert_eq!(batches.len(), 5, "all 5 jobs should have batch entries");
        let first = batches[0];
        assert!(
            batches.iter().all(|b| *b == first),
            "all jobs in same debounce batch should have the same batch ID, got {:?}",
            batches
        );

        Ok(())
    }

    /// Test: after a max_count reset, the new batch gets a different batch ID.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_batch_id_changes_on_reset(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_batch_reset_id_test".to_string()),
            max_total_debounces_amount: Some(2),
            ..Default::default()
        };
        let args_hm = empty_args();

        // Batch 1: jobs 1-4 (job 4 triggers reset at debounced_times=3 > 2)
        let mut batch1_jobs = Vec::new();
        for _ in 0..4 {
            let id = Uuid::new_v4();
            insert_flow_job(&db, id, "test-workspace", "f/test/flow").await;
            batch1_jobs.push(id);
        }
        for &j in &batch1_jobs {
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

        // Batch 2: jobs 5-6 (new batch after reset)
        let mut batch2_jobs = Vec::new();
        for _ in 0..2 {
            let id = Uuid::new_v4();
            insert_flow_job(&db, id, "test-workspace", "f/test/flow").await;
            batch2_jobs.push(id);
        }
        for &j in &batch2_jobs {
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

        let batch1_id: i64 = sqlx::query_scalar!(
            "SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = $1",
            batch1_jobs[0],
        )
        .fetch_one(&db)
        .await?;

        // Job 4 (the one that triggered reset) should have a different batch from jobs 1-3
        let reset_batch: i64 = sqlx::query_scalar!(
            "SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = $1",
            batch1_jobs[3],
        )
        .fetch_one(&db)
        .await?;

        assert_ne!(
            batch1_id, reset_batch,
            "reset job should have a different batch ID"
        );

        // Batch 2 jobs should share the same batch but different from batch 1
        let batch2_id: i64 = sqlx::query_scalar!(
            "SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = $1",
            batch2_jobs[0],
        )
        .fetch_one(&db)
        .await?;

        assert_ne!(
            batch1_id, batch2_id,
            "batch 2 should have a different batch ID from batch 1"
        );

        Ok(())
    }

    /// Test: different workspaces with the same debounce_key template produce different
    /// resolved keys and do not interfere with each other.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_workspace_isolation(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Create a second workspace with required related rows
        sqlx::query!(
            "INSERT INTO workspace (id, name, owner) VALUES ('ws2', 'Workspace 2', 'test-user')"
        )
        .execute(&db)
        .await?;
        sqlx::query!("INSERT INTO workspace_settings (workspace_id) VALUES ('ws2')")
            .execute(&db)
            .await?;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None, // default key includes workspace_id
            ..Default::default()
        };
        let args_hm = empty_args();

        // Job in workspace 1
        let job_ws1_a = Uuid::new_v4();
        let job_ws1_b = Uuid::new_v4();
        insert_flow_job(&db, job_ws1_a, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, job_ws1_b, "test-workspace", "f/test/flow").await;

        // Job in workspace 2
        let job_ws2 = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path)
             VALUES ($1, 'flow', 'flow', 'test-user', 'u/test-user', 'test@windmill.dev', 'ws2', 'f/test/flow')",
            job_ws2,
        )
        .execute(&db)
        .await?;
        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag) VALUES ($1, 'ws2', now(), 'flow')",
            job_ws2,
        )
        .execute(&db)
        .await?;
        sqlx::query!("INSERT INTO v2_job_runtime (id) VALUES ($1)", job_ws2)
            .execute(&db)
            .await?;

        // Debounce ws1 job A
        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job_ws1_a,
            &args,
            &db,
        )
        .await?;

        // Debounce ws2 job — should NOT debounce ws1 job A
        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "ws2",
            job_ws2,
            &args,
            &db,
        )
        .await?;

        // ws1 job A should still be queued (not debounced by ws2)
        assert!(
            is_queued(&db, &job_ws1_a).await,
            "ws1 job A should still be queued"
        );

        // Now debounce ws1 job B — should debounce ws1 job A
        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job_ws1_b,
            &args,
            &db,
        )
        .await?;

        // ws1 job A should now be completed (debounced by ws1 job B)
        assert!(
            is_completed(&db, &job_ws1_a).await,
            "ws1 job A should be debounced by ws1 job B"
        );
        // ws2 job should still be queued
        assert!(
            is_queued(&db, &job_ws2).await,
            "ws2 job should still be queued"
        );
        // ws1 job B should still be queued
        assert!(
            is_queued(&db, &job_ws1_b).await,
            "ws1 job B should still be queued"
        );

        Ok(())
    }

    /// Test: debounced job's completed result contains the expected "Debounced by" message.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_completed_result_format(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_result_format_key".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        let job1 = Uuid::new_v4();
        let job2 = Uuid::new_v4();
        insert_flow_job(&db, job1, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, job2, "test-workspace", "f/test/flow").await;

        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job1,
            &args,
            &db,
        )
        .await?;

        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job2,
            &args,
            &db,
        )
        .await?;

        // Job 1 should be completed with "Debounced by {job2}"
        assert!(is_completed(&db, &job1).await, "job1 should be completed");
        let result: Option<String> = sqlx::query_scalar!(
            "SELECT result::text FROM v2_job_completed WHERE id = $1",
            job1,
        )
        .fetch_one(&db)
        .await?;
        let result_str = result.expect("result should not be null");
        assert!(
            result_str.contains(&format!("Debounced by {job2}")),
            "result should contain 'Debounced by {job2}', got: {result_str}"
        );

        Ok(())
    }

    /// Test: debounce logs are appended to both the debounced job and the new job.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_logs_appended(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_logs_test_key".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        let job1 = Uuid::new_v4();
        let job2 = Uuid::new_v4();
        insert_flow_job(&db, job1, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, job2, "test-workspace", "f/test/flow").await;

        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job1,
            &args,
            &db,
        )
        .await?;

        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job2,
            &args,
            &db,
        )
        .await?;

        // Job 1 (debounced) should have "Debounced by job {job2}" in its logs
        let logs1: Option<String> = sqlx::query_scalar!(
            r#"SELECT logs as "logs!" FROM job_logs WHERE job_id = $1"#,
            job1,
        )
        .fetch_optional(&db)
        .await?;
        let logs1 = logs1.expect("debounced job should have logs");
        assert!(
            logs1.contains(&format!("Debounced by job {job2}")),
            "debounced job logs should contain 'Debounced by job {job2}', got: {logs1}"
        );

        // Job 2 (new) should have "debounce key" in its logs
        let logs2: Option<String> = sqlx::query_scalar!(
            r#"SELECT logs as "logs!" FROM job_logs WHERE job_id = $1"#,
            job2,
        )
        .fetch_optional(&db)
        .await?;
        let logs2 = logs2.expect("new job should have logs");
        assert!(
            logs2.contains("pp_logs_test_key"),
            "new job logs should contain the debounce key, got: {logs2}"
        );

        Ok(())
    }

    /// Test: debounce with negative delay behaves like no debounce.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_negative_delay(db: Pool<Postgres>) -> anyhow::Result<()> {
        let flow_id = Uuid::new_v4();
        insert_flow_job(&db, flow_id, "test-workspace", "f/test/flow").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(-5),
            debounce_key: Some("pp_negative_delay".to_string()),
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

        assert!(
            result.is_none(),
            "negative delay should be treated as no debounce"
        );

        Ok(())
    }

    /// Test: different runnable_paths with no custom debounce_key produce different resolved keys.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_different_paths_independent(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None, // default key includes runnable_path
            ..Default::default()
        };
        let args_hm = empty_args();

        // Two jobs on different paths
        let job_a = Uuid::new_v4();
        let job_b = Uuid::new_v4();
        let job_a2 = Uuid::new_v4();
        insert_flow_job(&db, job_a, "test-workspace", "f/test/flow_a").await;
        insert_flow_job(&db, job_b, "test-workspace", "f/test/flow_b").await;
        insert_flow_job(&db, job_a2, "test-workspace", "f/test/flow_a").await;

        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow_a".to_string()),
            "test-workspace",
            job_a,
            &args,
            &db,
        )
        .await?;

        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow_b".to_string()),
            "test-workspace",
            job_b,
            &args,
            &db,
        )
        .await?;

        // job_a should still be queued (flow_b shouldn't debounce it)
        assert!(is_queued(&db, &job_a).await, "job_a should still be queued");

        // Now push job_a2 on the same path as job_a — should debounce job_a
        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow_a".to_string()),
            "test-workspace",
            job_a2,
            &args,
            &db,
        )
        .await?;

        assert!(
            is_completed(&db, &job_a).await,
            "job_a should be debounced by job_a2"
        );
        assert!(is_queued(&db, &job_b).await, "job_b should be unaffected");
        assert!(is_queued(&db, &job_a2).await, "job_a2 should be queued");

        Ok(())
    }

    /// Test: push-time debounce with custom key containing $args interpolation
    /// differentiates on arg values.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_push_args_interpolation_differentiates(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("user:$args[user_id]".to_string()),
            ..Default::default()
        };

        // Job with user_id = "alice"
        let job_alice1 = Uuid::new_v4();
        insert_noop_job(&db, job_alice1, "test-workspace").await;
        let mut hm = HashMap::new();
        hm.insert(
            "user_id".to_string(),
            RawValue::from_string("\"alice\"".to_string()).unwrap(),
        );
        let args = PushArgs::from(&hm);
        let mut sf = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut sf,
            &None,
            "test-workspace",
            JobKind::Noop,
            job_alice1,
            &args,
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        // Job with user_id = "bob"
        let job_bob = Uuid::new_v4();
        insert_noop_job(&db, job_bob, "test-workspace").await;
        let mut hm = HashMap::new();
        hm.insert(
            "user_id".to_string(),
            RawValue::from_string("\"bob\"".to_string()).unwrap(),
        );
        let args = PushArgs::from(&hm);
        let mut sf = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut sf,
            &None,
            "test-workspace",
            JobKind::Noop,
            job_bob,
            &args,
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        // Both should still be queued (different user_id → different keys)
        assert!(
            is_queued(&db, &job_alice1).await,
            "alice job should still be queued"
        );
        assert!(
            is_queued(&db, &job_bob).await,
            "bob job should still be queued"
        );

        // Another alice job should debounce the first
        let job_alice2 = Uuid::new_v4();
        insert_noop_job(&db, job_alice2, "test-workspace").await;
        let mut hm = HashMap::new();
        hm.insert(
            "user_id".to_string(),
            RawValue::from_string("\"alice\"".to_string()).unwrap(),
        );
        let args = PushArgs::from(&hm);
        let mut sf = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut sf,
            &None,
            "test-workspace",
            JobKind::Noop,
            job_alice2,
            &args,
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        assert!(
            is_completed(&db, &job_alice1).await,
            "alice job 1 should be debounced by alice job 2"
        );
        assert!(
            is_queued(&db, &job_bob).await,
            "bob job should be unaffected"
        );

        Ok(())
    }

    /// Test: debounce_key entry points to the latest job after a chain, and
    /// previous_job_id tracks the one that was just debounced.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_key_tracking_chain(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("tracking_chain_key".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        let job1 = Uuid::new_v4();
        let job2 = Uuid::new_v4();
        let job3 = Uuid::new_v4();
        insert_flow_job(&db, job1, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, job2, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, job3, "test-workspace", "f/test/flow").await;

        // After job 1
        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job1,
            &args,
            &db,
        )
        .await?;
        let dk = get_debounce_key(&db, "tracking_chain_key").await.unwrap();
        assert_eq!(dk.0, job1, "should point to job1");
        assert_eq!(dk.1, None, "no previous job for first entry");
        assert_eq!(dk.2, 0, "debounced_times should be 0");

        // After job 2
        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job2,
            &args,
            &db,
        )
        .await?;
        let dk = get_debounce_key(&db, "tracking_chain_key").await.unwrap();
        assert_eq!(dk.0, job2, "should point to job2");
        assert_eq!(dk.1, Some(job1), "previous should be job1");
        assert_eq!(dk.2, 1, "debounced_times should be 1");

        // After job 3
        let args = PushArgs::from(&args_hm);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job3,
            &args,
            &db,
        )
        .await?;
        let dk = get_debounce_key(&db, "tracking_chain_key").await.unwrap();
        assert_eq!(dk.0, job3, "should point to job3");
        assert_eq!(dk.1, Some(job2), "previous should be job2");
        assert_eq!(dk.2, 2, "debounced_times should be 2");

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

    /// Stress test for push-time maybe_debounce: concurrent operations across multiple keys,
    /// each holding a caller transaction open (simulating push_inner) while debouncing.
    ///
    /// Note: push-time debounce holds a caller tx AND `add_completed_job` needs its own
    /// pool connection, so each concurrent push needs 2 pool connections. The sqlx::test
    /// pool defaults to ~10 connections, so max_concurrent must be <= pool_size/2.
    /// In production, pool_size ~50 allows ~25 concurrent pushes per server.
    ///
    /// Run with:
    ///   cargo test -p windmill-queue --test debounce_test --features private,enterprise \
    ///     -- --ignored test_push_debounce_contention_stress --nocapture
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    #[ignore]
    async fn test_push_debounce_contention_stress(db: Pool<Postgres>) -> anyhow::Result<()> {
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        let num_keys: usize = 10;
        let jobs_per_key: usize = 100;
        let total = num_keys * jobs_per_key;
        // Each push holds 1 tx + add_completed_job needs 1 more = 2 connections.
        // sqlx::test pool is ~10, so max_concurrent = 4 to stay safe.
        let max_concurrent: usize = 4;

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

    /// Helper: insert a flow job with args into v2_job + v2_job_queue + v2_job_runtime.
    async fn insert_flow_job_with_args(
        db: &Pool<Postgres>,
        job_id: Uuid,
        workspace_id: &str,
        runnable_path: &str,
        args: &serde_json::Value,
    ) {
        sqlx::query!(
            "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path, args)
             VALUES ($1, 'flow', 'flow', 'test-user', 'u/test-user', 'test@windmill.dev', $2, $3, $4)",
            job_id,
            workspace_id,
            runnable_path,
            args,
        )
        .execute(db)
        .await
        .expect("insert v2_job with args");

        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
             VALUES ($1, $2, now(), 'flow')",
            job_id,
            workspace_id,
        )
        .execute(db)
        .await
        .expect("insert v2_job_queue");

        sqlx::query!("INSERT INTO v2_job_runtime (id) VALUES ($1)", job_id)
            .execute(db)
            .await
            .expect("insert v2_job_runtime");
    }

    /// Test: debounce_args_to_accumulate excludes the named arg from the debounce key,
    /// so jobs with different values for that arg still debounce each other.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_args_to_accumulate_same_key(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None, // default key (includes args minus accumulated ones)
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };

        // Job 1: items = ["a", "b"]
        let job1 = Uuid::new_v4();
        let args1 = serde_json::json!({"items": ["a", "b"], "other": "same"});
        insert_flow_job_with_args(&db, job1, "test-workspace", "f/test/flow", &args1).await;

        // Job 2: items = ["c", "d"] (different items, same "other")
        let job2 = Uuid::new_v4();
        let args2 = serde_json::json!({"items": ["c", "d"], "other": "same"});
        insert_flow_job_with_args(&db, job2, "test-workspace", "f/test/flow", &args2).await;

        let args_hm1: HashMap<String, Box<RawValue>> = serde_json::from_value(args1).unwrap();
        let args = PushArgs::from(&args_hm1);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job1,
            &args,
            &db,
        )
        .await?;

        let args_hm2: HashMap<String, Box<RawValue>> = serde_json::from_value(args2).unwrap();
        let args = PushArgs::from(&args_hm2);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job2,
            &args,
            &db,
        )
        .await?;

        // Job 1 should be debounced (completed) because "items" is excluded from key
        assert!(
            is_completed(&db, &job1).await,
            "job1 should be debounced despite different 'items' values"
        );
        assert!(
            is_queued(&db, &job2).await,
            "job2 should still be queued (survivor)"
        );

        Ok(())
    }

    /// Test: debounce_args_to_accumulate does NOT cause debouncing when non-accumulated
    /// args differ — only the accumulated arg is excluded from the key.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_args_to_accumulate_different_non_accumulated(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None,
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };

        // Job 1: other = "foo"
        let job1 = Uuid::new_v4();
        let args1 = serde_json::json!({"items": ["a"], "other": "foo"});
        insert_flow_job_with_args(&db, job1, "test-workspace", "f/test/flow", &args1).await;

        // Job 2: other = "bar" (different non-accumulated arg)
        let job2 = Uuid::new_v4();
        let args2 = serde_json::json!({"items": ["b"], "other": "bar"});
        insert_flow_job_with_args(&db, job2, "test-workspace", "f/test/flow", &args2).await;

        let args_hm1: HashMap<String, Box<RawValue>> = serde_json::from_value(args1).unwrap();
        let args = PushArgs::from(&args_hm1);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job1,
            &args,
            &db,
        )
        .await?;

        let args_hm2: HashMap<String, Box<RawValue>> = serde_json::from_value(args2).unwrap();
        let args = PushArgs::from(&args_hm2);
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow".to_string()),
            "test-workspace",
            job2,
            &args,
            &db,
        )
        .await?;

        // Both should still be queued — different "other" arg means different keys
        assert!(
            is_queued(&db, &job1).await,
            "job1 should still be queued (different key due to 'other' arg)"
        );
        assert!(
            is_queued(&db, &job2).await,
            "job2 should still be queued (different key due to 'other' arg)"
        );

        Ok(())
    }

    /// Test: batch tracking correctly groups debounced jobs so that accumulated args
    /// can be collected at execution time via v2_job_debounce_batch.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_args_to_accumulate_batch_collection(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None,
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };

        // Create 3 jobs with different "items" but same "other"
        let jobs: Vec<(Uuid, serde_json::Value)> = vec![
            (
                Uuid::new_v4(),
                serde_json::json!({"items": ["a", "b"], "other": "x"}),
            ),
            (
                Uuid::new_v4(),
                serde_json::json!({"items": ["c"], "other": "x"}),
            ),
            (
                Uuid::new_v4(),
                serde_json::json!({"items": ["d", "e", "f"], "other": "x"}),
            ),
        ];

        for (id, args) in &jobs {
            insert_flow_job_with_args(&db, *id, "test-workspace", "f/test/flow", args).await;
        }

        for (id, args) in &jobs {
            let args_hm: HashMap<String, Box<RawValue>> =
                serde_json::from_value(args.clone()).unwrap();
            let push_args = PushArgs::from(&args_hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                *id,
                &push_args,
                &db,
            )
            .await?;
        }

        let survivor = jobs[2].0; // last job survives
        assert!(
            is_queued(&db, &survivor).await,
            "last job should be the survivor"
        );

        // All 3 jobs should be in the same debounce batch
        let batch_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = ANY($1)",
            &jobs.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        )
        .fetch_all(&db)
        .await?;

        assert_eq!(batch_ids.len(), 3, "all 3 jobs should have batch entries");
        assert!(
            batch_ids.iter().all(|b| *b == batch_ids[0]),
            "all jobs should share the same batch ID"
        );

        // Simulate what maybe_apply_debouncing does: collect accumulated args from batch
        let accumulated: Vec<Option<String>> = sqlx::query_scalar!(
            "WITH ids AS (
                SELECT id as job_id FROM v2_job_debounce_batch WHERE debounce_batch = (
                    SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = $1
                )
            ) SELECT args->>'items' FROM ids LEFT JOIN v2_job ON v2_job.id = ids.job_id",
            survivor,
        )
        .fetch_all(&db)
        .await?;

        // Merge all items arrays (same logic as maybe_apply_debouncing)
        let mut all_items: Vec<serde_json::Value> = vec![];
        for s in accumulated.iter().flatten() {
            let items: Vec<serde_json::Value> = serde_json::from_str(s).unwrap();
            all_items.extend(items);
        }
        all_items.sort_by(|a, b| a.as_str().unwrap().cmp(b.as_str().unwrap()));

        assert_eq!(
            all_items,
            vec!["a", "b", "c", "d", "e", "f"],
            "accumulated items should contain all items from all debounced jobs"
        );

        Ok(())
    }

    /// Test: maybe_apply_debouncing actually merges accumulated args into the surviving job's args.
    /// This is an end-to-end test that sets up runnable_settings in the DB, constructs a
    /// PulledJobResult, and verifies the accumulated arg is written into the job.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_maybe_apply_debouncing_merges_accumulated_args(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        use windmill_common::runnable_settings::RunnableSettings;
        use windmill_common::runnable_settings::{
            insert_rs, ConcurrencySettings, RunnableSettingsTrait,
        };
        use windmill_queue::{MiniPulledJob, PulledJob, PulledJobResult};

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None,
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };

        // Insert debouncing_settings and concurrency_settings into the DB
        let debouncing_hash = settings.insert_cached(&db).await?;
        let concurrency_hash = ConcurrencySettings::default().insert_cached(&db).await?;

        let rs = RunnableSettings {
            debouncing_settings: debouncing_hash,
            concurrency_settings: concurrency_hash,
        };
        let rs_handle = insert_rs(rs, &db).await?;

        // Create 3 jobs with different "items" values
        let jobs: Vec<(Uuid, serde_json::Value)> = vec![
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [1, 2], "other": "x"}),
            ),
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [3], "other": "x"}),
            ),
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [4, 5, 6], "other": "x"}),
            ),
        ];

        for (id, args) in &jobs {
            insert_flow_job_with_args(&db, *id, "test-workspace", "f/test/flow", args).await;
            // Set runnable_settings_handle on the job
            sqlx::query!(
                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                rs_handle,
                id,
            )
            .execute(&db)
            .await?;
        }

        // Debounce all 3 jobs via post-preprocessing
        for (id, args) in &jobs {
            let args_hm: HashMap<String, Box<RawValue>> =
                serde_json::from_value(args.clone()).unwrap();
            let push_args = PushArgs::from(&args_hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                *id,
                &push_args,
                &db,
            )
            .await?;
        }

        let survivor_id = jobs[2].0;
        assert!(
            is_queued(&db, &survivor_id).await,
            "last job should survive"
        );

        // Build a PulledJobResult for the surviving job (mimicking what the worker does)
        let survivor_args: HashMap<String, Box<RawValue>> =
            serde_json::from_value(jobs[2].1.clone()).unwrap();

        let mini = MiniPulledJob {
            workspace_id: "test-workspace".to_string(),
            id: survivor_id,
            args: Some(sqlx::types::Json(survivor_args)),
            parent_job: None,
            created_by: "test-user".to_string(),
            scheduled_for: Utc::now(),
            started_at: None,
            runnable_path: Some("f/test/flow".to_string()),
            kind: JobKind::Flow,
            runnable_id: None,
            canceled_reason: None,
            canceled_by: None,
            permissioned_as: "u/test-user".to_string(),
            permissioned_as_email: "test@windmill.dev".to_string(),
            flow_status: None,
            tag: "flow".to_string(),
            script_lang: None,
            same_worker: false,
            pre_run_error: None,
            concurrent_limit: None,
            concurrency_time_window_s: None,
            flow_innermost_root_job: None,
            root_job: None,
            timeout: None,
            flow_step_id: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            priority: None,
            preprocessed: None,
            script_entrypoint_override: None,
            trigger: None,
            trigger_kind: None,
            visible_to_owner: false,
            permissioned_as_end_user_email: None,
            runnable_settings_handle: rs_handle,
        };

        let pulled = PulledJob {
            job: mini,
            raw_code: None,
            raw_lock: None,
            raw_flow: None,
            parent_runnable_path: None,
            permissioned_as_email: None,
            permissioned_as_username: None,
            permissioned_as_is_admin: None,
            permissioned_as_is_operator: None,
            permissioned_as_groups: None,
            permissioned_as_folders: None,
        };

        let mut result = PulledJobResult {
            job: Some(pulled),
            suspended: false,
            missing_concurrency_key: false,
            error_while_preprocessing: None,
        };

        // Call the real maybe_apply_debouncing
        result.maybe_apply_debouncing(&db).await?;

        // The job should still be present (not debounced itself)
        assert!(
            result.job.is_some(),
            "survivor job should not be nulled out"
        );

        let job = result.job.unwrap();
        let args = job.job.args.expect("args should be present");
        let items_raw = args.get("items").expect("items arg should exist");
        let items: Vec<serde_json::Value> = serde_json::from_str(items_raw.get())?;

        // Should have all 6 items accumulated from all 3 debounced jobs
        let mut item_nums: Vec<i64> = items
            .iter()
            .map(|v| v.as_i64().expect("item should be a number"))
            .collect();
        item_nums.sort();

        assert_eq!(
            item_nums,
            vec![1, 2, 3, 4, 5, 6],
            "accumulated items should contain all values from all debounced jobs"
        );

        // "other" arg should be unchanged
        let other_raw = args.get("other").expect("other arg should exist");
        let other: String = serde_json::from_str(other_raw.get())?;
        assert_eq!(other, "x", "non-accumulated arg should be unchanged");

        Ok(())
    }
}
