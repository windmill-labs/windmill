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

    /// Test: max_total_debounces_amount limit - push-time debounce deletes key and
    /// completes previous job when limit is reached. With max=2, the 2nd event
    /// (debounced_times=1, total events=2) triggers the limit.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_max_count_limit(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("count_limit_key".to_string()),
            max_total_debounces_amount: Some(2),
            ..Default::default()
        };
        let args_hm = empty_args();

        // With max=2: job1 debounced (dt=0), job2 triggers limit (dt=1, 1+1>=2),
        // job3 debounced (fresh INSERT), job4 triggers limit (dt=1 again)
        let mut jobs = Vec::new();
        let mut scheduled_fors = Vec::new();
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
            scheduled_fors.push(scheduled_for);
        }

        // Job 1: debounced (scheduled_for set)
        assert!(
            scheduled_fors[0].is_some(),
            "job1 should be debounced (scheduled_for set)"
        );
        // Job 2: limit exceeded → scheduled_for cleared, previous job completed
        assert!(
            scheduled_fors[1].is_none(),
            "job2 should execute immediately (limit exceeded)"
        );
        assert!(
            is_completed(&db, &jobs[0]).await,
            "job1 should be completed (debounced by job2 at limit)"
        );
        // Job 3: new batch (fresh INSERT after DELETE)
        assert!(
            scheduled_fors[2].is_some(),
            "job3 should be debounced (new batch)"
        );
        // Job 4: limit exceeded again
        assert!(
            scheduled_fors[3].is_none(),
            "job4 should execute immediately (limit exceeded)"
        );
        assert!(
            is_completed(&db, &jobs[2]).await,
            "job3 should be completed (debounced by job4 at limit)"
        );

        // The debounce_key entry should be deleted after the last limit exceeded
        let dk = get_debounce_key(&db, "count_limit_key").await;
        assert!(
            dk.is_none(),
            "debounce_key entry should be deleted after limit exceeded"
        );

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

    /// Test: Post-preprocessing debounce with max count limit deletes the debounce_key entry
    /// and completes the previous job. With max=2, the 2nd event triggers the limit.
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

        // With max=2: job1 debounced (dt=0), job2 limit exceeded (dt=1, 1+1>=2),
        // job3 debounced (fresh INSERT), job4 limit exceeded (dt=1 again)
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

        // Job 1: debounced (first in batch)
        assert!(results[0].is_some(), "first job should get scheduled_for");

        // Job 2: limit exceeded → execute immediately, complete job1
        assert!(
            results[1].is_none(),
            "second job should execute immediately (limit exceeded)"
        );
        assert!(
            is_completed(&db, &jobs[0]).await,
            "job1 should be completed (debounced by job2 at limit)"
        );

        // Job 3: new batch (fresh INSERT after DELETE)
        assert!(
            results[2].is_some(),
            "third job should get scheduled_for (new batch)"
        );

        // Job 4: limit exceeded again → execute immediately, complete job3
        assert!(
            results[3].is_none(),
            "fourth job should execute immediately (limit exceeded)"
        );
        assert!(
            is_completed(&db, &jobs[2]).await,
            "job3 should be completed (debounced by job4 at limit)"
        );

        // debounce_key should be deleted after the last limit exceeded
        let dk = get_debounce_key(&db, "pp_max_count_key").await;
        assert!(
            dk.is_none(),
            "debounce_key entry should be deleted when limits exceeded"
        );

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
        // When limit is exceeded, the function returns None (execute immediately)
        assert!(
            r2.is_none(),
            "should return None when time limit is exceeded"
        );

        // Verify the debounce_key entry is deleted (not just reset)
        let dk = get_debounce_key(&db, "pp_time_limit_key").await;
        assert!(
            dk.is_none(),
            "debounce_key entry should be deleted when time limit exceeded"
        );

        // Job 1 should be completed (debounced by job2 when limit exceeded)
        assert!(
            is_completed(&db, &job1).await,
            "job1 should be completed (debounced by job2 at time limit)"
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

        // scheduled_for should be cleared (execute immediately when limit exceeded)
        assert!(
            sf2.is_none(),
            "scheduled_for should be cleared when time limit exceeded"
        );

        // debounce_key should be deleted (not just reset)
        let dk = get_debounce_key(&db, "push_time_limit_key").await;
        assert!(
            dk.is_none(),
            "debounce_key entry should be deleted when time limit exceeded"
        );

        // Job 1 should be completed (debounced by job2 at time limit)
        assert!(
            is_completed(&db, &job1).await,
            "job1 should be completed (debounced by job2 at time limit)"
        );

        Ok(())
    }

    /// Test: max_count boundary — with max=3, the 3rd event (debounced_times=2,
    /// total events=3) triggers the limit. Events 1-2 debounce, event 3 launches.
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
        // With max=3: job1 dt=0 (1 event), job2 dt=1 (2 events), job3 dt=2 (3 events → limit),
        // job4 dt=0 (new batch), job5 dt=1 (2 events)
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

        // Jobs 1-2: debounced (within limit)
        assert!(results[0].is_some(), "job 1 should get scheduled_for");
        assert!(results[1].is_some(), "job 2 should get scheduled_for");

        // Job 3: limit exceeded (dt=2, 2+1=3 >= 3) → execute immediately
        assert!(
            results[2].is_none(),
            "job 3 should return None (limit exceeded)"
        );

        // Jobs 4-5: new batch after DELETE
        assert!(
            results[3].is_some(),
            "job 4 should get scheduled_for (new batch)"
        );
        assert!(
            results[4].is_some(),
            "job 5 should get scheduled_for (within limit)"
        );

        // debounce_key should exist (pointing to job5, within new batch)
        let dk = get_debounce_key(&db, "pp_count_boundary_key").await;
        assert!(
            dk.is_some(),
            "debounce_key entry should exist (new batch in progress)"
        );

        Ok(())
    }

    /// Test: after a max_count limit exceeded, a new batch starts completely fresh.
    /// The debounce_key entry is deleted, so the next cycle starts with a fresh INSERT.
    /// With max=2: every 2 events forms a batch (1st debounced, 2nd triggers limit).
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

        // With max=2: job1 debounced, job2 triggers limit (completes job1),
        // job3 debounced (new batch), job4 triggers limit (completes job3)
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
        assert!(cycle1_results[0].is_some(), "cycle1 job1 debounced");
        assert!(
            cycle1_results[1].is_none(),
            "cycle1 job2 should execute immediately (limit)"
        );
        assert!(
            cycle1_results[2].is_some(),
            "cycle1 job3 debounced (new batch)"
        );
        assert!(
            cycle1_results[3].is_none(),
            "cycle1 job4 should execute immediately (limit)"
        );

        // Verify debounce_key entry is deleted after limit exceeded
        let dk = get_debounce_key(&db, "pp_reset_cycle_key").await;
        assert!(
            dk.is_none(),
            "debounce_key entry should be deleted after limit exceeded"
        );

        // Cycle 2: completely fresh batch since entry was deleted
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
        // Cycle 2 behaves identically: [debounced, limit, debounced, limit]
        assert!(
            cycle2_results[0].is_some(),
            "cycle2 job1 debounced (fresh INSERT)"
        );
        assert!(
            cycle2_results[1].is_none(),
            "cycle2 job2 should execute immediately (limit)"
        );
        assert!(
            cycle2_results[2].is_some(),
            "cycle2 job3 debounced (new batch)"
        );
        assert!(
            cycle2_results[3].is_none(),
            "cycle2 job4 should execute immediately (limit)"
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

    /// Test: the limit-triggered job stays in the same batch as its predecessors
    /// (so args can be accumulated), and the next batch after reset is different.
    /// With max=3: batch of 3 events (2 debounced + 1 limit trigger), then new batch.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_batch_id_changes_on_reset(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("pp_batch_reset_id_test".to_string()),
            max_total_debounces_amount: Some(3),
            ..Default::default()
        };
        let args_hm = empty_args();

        // With max=3: jobs 1-2 debounced (dt=0,1), job 3 triggers limit (dt=2, 2+1>=3)
        let mut batch1_jobs = Vec::new();
        for _ in 0..3 {
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

        // Batch 2: jobs 4-5 (new batch after DELETE, within limit)
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

        // Job 3 (limit-triggered) stays in the same batch as jobs 1-2
        // so that maybe_apply_debouncing can accumulate args from all 3.
        let trigger_batch: i64 = sqlx::query_scalar!(
            "SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = $1",
            batch1_jobs[2],
        )
        .fetch_one(&db)
        .await?;

        assert_eq!(
            batch1_id, trigger_batch,
            "limit-triggered job should stay in the same batch for arg accumulation"
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

    /// Test: 5 webhook calls with max_total_debounces_amount=2 and debounce_args_to_accumulate.
    /// Simulates the flow described by the user: debounce_delay_s=50, max=2, accumulate x.
    ///
    /// Expected behavior:
    ///   Call 1: debounced (first in batch, scheduled_for set)
    ///   Call 2: launched immediately (limit reached at 2 total events, completes call 1)
    ///   Call 3: debounced (new batch starts fresh)
    ///   Call 4: launched immediately (limit reached again, completes call 3)
    ///   Call 5: debounced (new batch, waiting for delay or more events)
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_webhook_5_calls_max_2(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(50),
            debounce_key: Some("webhook_5calls_key".to_string()),
            max_total_debounces_amount: Some(2),
            debounce_args_to_accumulate: Some(vec!["x".to_string()]),
            ..Default::default()
        };

        let mut jobs = Vec::new();
        let mut results = Vec::new();

        for i in 0..5 {
            let id = Uuid::new_v4();
            let args_val = serde_json::json!({"x": [i + 1]});
            insert_flow_job_with_args(&db, id, "test-workspace", "f/test/flow", &args_val).await;
            jobs.push(id);

            let args_hm: HashMap<String, Box<RawValue>> = serde_json::from_value(args_val).unwrap();
            let args = PushArgs::from(&args_hm);
            let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow".to_string()),
                "test-workspace",
                id,
                &args,
                &db,
            )
            .await?;
            results.push(result);
        }

        // Call 1: debounced (first in batch)
        assert!(
            results[0].is_some(),
            "call 1 should be debounced (scheduled_for set)"
        );

        // Call 2: launched immediately (limit exceeded: dt=1, 1+1 >= 2)
        assert!(
            results[1].is_none(),
            "call 2 should launch immediately (limit reached at 2 total events)"
        );

        // Call 3: debounced (new batch, fresh INSERT after key was deleted by call 2)
        assert!(
            results[2].is_some(),
            "call 3 should be debounced (new batch started)"
        );

        // Call 4: launched immediately (limit exceeded again)
        assert!(
            results[3].is_none(),
            "call 4 should launch immediately (limit reached again)"
        );

        // Call 5: debounced (new batch)
        assert!(
            results[4].is_some(),
            "call 5 should be debounced (new batch, waiting for delay)"
        );

        // Verify final state after all 5 calls:
        // Completed: jobs[0] (debounced by call 2), jobs[2] (debounced by call 4)
        assert!(
            is_completed(&db, &jobs[0]).await,
            "job from call 1 should be completed (debounced by call 2)"
        );
        assert!(
            is_completed(&db, &jobs[2]).await,
            "job from call 3 should be completed (debounced by call 4)"
        );

        // Queued: jobs[1] (launched immediately), jobs[3] (launched immediately), jobs[4] (debounced, waiting)
        assert!(
            is_queued(&db, &jobs[1]).await,
            "call 2's job should be queued (launched immediately)"
        );
        assert!(
            is_queued(&db, &jobs[3]).await,
            "call 4's job should be queued (launched immediately)"
        );
        assert!(
            is_queued(&db, &jobs[4]).await,
            "call 5's job should be queued (debounced, waiting)"
        );

        // debounce_key should exist pointing to call 5's job (the active batch)
        let dk = get_debounce_key(&db, "webhook_5calls_key").await;
        assert!(dk.is_some(), "debounce_key should exist for call 5's batch");
        let (dk_job_id, _, dk_times) = dk.unwrap();
        assert_eq!(dk_job_id, jobs[4], "debounce_key should point to call 5");
        assert_eq!(
            dk_times, 0,
            "debounced_times should be 0 (first in new batch)"
        );

        Ok(())
    }

    /// Test: 5 webhook calls with max_total_debounces_amount=2 verifies both
    /// debounce behavior AND accumulated arg values via maybe_apply_debouncing.
    ///
    /// Each call sends {x: [i]}. Expected:
    ///   Call 1 (x=[1]): debounced
    ///   Call 2 (x=[2]): fires immediately (limit), accumulated x=[1,2]
    ///   Call 3 (x=[3]): debounced (new batch)
    ///   Call 4 (x=[4]): fires immediately (limit), accumulated x=[3,4]
    ///   Call 5 (x=[5]): debounced (new batch), only x=[5] since batch has 1 job
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_max_count_accumulation(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(50),
            debounce_key: Some("max_count_accum_key".to_string()),
            max_total_debounces_amount: Some(2),
            debounce_args_to_accumulate: Some(vec!["x".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let mut jobs = Vec::new();
        let mut results = Vec::new();

        for i in 0..5 {
            let id = Uuid::new_v4();
            let args_val = serde_json::json!({"x": [i + 1]});
            insert_flow_job_with_args(&db, id, "test-workspace", "f/test/accum_flow", &args_val)
                .await;
            jobs.push((id, args_val.clone()));

            let args_hm: HashMap<String, Box<RawValue>> = serde_json::from_value(args_val).unwrap();
            let args = PushArgs::from(&args_hm);
            let result = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/accum_flow".to_string()),
                "test-workspace",
                id,
                &args,
                &db,
            )
            .await?;
            results.push(result);
        }

        // Verify debounce behavior
        assert!(results[0].is_some(), "call 1 debounced");
        assert!(results[1].is_none(), "call 2 fires immediately");
        assert!(results[2].is_some(), "call 3 debounced");
        assert!(results[3].is_none(), "call 4 fires immediately");
        assert!(results[4].is_some(), "call 5 debounced");

        // Call 2 fires immediately with MaxCountExceeded.
        // Simulate worker: store runnable_settings_handle, then call maybe_apply_debouncing.
        let survivor_2 = jobs[1].0;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            survivor_2,
        )
        .execute(&db)
        .await?;

        let mut pulled_2 = make_pulled_job_result(
            survivor_2,
            "test-workspace",
            "f/test/accum_flow",
            &jobs[1].1,
            JobKind::Flow,
            "flow",
            rs_handle,
        );
        pulled_2.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled_2, &[1, 2], "x");

        // Call 4 fires immediately with MaxCountExceeded.
        let survivor_4 = jobs[3].0;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            survivor_4,
        )
        .execute(&db)
        .await?;

        let mut pulled_4 = make_pulled_job_result(
            survivor_4,
            "test-workspace",
            "f/test/accum_flow",
            &jobs[3].1,
            JobKind::Flow,
            "flow",
            rs_handle,
        );
        pulled_4.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled_4, &[3, 4], "x");

        // Call 5 is debounced (only job in its batch so far).
        // When eventually pulled, it should only have its own args.
        let survivor_5 = jobs[4].0;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            survivor_5,
        )
        .execute(&db)
        .await?;

        let mut pulled_5 = make_pulled_job_result(
            survivor_5,
            "test-workspace",
            "f/test/accum_flow",
            &jobs[4].1,
            JobKind::Flow,
            "flow",
            rs_handle,
        );
        pulled_5.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled_5, &[5], "x");

        Ok(())
    }

    /// Test: same as above but triggered by max_total_debouncing_time instead of count.
    /// Uses a very short time window so that the 2nd call exceeds it.
    ///
    /// Call 1 (x=[10]): debounced
    /// -- sleep past max_total_debouncing_time --
    /// Call 2 (x=[20]): fires immediately (time exceeded), accumulated x=[10,20]
    /// Call 3 (x=[30]): debounced (new batch)
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_max_time_accumulation(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(50),
            debounce_key: Some("max_time_accum_key".to_string()),
            max_total_debouncing_time: Some(1), // 1 second
            debounce_args_to_accumulate: Some(vec!["x".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // Call 1: debounced
        let id1 = Uuid::new_v4();
        let args1 = serde_json::json!({"x": [10]});
        insert_flow_job_with_args(&db, id1, "test-workspace", "f/test/time_accum", &args1).await;

        let args_hm: HashMap<String, Box<RawValue>> =
            serde_json::from_value(args1.clone()).unwrap();
        let r1 = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/time_accum".to_string()),
            "test-workspace",
            id1,
            &PushArgs::from(&args_hm),
            &db,
        )
        .await?;
        assert!(r1.is_some(), "call 1 should be debounced");

        // Wait for time to exceed
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Call 2: time exceeded, fires immediately
        let id2 = Uuid::new_v4();
        let args2 = serde_json::json!({"x": [20]});
        insert_flow_job_with_args(&db, id2, "test-workspace", "f/test/time_accum", &args2).await;

        let args_hm2: HashMap<String, Box<RawValue>> =
            serde_json::from_value(args2.clone()).unwrap();
        let r2 = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/time_accum".to_string()),
            "test-workspace",
            id2,
            &PushArgs::from(&args_hm2),
            &db,
        )
        .await?;
        assert!(
            r2.is_none(),
            "call 2 should fire immediately (time exceeded)"
        );

        // Simulate worker: store handle and call maybe_apply_debouncing
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            id2,
        )
        .execute(&db)
        .await?;

        let mut pulled = make_pulled_job_result(
            id2,
            "test-workspace",
            "f/test/time_accum",
            &args2,
            JobKind::Flow,
            "flow",
            rs_handle,
        );
        pulled.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled, &[10, 20], "x");

        // Call 3: new batch, debounced
        let id3 = Uuid::new_v4();
        let args3 = serde_json::json!({"x": [30]});
        insert_flow_job_with_args(&db, id3, "test-workspace", "f/test/time_accum", &args3).await;

        let args_hm3: HashMap<String, Box<RawValue>> =
            serde_json::from_value(args3.clone()).unwrap();
        let r3 = windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/time_accum".to_string()),
            "test-workspace",
            id3,
            &PushArgs::from(&args_hm3),
            &db,
        )
        .await?;
        assert!(r3.is_some(), "call 3 should be debounced (new batch)");

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
            retry_settings: None,
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

        // Verify accumulated args were persisted to v2_job (needed for flows
        // where subsequent steps re-read args from the DB)
        let db_args: Option<serde_json::Value> =
            sqlx::query_scalar!("SELECT args FROM v2_job WHERE id = $1", survivor_id,)
                .fetch_one(&db)
                .await?;
        let db_args = db_args.expect("v2_job args should not be null after accumulation");
        let db_items = db_args
            .get("items")
            .expect("persisted args should contain 'items'");
        let db_items: Vec<i64> =
            serde_json::from_value::<Vec<serde_json::Value>>(db_items.clone())?
                .iter()
                .map(|v| v.as_i64().unwrap())
                .collect();
        let mut db_items_sorted = db_items.clone();
        db_items_sorted.sort();
        assert_eq!(
            db_items_sorted,
            vec![1, 2, 3, 4, 5, 6],
            "persisted args in v2_job should contain all accumulated items"
        );

        Ok(())
    }

    // =========================================================================
    // Helpers for focused accumulation tests
    // =========================================================================

    /// Helper: insert a flow job with flow_status and v2_job_status, simulating a flow with a preprocessor.
    async fn insert_flow_job_with_preprocessor(
        db: &Pool<Postgres>,
        job_id: Uuid,
        workspace_id: &str,
        runnable_path: &str,
        preprocessed: bool,
        flow_status_step: i32,
    ) {
        sqlx::query!(
            "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path, preprocessed)
             VALUES ($1, 'flow', 'flow', 'test-user', 'u/test-user', 'test@windmill.dev', $2, $3, $4)",
            job_id,
            workspace_id,
            runnable_path,
            preprocessed,
        )
        .execute(db)
        .await
        .expect("insert v2_job");

        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag, running)
             VALUES ($1, $2, now(), 'flow', false)",
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

        let flow_status = serde_json::json!({
            "step": flow_status_step,
            "modules": [
                {"id": "a", "type": "WaitingForPriorSteps"}
            ],
            "failure_module": {
                "type": "WaitingForPriorSteps",
                "id": "failure"
            },
            "preprocessor_module": {
                "type": "Success",
                "id": "preprocessor",
                "job": Uuid::new_v4().to_string(),
                "flow_jobs": null,
                "flow_jobs_success": null,
                "branch_chosen": null,
                "approvers": [],
                "failed_retries": [],
                "skipped": false
            },
            "cleanup_module": {"flow_jobs_to_clean": []},
            "retry": {"fail_count": 0, "failed_jobs": []}
        });

        sqlx::query!(
            "INSERT INTO v2_job_status (id, flow_status) VALUES ($1, $2)",
            job_id,
            flow_status,
        )
        .execute(db)
        .await
        .expect("insert v2_job_status");
    }

    /// Helper: insert a script job with args into v2_job + v2_job_queue + v2_job_runtime.
    async fn insert_script_job_with_args(
        db: &Pool<Postgres>,
        job_id: Uuid,
        workspace_id: &str,
        runnable_path: &str,
        args: &serde_json::Value,
    ) {
        sqlx::query!(
            "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, runnable_path, args)
             VALUES ($1, 'script', 'deno', 'test-user', 'u/test-user', 'test@windmill.dev', $2, $3, $4)",
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
             VALUES ($1, $2, now(), 'deno')",
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

    /// Helper: build a PulledJobResult from job data, for calling maybe_apply_debouncing.
    fn make_pulled_job_result(
        job_id: Uuid,
        workspace_id: &str,
        runnable_path: &str,
        args: &serde_json::Value,
        kind: JobKind,
        tag: &str,
        rs_handle: Option<i64>,
    ) -> windmill_queue::PulledJobResult {
        use windmill_queue::{MiniPulledJob, PulledJob, PulledJobResult};

        let args_hm: HashMap<String, Box<RawValue>> = serde_json::from_value(args.clone()).unwrap();

        let mini = MiniPulledJob {
            workspace_id: workspace_id.to_string(),
            id: job_id,
            args: Some(sqlx::types::Json(args_hm)),
            parent_job: None,
            created_by: "test-user".to_string(),
            scheduled_for: Utc::now(),
            started_at: None,
            runnable_path: Some(runnable_path.to_string()),
            kind,
            runnable_id: None,
            canceled_reason: None,
            canceled_by: None,
            permissioned_as: "u/test-user".to_string(),
            permissioned_as_email: "test@windmill.dev".to_string(),
            flow_status: None,
            tag: tag.to_string(),
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

        PulledJobResult {
            job: Some(pulled),
            suspended: false,
            missing_concurrency_key: false,
            error_while_preprocessing: None,
        }
    }

    /// Helper: insert debouncing settings into the DB and return the runnable_settings_handle.
    async fn setup_debouncing_settings(
        db: &Pool<Postgres>,
        settings: &DebouncingSettings,
    ) -> Option<i64> {
        use windmill_common::runnable_settings::{
            insert_rs, ConcurrencySettings, RunnableSettings, RunnableSettingsTrait,
        };

        let debouncing_hash = settings.insert_cached(db).await.expect("insert debouncing");
        let concurrency_hash = ConcurrencySettings::default()
            .insert_cached(db)
            .await
            .expect("insert concurrency");

        insert_rs(
            RunnableSettings {
                debouncing_settings: debouncing_hash,
                concurrency_settings: concurrency_hash,
                retry_settings: None,
            },
            db,
        )
        .await
        .expect("insert rs")
    }

    /// Helper: assert that accumulated items match expected values.
    fn assert_accumulated_items(
        result: &windmill_queue::PulledJobResult,
        expected: &[i64],
        arg_name: &str,
    ) {
        let job = result
            .job
            .as_ref()
            .expect("survivor job should not be nulled out");
        let args = job.job.args.as_ref().expect("args should be present");
        let items_raw = args.get(arg_name).expect("accumulated arg should exist");
        let items: Vec<serde_json::Value> =
            serde_json::from_str(items_raw.get()).expect("items should be valid JSON array");

        let mut item_nums: Vec<i64> = items
            .iter()
            .map(|v| v.as_i64().expect("item should be a number"))
            .collect();
        item_nums.sort();

        assert_eq!(
            item_nums, expected,
            "accumulated items should contain all values from all debounced jobs"
        );
    }

    // =========================================================================
    // Argument accumulation tests for scripts, flows, flows with preprocessor
    // =========================================================================

    /// Test: Script debounce accumulation via push-time maybe_debounce + maybe_apply_debouncing.
    /// Pushes 3 script jobs with different "items" values, verifies they accumulate.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_script_debounce_accumulation(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("script_accum_key".to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

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

        // Insert script jobs and set runnable_settings_handle
        for (id, args) in &jobs {
            insert_script_job_with_args(&db, *id, "test-workspace", "f/test/script", args).await;
            sqlx::query!(
                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                rs_handle,
                id,
            )
            .execute(&db)
            .await?;
        }

        // Push-time debounce: each job debounces the previous one
        for (id, args) in &jobs {
            let args_hm: HashMap<String, Box<RawValue>> =
                serde_json::from_value(args.clone()).unwrap();
            let push_args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Script,
                *id,
                &push_args,
                &mut tx,
            )
            .await?;
            tx.commit().await?;
        }

        // Last job should survive, first two should be completed (skipped)
        let survivor_id = jobs[2].0;
        assert!(
            is_queued(&db, &survivor_id).await,
            "last job should survive in queue"
        );
        assert!(
            is_completed(&db, &jobs[0].0).await,
            "job 0 should be debounced"
        );
        assert!(
            is_completed(&db, &jobs[1].0).await,
            "job 1 should be debounced"
        );

        // Call maybe_apply_debouncing on the survivor
        let mut result = make_pulled_job_result(
            survivor_id,
            "test-workspace",
            "f/test/script",
            &jobs[2].1,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        result.maybe_apply_debouncing(&db).await?;

        // Verify accumulation
        assert_accumulated_items(&result, &[1, 2, 3, 4, 5, 6], "items");

        // "other" arg should be unchanged
        let job = result.job.as_ref().unwrap();
        let other_raw = job.job.args.as_ref().unwrap().get("other").unwrap();
        let other: String = serde_json::from_str(other_raw.get())?;
        assert_eq!(other, "x", "non-accumulated arg should be unchanged");

        Ok(())
    }

    /// Helper: push a script job through push-time `maybe_debounce` with the given key.
    /// Returns the args JSON it was pushed with.
    async fn push_debounced_script(
        db: &Pool<Postgres>,
        id: Uuid,
        items: Vec<i64>,
        settings: &DebouncingSettings,
        rs_handle: Option<i64>,
    ) -> serde_json::Value {
        let args_val = serde_json::json!({ "items": items });
        insert_script_job_with_args(db, id, "test-workspace", "f/test/script", &args_val).await;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            id,
        )
        .execute(db)
        .await
        .unwrap();
        let args_hm: HashMap<String, Box<RawValue>> =
            serde_json::from_value(args_val.clone()).unwrap();
        let push_args = PushArgs::from(&args_hm);
        let mut scheduled_for = None;
        let mut tx = db.begin().await.unwrap();
        windmill_queue::jobs_ee::maybe_debounce(
            settings,
            &mut scheduled_for,
            &Some("f/test/script".to_string()),
            "test-workspace",
            JobKind::Script,
            id,
            &push_args,
            &mut tx,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        args_val
    }

    /// Regression test for the "running survivor" data-loss bug.
    ///
    /// When a debounce survivor has already been pulled and is executing (running), it
    /// has committed its batch and can no longer accumulate later arrivals. The old
    /// behavior superseded the running survivor anyway: it was completed/skipped
    /// ("Debounced Running by ...") and deleted from the queue, silently dropping its
    /// accumulated work, while the late arrival could not merge into it.
    ///
    /// Fix: a late arrival that finds the current survivor already running starts a
    /// FRESH debounce window. The running survivor is left to finish with its own
    /// accumulated batch; the late arrival accumulates only its own batch. No job is
    /// killed and no item is dropped or double-run.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_running_survivor_not_superseded(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "running_survivor_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // J1, J2 share a window; J2 is the survivor with batch {J1, J2}.
        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;
        assert!(is_completed(&db, &j1).await, "J1 should be debounced by J2");
        assert!(is_queued(&db, &j2).await, "J2 should be the survivor");

        // Worker pulls J2 and marks it running: the window where the key still points
        // to J2 and its batch is intact, but J2 can no longer accumulate new arrivals.
        sqlx::query!(
            "UPDATE v2_job_queue SET running = true, started_at = now() WHERE id = $1",
            j2
        )
        .execute(&db)
        .await?;

        // Late arrival J3 while J2 is running.
        let j3 = Uuid::new_v4();
        let j3_args = push_debounced_script(&db, j3, vec![3], &settings, rs_handle).await;

        // The running survivor J2 must NOT be superseded: still queued, not completed.
        assert!(
            is_queued(&db, &j2).await,
            "running survivor J2 must stay in the queue"
        );
        assert!(
            !is_completed(&db, &j2).await,
            "running survivor J2 must not be completed/skipped"
        );

        // J3 must own the debounce key as the head of a FRESH window (no previous job).
        let (dk_job, dk_prev, dk_times) = get_debounce_key(&db, key)
            .await
            .expect("debounce key exists");
        assert_eq!(dk_job, j3, "J3 should hold the debounce key");
        assert!(
            dk_prev.is_none(),
            "J3 should start a fresh window with no previous job (got {dk_prev:?})"
        );
        assert_eq!(
            dk_times, 0,
            "fresh window should reset debounced_times to 0"
        );

        // The running survivor J2 accumulates only its own committed batch: [1, 2].
        let mut j2_res = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &serde_json::json!({"items": [2]}),
            JobKind::Script,
            "deno",
            rs_handle,
        );
        j2_res.maybe_apply_debouncing(&db).await?;
        assert!(
            j2_res.job.is_some(),
            "running survivor J2 must still execute (not nulled out)"
        );
        assert_accumulated_items(&j2_res, &[1, 2], "items");

        // The late arrival J3 accumulates only its own batch: [3]. No overlap with J2.
        let mut j3_res = make_pulled_job_result(
            j3,
            "test-workspace",
            "f/test/script",
            &j3_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        j3_res.maybe_apply_debouncing(&db).await?;
        assert!(j3_res.job.is_some(), "J3 must execute");
        assert_accumulated_items(&j3_res, &[3], "items");

        Ok(())
    }

    /// A second arrival debouncing a NON-running survivor must keep accumulating into
    /// the same batch (the normal debounce behavior must be unchanged by the
    /// running-survivor guard).
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_queued_survivor_still_accumulates(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "queued_survivor_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // Three arrivals, none running: classic debounce, all accumulate into J3.
        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        let j3 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;
        let j3_args = push_debounced_script(&db, j3, vec![3], &settings, rs_handle).await;

        assert!(is_completed(&db, &j1).await, "J1 debounced");
        assert!(is_completed(&db, &j2).await, "J2 debounced");
        assert!(is_queued(&db, &j3).await, "J3 is the survivor");

        // Window keeps growing: J3 is the third arrival in the same batch.
        let (dk_job, dk_prev, dk_times) = get_debounce_key(&db, key)
            .await
            .expect("debounce key exists");
        assert_eq!(dk_job, j3);
        assert_eq!(dk_prev, Some(j2), "previous job should be J2");
        assert_eq!(dk_times, 2, "debounced_times should keep incrementing");

        let mut j3_res = make_pulled_job_result(
            j3,
            "test-workspace",
            "f/test/script",
            &j3_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        j3_res.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&j3_res, &[1, 2, 3], "items");

        Ok(())
    }

    /// The running-survivor guard must also apply to plain debounce (delay only, no
    /// argument accumulation): a running survivor must never be completed/skipped.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_running_survivor_no_accumulation(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "running_no_accum_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            // No debounce_args_to_accumulate.
            ..Default::default()
        };

        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        insert_noop_job(&db, j1, "test-workspace").await;
        insert_noop_job(&db, j2, "test-workspace").await;

        let push = |id: Uuid| {
            let settings = settings.clone();
            let db = db.clone();
            async move {
                let args_hm = empty_args();
                let args = PushArgs::from(&args_hm);
                let mut scheduled_for = None;
                let mut tx = db.begin().await.unwrap();
                windmill_queue::jobs_ee::maybe_debounce(
                    &settings,
                    &mut scheduled_for,
                    &Some("f/test/script".to_string()),
                    "test-workspace",
                    JobKind::Noop,
                    id,
                    &args,
                    &mut tx,
                )
                .await
                .unwrap();
                tx.commit().await.unwrap();
            }
        };

        push(j1).await;
        push(j2).await;
        assert!(is_completed(&db, &j1).await, "J1 debounced by J2");

        // J2 starts running.
        sqlx::query!(
            "UPDATE v2_job_queue SET running = true, started_at = now() WHERE id = $1",
            j2
        )
        .execute(&db)
        .await?;

        // Late arrival J3.
        let j3 = Uuid::new_v4();
        insert_noop_job(&db, j3, "test-workspace").await;
        push(j3).await;

        // Running survivor J2 is preserved; J3 takes over a fresh window.
        assert!(is_queued(&db, &j2).await, "running J2 stays queued");
        assert!(!is_completed(&db, &j2).await, "running J2 not completed");
        let (dk_job, dk_prev, dk_times) = get_debounce_key(&db, key)
            .await
            .expect("debounce key exists");
        assert_eq!(dk_job, j3);
        assert!(dk_prev.is_none(), "fresh window: no previous job");
        assert_eq!(dk_times, 0, "fresh window resets debounced_times");

        Ok(())
    }

    /// Once a survivor has fully been pulled (batch + key consumed by
    /// maybe_apply_debouncing) and is running, a later arrival naturally starts a new
    /// window. This locks in that the committed-running case stays correct alongside
    /// the in-flight-running guard.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_committed_running_survivor_independent(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "committed_running_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        let j2_args = push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;

        // J2 is pulled: accumulate its batch and consume key + batch.
        let mut j2_res = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &j2_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        j2_res.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&j2_res, &[1, 2], "items");
        assert!(
            get_debounce_key(&db, key).await.is_none(),
            "key consumed when survivor pulled"
        );

        // J2 now running.
        sqlx::query!(
            "UPDATE v2_job_queue SET running = true, started_at = now() WHERE id = $1",
            j2
        )
        .execute(&db)
        .await?;

        // Late arrival J3: fresh window, independent batch, J2 untouched.
        let j3 = Uuid::new_v4();
        let j3_args = push_debounced_script(&db, j3, vec![3], &settings, rs_handle).await;
        assert!(is_queued(&db, &j2).await, "running J2 untouched");
        assert!(!is_completed(&db, &j2).await, "running J2 not completed");
        let (dk_job, _, dk_times) = get_debounce_key(&db, key).await.expect("key exists");
        assert_eq!(dk_job, j3);
        assert_eq!(dk_times, 0);

        let mut j3_res = make_pulled_job_result(
            j3,
            "test-workspace",
            "f/test/script",
            &j3_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        j3_res.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&j3_res, &[3], "items");

        Ok(())
    }

    /// Flow post-preprocessing debounce must apply the same running-survivor guard:
    /// a running flow survivor must not be completed/skipped by a late flow arrival.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_running_survivor_not_superseded(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "pp_running_survivor_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        let flow1 = Uuid::new_v4();
        let flow2 = Uuid::new_v4();
        insert_flow_job(&db, flow1, "test-workspace", "f/test/flow").await;
        insert_flow_job(&db, flow2, "test-workspace", "f/test/flow").await;

        let pp = |id: Uuid| {
            let settings = settings.clone();
            let db = db.clone();
            let args_hm = args_hm.clone();
            async move {
                let args = PushArgs::from(&args_hm);
                windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                    &settings,
                    &Some("f/test/flow".to_string()),
                    "test-workspace",
                    id,
                    &args,
                    &db,
                )
                .await
                .unwrap()
            }
        };

        pp(flow1).await;
        pp(flow2).await;
        assert!(is_completed(&db, &flow1).await, "flow1 debounced by flow2");

        // flow2 (the survivor) starts running.
        sqlx::query!(
            "UPDATE v2_job_queue SET running = true, started_at = now() WHERE id = $1",
            flow2
        )
        .execute(&db)
        .await?;

        // Late flow3 arrival while flow2 is running.
        let flow3 = Uuid::new_v4();
        insert_flow_job(&db, flow3, "test-workspace", "f/test/flow").await;
        let sched = pp(flow3).await;
        assert!(sched.is_some(), "flow3 should be debounced (fresh window)");

        // Running flow2 must be preserved; flow3 owns a fresh window.
        assert!(is_queued(&db, &flow2).await, "running flow2 stays queued");
        assert!(
            !is_completed(&db, &flow2).await,
            "running flow2 must not be completed"
        );
        let (dk_job, dk_prev, dk_times) = get_debounce_key(&db, key).await.expect("key exists");
        assert_eq!(dk_job, flow3, "flow3 holds the key");
        assert!(dk_prev.is_none(), "fresh window: no previous job");
        assert_eq!(dk_times, 0, "fresh window resets debounced_times");

        Ok(())
    }

    /// A running survivor resets the debounce window for the late arrival, so an
    /// inherited high `debounced_times` cannot push the new arrival over
    /// max_total_debounces_amount and force an immediate (un-debounced) run.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_running_survivor_resets_limit_window(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "running_limit_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            // Limit of 3: J1, J2 stay debounced; a third arrival in the SAME window
            // would trip the limit (current_amount + 1 >= 3) and fire immediately.
            max_total_debounces_amount: Some(3),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // Build up the window close to the limit: J1, J2 (debounced_times = 1 on J2).
        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;
        let (_, _, times_before) = get_debounce_key(&db, key).await.expect("key exists");
        assert_eq!(times_before, 1);

        // J2 starts running.
        sqlx::query!(
            "UPDATE v2_job_queue SET running = true, started_at = now() WHERE id = $1",
            j2
        )
        .execute(&db)
        .await?;

        // J3 arrives. Without the reset it would inherit debounced_times and could trip
        // the max-count limit and fire immediately, killing running J2. With the guard
        // it starts a fresh window (debounced_times = 0) and is debounced normally.
        let j3 = Uuid::new_v4();
        let mut scheduled_for = None;
        {
            let args_val = serde_json::json!({ "items": [3] });
            insert_script_job_with_args(&db, j3, "test-workspace", "f/test/script", &args_val)
                .await;
            sqlx::query!(
                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                rs_handle,
                j3,
            )
            .execute(&db)
            .await?;
            let args_hm: HashMap<String, Box<RawValue>> = serde_json::from_value(args_val).unwrap();
            let push_args = PushArgs::from(&args_hm);
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Script,
                j3,
                &push_args,
                &mut tx,
            )
            .await?;
            tx.commit().await?;
        }

        // J3 is debounced (scheduled_for set, not fired immediately) and J2 survives.
        assert!(
            scheduled_for.is_some(),
            "J3 should be debounced, not fired immediately"
        );
        assert!(is_queued(&db, &j2).await, "running J2 stays queued");
        assert!(!is_completed(&db, &j2).await, "running J2 not completed");
        let (dk_job, dk_prev, dk_times) = get_debounce_key(&db, key).await.expect("key exists");
        assert_eq!(dk_job, j3);
        assert!(dk_prev.is_none());
        assert_eq!(dk_times, 0, "fresh window resets the limit counter");

        Ok(())
    }

    /// Concurrency regression: two late arrivals racing AFTER the survivor started
    /// running must not both spawn independent windows. The running check reads the
    /// post-conflict-lock holder (`debounce_key.job_id`), so the row lock serializes the
    /// two upserts: the first observes the running survivor and opens a fresh window;
    /// the second observes that fresh-window head (queued, not running) and debounces
    /// into it. Exactly one late arrival survives, the other is debounced — never both.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_concurrent_arrivals_after_running_survivor(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "concurrent_running_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // J2 is the survivor and starts running.
        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;
        sqlx::query!(
            "UPDATE v2_job_queue SET running = true, started_at = now() WHERE id = $1",
            j2
        )
        .execute(&db)
        .await?;

        // Insert the two late arrivals up front, then race only their maybe_debounce calls.
        let j3 = Uuid::new_v4();
        let j4 = Uuid::new_v4();
        for (id, items) in [(j3, 3i64), (j4, 4i64)] {
            let args_val = serde_json::json!({ "items": [items] });
            insert_script_job_with_args(&db, id, "test-workspace", "f/test/script", &args_val)
                .await;
            sqlx::query!(
                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                rs_handle,
                id,
            )
            .execute(&db)
            .await?;
        }

        let race = |id: Uuid, items: i64| {
            let db = db.clone();
            let settings = settings.clone();
            async move {
                let args_hm: HashMap<String, Box<RawValue>> =
                    serde_json::from_value(serde_json::json!({ "items": [items] })).unwrap();
                let push_args = PushArgs::from(&args_hm);
                let mut scheduled_for = None;
                let mut tx = db.begin().await.unwrap();
                windmill_queue::jobs_ee::maybe_debounce(
                    &settings,
                    &mut scheduled_for,
                    &Some("f/test/script".to_string()),
                    "test-workspace",
                    JobKind::Script,
                    id,
                    &push_args,
                    &mut tx,
                )
                .await
                .unwrap();
                tx.commit().await.unwrap();
            }
        };
        tokio::join!(race(j3, 3), race(j4, 4));

        // The running survivor is untouched.
        assert!(is_queued(&db, &j2).await, "running J2 stays queued");
        assert!(!is_completed(&db, &j2).await, "running J2 not completed");

        // Exactly one late arrival survives; the other is debounced into the same window.
        let j3q = is_queued(&db, &j3).await;
        let j4q = is_queued(&db, &j4).await;
        let j3c = is_completed(&db, &j3).await;
        let j4c = is_completed(&db, &j4).await;
        assert!(
            (j3q && j4c && !j4q && !j3c) || (j4q && j3c && !j3q && !j4c),
            "exactly one late arrival must survive and the other be debounced \
             (not two independent windows); got j3 queued={j3q} completed={j3c}, \
             j4 queued={j4q} completed={j4c}"
        );

        // The surviving holder chained the debounced arrival into one window.
        let (holder, prev, times) = get_debounce_key(&db, key).await.expect("key exists");
        let (survivor, debounced) = if j3q { (j3, j4) } else { (j4, j3) };
        assert_eq!(holder, survivor, "key points to the surviving late arrival");
        assert_eq!(
            prev,
            Some(debounced),
            "the surviving window debounced the other late arrival"
        );
        assert_eq!(times, 1, "single fresh window with one debounce");

        // Both late arrivals must share a batch: when the survivor is pulled, its
        // accumulation must include the debounced arrival's items, not just its own.
        let survivor_args = serde_json::json!({ "items": [if j3q { 3 } else { 4 }] });
        let mut survivor_res = make_pulled_job_result(
            survivor,
            "test-workspace",
            "f/test/script",
            &survivor_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        survivor_res.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&survivor_res, &[3, 4], "items");

        Ok(())
    }

    /// Regression: a push that would chain onto a queued holder must not error if the
    /// worker pull path concurrently deletes that holder's debounce_key
    /// (`DELETE ... WHERE job_id = ...`, which does NOT take the push advisory lock).
    /// The upsert is a single atomic `INSERT ... ON CONFLICT`, so a deleted holder simply
    /// yields a fresh window rather than a "no row updated" failure. Races the two and
    /// asserts the push always succeeds and leaves a consistent key.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_push_races_key_deletion_by_pull(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "push_vs_pull_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // 50 rounds to give the interleaving a chance to land in the read/write window
        // that the old split read+UPDATE path would have failed on.
        for round in 0..50 {
            sqlx::query!("DELETE FROM debounce_key WHERE key = $1", key)
                .execute(&db)
                .await?;
            let holder = Uuid::new_v4();
            push_debounced_script(&db, holder, vec![round], &settings, rs_handle).await;

            let late = Uuid::new_v4();
            let late_args = serde_json::json!({ "items": [round * 1000] });
            insert_script_job_with_args(&db, late, "test-workspace", "f/test/script", &late_args)
                .await;
            sqlx::query!(
                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                rs_handle,
                late,
            )
            .execute(&db)
            .await?;

            // Race: push the late arrival (chains onto `holder`) against the worker pull
            // cleanup deleting `holder`'s key.
            let push = {
                let db = db.clone();
                let settings = settings.clone();
                async move {
                    let args_hm: HashMap<String, Box<RawValue>> =
                        serde_json::from_value(serde_json::json!({ "items": [round * 1000] }))
                            .unwrap();
                    let push_args = PushArgs::from(&args_hm);
                    let mut scheduled_for = None;
                    let mut tx = db.begin().await.unwrap();
                    let res = windmill_queue::jobs_ee::maybe_debounce(
                        &settings,
                        &mut scheduled_for,
                        &Some("f/test/script".to_string()),
                        "test-workspace",
                        JobKind::Script,
                        late,
                        &push_args,
                        &mut tx,
                    )
                    .await;
                    if res.is_ok() {
                        tx.commit().await.unwrap();
                    }
                    res
                }
            };
            let delete = {
                let db = db.clone();
                async move {
                    sqlx::query!("DELETE FROM debounce_key WHERE job_id = $1", holder)
                        .execute(&db)
                        .await
                }
            };
            let (push_res, _) = tokio::join!(push, delete);
            assert!(
                push_res.is_ok(),
                "round {round}: push must not error when the holder key is concurrently deleted: {push_res:?}"
            );
        }

        Ok(())
    }

    /// Claim-based exactly-once: if two survivors end up on the same batch (only
    /// possible in a narrow push/pull race), the args of each member are accumulated
    /// into exactly ONE run. The survivor that claims the batch first accumulates
    /// everyone; the second survivor finds its contribution already consumed and runs
    /// empty — no item is dropped and none is processed twice.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_batch_consumed_exactly_once(db: Pool<Postgres>) -> anyhow::Result<()> {
        let key = "exactly_once_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // J1 superseded, J2 the (first) survivor of batch B = {J1, J2}.
        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        let j2_args = push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;

        // Simulate the race outcome: a second survivor J3 ended up on the SAME batch B.
        let j3 = Uuid::new_v4();
        let j3_args = serde_json::json!({ "items": [3] });
        insert_script_job_with_args(&db, j3, "test-workspace", "f/test/script", &j3_args).await;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            j3,
        )
        .execute(&db)
        .await?;
        sqlx::query!(
            "INSERT INTO v2_job_debounce_batch (id, debounce_batch)
             SELECT $1, debounce_batch FROM v2_job_debounce_batch WHERE id = $2",
            j3,
            j2,
        )
        .execute(&db)
        .await?;

        // J2 pulled first: claims the whole batch, accumulates everyone's items.
        let mut j2_res = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &j2_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        j2_res.maybe_apply_debouncing(&db).await?;
        assert!(j2_res.job.is_some(), "J2 runs");
        assert_accumulated_items(&j2_res, &[1, 2, 3], "items");

        // J3 pulled next: its contribution was already consumed by J2 -> runs empty,
        // so [3] is not processed a second time.
        let mut j3_res = make_pulled_job_result(
            j3,
            "test-workspace",
            "f/test/script",
            &j3_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        j3_res.maybe_apply_debouncing(&db).await?;
        let job = j3_res.job.as_ref().expect("J3 still runs (empty)");
        let items: Vec<serde_json::Value> =
            serde_json::from_str(job.job.args.as_ref().unwrap().get("items").unwrap().get())?;
        assert!(
            items.is_empty(),
            "J3's items must be empty (already consumed by J2), got {items:?}"
        );

        Ok(())
    }

    /// A survivor re-pulled (e.g. crash recovery) must NOT mistake its own earlier
    /// claim for a sibling's and wipe its accumulated args. consumed_by = self is
    /// distinguished from consumed_by = another job.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_repull_keeps_accumulated(db: Pool<Postgres>) -> anyhow::Result<()> {
        let key = "repull_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        let j2_args = push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;

        // First pull: J2 claims its batch and accumulates [1, 2].
        let mut first = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &j2_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        first.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&first, &[1, 2], "items");

        // Re-pull with the args persisted by the first pull: J2 sees its OWN prior claim
        // (consumed_by = j2), so it keeps the accumulated args rather than running empty.
        let persisted = first.job.as_ref().unwrap().job.args.as_ref().unwrap();
        let persisted_json = serde_json::to_value(persisted).unwrap();
        let mut second = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &persisted_json,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        second.maybe_apply_debouncing(&db).await?;
        assert!(second.job.is_some(), "re-pulled J2 still runs");
        assert_accumulated_items(&second, &[1, 2], "items");

        Ok(())
    }

    /// Helper: insert a script job and put it on the SAME debounce batch as `of_job`
    /// (simulating a chained survivor). Returns its args JSON.
    async fn add_survivor_to_batch_of(
        db: &Pool<Postgres>,
        id: Uuid,
        items: Vec<i64>,
        of_job: Uuid,
        rs_handle: Option<i64>,
    ) -> serde_json::Value {
        let args = serde_json::json!({ "items": items });
        insert_script_job_with_args(db, id, "test-workspace", "f/test/script", &args).await;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            id,
        )
        .execute(db)
        .await
        .unwrap();
        let inserted = sqlx::query!(
            "INSERT INTO v2_job_debounce_batch (id, debounce_batch)
             SELECT $1, debounce_batch FROM v2_job_debounce_batch WHERE id = $2",
            id,
            of_job,
        )
        .execute(db)
        .await
        .unwrap();
        // `of_job` must already have a batch row, else this no-ops and the test would
        // pass vacuously (the job would end up never-batched, keeping its own args).
        assert_eq!(
            inserted.rows_affected(),
            1,
            "add_survivor_to_batch_of: {of_job} has no batch row to share"
        );
        args
    }

    /// Helper: read the accumulated `items` of a pulled job as a sorted Vec<i64>.
    fn items_of(result: &windmill_queue::PulledJobResult) -> Vec<i64> {
        let job = result.job.as_ref().expect("job present");
        let raw = job.job.args.as_ref().unwrap().get("items").unwrap();
        let mut v: Vec<i64> = serde_json::from_str::<Vec<serde_json::Value>>(raw.get())
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap())
            .collect();
        v.sort();
        v
    }

    /// Edge: an accumulate-debounced job that was NEVER batched (CE / workers behind v2:
    /// no v2_job_debounce_batch row) must keep its own args, not be emptied.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_never_batched_keeps_own_args(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("never_batched_key".to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // Insert a job with the debounce handle but DO NOT push through maybe_debounce,
        // so it has no batch row at all.
        let j = Uuid::new_v4();
        let args = serde_json::json!({ "items": [7, 8] });
        insert_script_job_with_args(&db, j, "test-workspace", "f/test/script", &args).await;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            j,
        )
        .execute(&db)
        .await?;

        let mut res = make_pulled_job_result(
            j,
            "test-workspace",
            "f/test/script",
            &args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        res.maybe_apply_debouncing(&db).await?;
        assert!(res.job.is_some(), "never-batched job still runs");
        assert_accumulated_items(&res, &[7, 8], "items");
        Ok(())
    }

    /// Edge: two survivors of one batch pulled CONCURRENTLY. The atomic claim must
    /// partition the batch disjointly — the union of what they each accumulate is the
    /// full set, with NO item processed by both.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_concurrent_claim_disjoint(db: Pool<Postgres>) -> anyhow::Result<()> {
        let key = "concurrent_claim_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await; // superseded
        let j2_args = push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await; // survivor 1
        let j3 = Uuid::new_v4();
        let j3_args = add_survivor_to_batch_of(&db, j3, vec![3], j2, rs_handle).await; // survivor 2

        let pull = |id: Uuid, args: serde_json::Value| {
            let db = db.clone();
            async move {
                let mut res = make_pulled_job_result(
                    id,
                    "test-workspace",
                    "f/test/script",
                    &args,
                    JobKind::Script,
                    "deno",
                    rs_handle,
                );
                res.maybe_apply_debouncing(&db).await.unwrap();
                res
            }
        };
        let (r2, r3) = tokio::join!(pull(j2, j2_args), pull(j3, j3_args));

        let mut union = items_of(&r2);
        union.extend(items_of(&r3));
        union.sort();
        assert_eq!(
            union,
            vec![1, 2, 3],
            "every item accumulated exactly once across the two concurrent survivors"
        );
        // disjoint: no overlap between the two survivors' items
        let i2 = items_of(&r2);
        let i3 = items_of(&r3);
        assert!(
            !i2.iter().any(|x| i3.contains(x)),
            "no item processed by both survivors; got j2={i2:?} j3={i3:?}"
        );
        Ok(())
    }

    /// Edge: three survivors on one batch pulled in sequence. The first claims the whole
    /// batch; the rest find themselves consumed and run empty.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_three_survivors_first_takes_all(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "three_survivors_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        let j2_args = push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;
        let j3 = Uuid::new_v4();
        let j3_args = add_survivor_to_batch_of(&db, j3, vec![3], j2, rs_handle).await;
        let j4 = Uuid::new_v4();
        let j4_args = add_survivor_to_batch_of(&db, j4, vec![4], j2, rs_handle).await;

        let mk = |id, args: &serde_json::Value| {
            make_pulled_job_result(
                id,
                "test-workspace",
                "f/test/script",
                args,
                JobKind::Script,
                "deno",
                rs_handle,
            )
        };
        let (mut r2, mut r3, mut r4) = (mk(j2, &j2_args), mk(j3, &j3_args), mk(j4, &j4_args));
        r2.maybe_apply_debouncing(&db).await?;
        r3.maybe_apply_debouncing(&db).await?;
        r4.maybe_apply_debouncing(&db).await?;

        assert_eq!(
            items_of(&r2),
            vec![1, 2, 3, 4],
            "first survivor takes the whole batch"
        );
        assert!(items_of(&r3).is_empty(), "second survivor runs empty");
        assert!(items_of(&r4).is_empty(), "third survivor runs empty");
        Ok(())
    }

    /// Edge: plain debounce (no accumulate args) must HARD-DELETE its batch rows on pull
    /// (not leave consumed rows lingering), so the non-accumulate path doesn't leak.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_non_accumulate_deletes_batch(db: Pool<Postgres>) -> anyhow::Result<()> {
        let key = "non_accum_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            // no debounce_args_to_accumulate
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        // push_debounced_script sends {items:[...]} but with no accumulate arg configured,
        // the batch is created yet never accumulated.
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        let j2_args = push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;

        let batch_rows_before: i64 =
            sqlx::query_scalar!("SELECT count(*) as \"c!\" FROM v2_job_debounce_batch")
                .fetch_one(&db)
                .await?;
        assert!(batch_rows_before >= 2, "batch rows exist before pull");

        let mut res = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &j2_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        res.maybe_apply_debouncing(&db).await?;

        let remaining: i64 =
            sqlx::query_scalar!("SELECT count(*) as \"c!\" FROM v2_job_debounce_batch")
                .fetch_one(&db)
                .await?;
        assert_eq!(
            remaining, 0,
            "non-accumulate pull hard-deletes the batch rows"
        );
        Ok(())
    }

    /// Edge: GC sweep deletes consumed rows past the grace period but keeps recently
    /// consumed and not-yet-consumed rows.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_gc_consumed_batches(db: Pool<Postgres>) -> anyhow::Result<()> {
        let old = Uuid::new_v4();
        let recent = Uuid::new_v4();
        let unconsumed = Uuid::new_v4();
        // A consumed-long-ago sibling that is STILL QUEUED (e.g. stuck behind a
        // concurrency limit): its marker must survive GC so its eventual pull still sees
        // "already consumed" and runs empty (no duplicate).
        let queued_old = Uuid::new_v4();
        insert_script_job_with_args(
            &db,
            queued_old,
            "test-workspace",
            "f/test/script",
            &serde_json::json!({ "items": [9] }),
        )
        .await;
        sqlx::query!(
            "INSERT INTO v2_job_debounce_batch (id, debounce_batch, consumed_at) VALUES
                ($1, nextval('debounce_batch_seq'), now() - interval '20 minutes'),
                ($2, nextval('debounce_batch_seq'), now() - interval '1 minute'),
                ($3, nextval('debounce_batch_seq'), NULL),
                ($4, nextval('debounce_batch_seq'), now() - interval '20 minutes')",
            old,
            recent,
            unconsumed,
            queued_old,
        )
        .execute(&db)
        .await?;

        // Mirror the monitor GC sweep (age floor + only-if-no-longer-queued).
        let deleted = sqlx::query_scalar!(
            "WITH del AS (
                DELETE FROM v2_job_debounce_batch
                WHERE consumed_at IS NOT NULL
                  AND consumed_at < now() - interval '10 minutes'
                  AND id NOT IN (SELECT id FROM v2_job_queue)
                RETURNING 1
            ) SELECT count(*) as \"c!\" FROM del"
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            deleted, 1,
            "only the old, no-longer-queued consumed row is GC'd"
        );

        let exists = |id: Uuid, db: Pool<Postgres>| async move {
            sqlx::query_scalar!(
                "SELECT EXISTS(SELECT 1 FROM v2_job_debounce_batch WHERE id = $1) as \"e!\"",
                id
            )
            .fetch_one(&db)
            .await
            .unwrap()
        };
        assert!(!exists(old, db.clone()).await, "old consumed row gone");
        assert!(
            exists(recent, db.clone()).await,
            "recently consumed row kept"
        );
        assert!(exists(unconsumed, db.clone()).await, "unconsumed row kept");
        assert!(
            exists(queued_old, db.clone()).await,
            "old consumed row whose job is still queued must be kept"
        );
        Ok(())
    }

    /// Helper: mark a queued job as running (simulates a survivor that the
    /// concurrency limiter has just started executing).
    async fn set_running(db: &Pool<Postgres>, job_id: &Uuid) {
        sqlx::query!(
            "UPDATE v2_job_queue SET running = true WHERE id = $1",
            job_id
        )
        .execute(db)
        .await
        .expect("set running");
    }

    /// Regression (ref #9781): post-preprocessing debounce with
    /// `debounce_args_to_accumulate` under a concurrency limit. A survivor accumulates
    /// its own element and starts running; a later same-key message must start a NEW
    /// batch (survive) rather than be folded into the running survivor and silently
    /// dropped. Exercises the full EE path via `jobs_ee::maybe_debounce_post_preprocessing`.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_post_preprocessing_debounce_into_running_survivor_loses_message(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("ported_running_survivor_key".to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // --- Wave 1: a single message becomes the survivor and starts running. ---
        let survivor = Uuid::new_v4();
        let survivor_args = serde_json::json!({ "items": [1] });
        insert_flow_job_with_preprocessor(
            &db,
            survivor,
            "test-workspace",
            "f/test/flow_run",
            true,
            0,
        )
        .await;
        sqlx::query!(
            "UPDATE v2_job SET args = $2 WHERE id = $1",
            survivor,
            survivor_args
        )
        .execute(&db)
        .await?;

        let survivor_args_hm: HashMap<String, Box<RawValue>> =
            serde_json::from_value(survivor_args.clone()).unwrap();
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow_run".to_string()),
            "test-workspace",
            survivor,
            &PushArgs::from(&survivor_args_hm),
            &db,
        )
        .await?;
        assert!(is_queued(&db, &survivor).await, "survivor should be queued");

        // Worker pulls the survivor: accumulate its own [1], consume the batch, run.
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            survivor,
        )
        .execute(&db)
        .await?;
        let mut pulled = make_pulled_job_result(
            survivor,
            "test-workspace",
            "f/test/flow_run",
            &survivor_args,
            JobKind::Flow,
            "flow",
            rs_handle,
        );
        pulled.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled, &[1], "items");
        set_running(&db, &survivor).await; // survivor is now RUNNING

        // --- Wave 2: a new message arrives while the survivor is running. ---
        let late = Uuid::new_v4();
        let late_args = serde_json::json!({ "items": [2] });
        insert_flow_job_with_preprocessor(&db, late, "test-workspace", "f/test/flow_run", true, 0)
            .await;
        sqlx::query!("UPDATE v2_job SET args = $2 WHERE id = $1", late, late_args)
            .execute(&db)
            .await?;

        let late_args_hm: HashMap<String, Box<RawValue>> =
            serde_json::from_value(late_args.clone()).unwrap();
        windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
            &settings,
            &Some("f/test/flow_run".to_string()),
            "test-workspace",
            late,
            &PushArgs::from(&late_args_hm),
            &db,
        )
        .await?;

        // The late message must survive (new batch): still queued, and the debounce_key
        // moved off the already-running survivor.
        let late_survived = is_queued(&db, &late).await && !is_completed(&db, &late).await;
        let dk = get_debounce_key(&db, "ported_running_survivor_key").await;
        let key_moved_off_running_survivor =
            dk.map(|(job_id, _, _)| job_id != survivor).unwrap_or(true);
        assert!(
            late_survived && key_moved_off_running_survivor,
            "message arriving while the survivor is running must start a new batch \
             (survive), not be folded into the running survivor and dropped. \
             late_survived={late_survived}, key_moved_off={key_moved_off_running_survivor}"
        );
        Ok(())
    }

    /// Flow-node debounce (third EE entry point, `jobs_ee::maybe_debounce_flow_node`):
    /// a running survivor child must not be superseded by a later same-key child; the
    /// late child starts a fresh window and the running child is left to finish.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_flow_node_debounce_running_survivor_not_superseded(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "flow_node_running_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        let flow1 = Uuid::new_v4();
        let child1 = Uuid::new_v4();
        insert_flow_job(&db, flow1, "test-workspace", "f/test/my_flow").await;
        insert_child_job_with_parent(&db, child1, flow1, "test-workspace").await;

        // child1 becomes the survivor, then starts running.
        {
            let args = PushArgs::from(&args_hm);
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce_flow_node(
                &settings,
                child1,
                flow1,
                "f/test/my_flow",
                "step_a",
                "test-workspace",
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }
        set_running(&db, &child1).await;

        // child2 (later same-key child) arrives while child1 is running.
        let flow2 = Uuid::new_v4();
        let child2 = Uuid::new_v4();
        insert_flow_job(&db, flow2, "test-workspace", "f/test/my_flow").await;
        insert_child_job_with_parent(&db, child2, flow2, "test-workspace").await;
        {
            let args = PushArgs::from(&args_hm);
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce_flow_node(
                &settings,
                child2,
                flow2,
                "f/test/my_flow",
                "step_a",
                "test-workspace",
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // The running child1 (and its parent flow1) must be left alone; child2 owns a
        // fresh window.
        assert!(is_queued(&db, &child1).await, "running child1 stays queued");
        assert!(
            !is_completed(&db, &child1).await,
            "running child1 not completed"
        );
        assert!(
            !is_completed(&db, &flow1).await,
            "flow1 of running child not completed"
        );
        let (dk_job, dk_prev, dk_times) = get_debounce_key(&db, key).await.expect("key exists");
        assert_eq!(dk_job, child2, "child2 holds the key");
        assert!(dk_prev.is_none(), "fresh window: no previous child");
        assert_eq!(dk_times, 0, "fresh window resets debounced_times");
        Ok(())
    }

    /// Edge: accumulate values that are bare scalars (not arrays) — the `T | T[]` union
    /// case. Each scalar contribution must be wrapped into a single-element list so the
    /// survivor accumulates them all. Exercises the non-array fallback in the claim path.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_accumulate_scalar_values(db: Pool<Postgres>) -> anyhow::Result<()> {
        let key = "scalar_accum_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // Push two jobs whose `items` is a BARE SCALAR, not an array.
        let push_scalar = |id: Uuid, v: i64, db: Pool<Postgres>, settings: DebouncingSettings| async move {
            let args_val = serde_json::json!({ "items": v });
            insert_script_job_with_args(&db, id, "test-workspace", "f/test/script", &args_val)
                .await;
            sqlx::query!(
                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                rs_handle,
                id,
            )
            .execute(&db)
            .await
            .unwrap();
            let hm: HashMap<String, Box<RawValue>> = serde_json::from_value(args_val).unwrap();
            let mut sf = None;
            let mut tx = db.begin().await.unwrap();
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut sf,
                &Some("f/test/script".to_string()),
                "test-workspace",
                JobKind::Script,
                id,
                &PushArgs::from(&hm),
                &mut tx,
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        };
        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_scalar(j1, 1, db.clone(), settings.clone()).await;
        push_scalar(j2, 2, db.clone(), settings.clone()).await;

        let mut res = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &serde_json::json!({ "items": 2 }),
            JobKind::Script,
            "deno",
            rs_handle,
        );
        res.maybe_apply_debouncing(&db).await?;
        // Both bare scalars are wrapped and accumulated into a list.
        assert_accumulated_items(&res, &[1, 2], "items");
        Ok(())
    }

    /// Edge: GC reclaiming a survivor's consumed row before a re-pull must NOT lose data —
    /// the re-pull finds no row (had_row=false) and keeps its already-persisted accumulated
    /// args, rather than running empty.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_debounce_repull_after_gc_keeps_accumulated(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let key = "repull_gc_key";
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some(key.to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let j1 = Uuid::new_v4();
        let j2 = Uuid::new_v4();
        push_debounced_script(&db, j1, vec![1], &settings, rs_handle).await;
        let j2_args = push_debounced_script(&db, j2, vec![2], &settings, rs_handle).await;

        let mut first = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &j2_args,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        first.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&first, &[1, 2], "items");

        // Simulate the GC sweep reclaiming the (now consumed) batch rows for this batch.
        sqlx::query!(
            "DELETE FROM v2_job_debounce_batch WHERE debounce_batch = (
                SELECT debounce_batch FROM v2_job_debounce_batch WHERE id = $1
            )",
            j2,
        )
        .execute(&db)
        .await
        .ok();
        // (and any that were already consumed elsewhere)
        sqlx::query!("DELETE FROM v2_job_debounce_batch WHERE consumed_at IS NOT NULL")
            .execute(&db)
            .await?;

        // Re-pull with the args persisted on the first pull: no batch row now, so it must
        // fall back to its own (already-accumulated) args — no loss.
        let persisted =
            serde_json::to_value(first.job.as_ref().unwrap().job.args.as_ref().unwrap()).unwrap();
        let mut second = make_pulled_job_result(
            j2,
            "test-workspace",
            "f/test/script",
            &persisted,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        second.maybe_apply_debouncing(&db).await?;
        assert!(second.job.is_some(), "re-pulled survivor still runs");
        assert_accumulated_items(&second, &[1, 2], "items");
        Ok(())
    }

    /// Throughput benchmark for the FULL debounce path (EE push +
    /// `jobs_ee::maybe_debounce`/`complete_debounced_job`/`upsert_debounce_key`, then OSS
    /// `maybe_apply_debouncing` claim/accumulate/consume). #[ignore]d — run manually:
    ///   cargo test -p windmill-queue --test debounce_test --features private,enterprise \
    ///     bench_debounce_full_path -- --ignored --nocapture --test-threads=1
    /// Each cycle = a burst of BURST pushes to one key (debounced) + one survivor pull
    /// (accumulate+consume), run across CONCURRENCY tasks. Compare before/after by running
    /// it on each code revision.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    #[ignore]
    async fn bench_debounce_full_path(db: Pool<Postgres>) -> anyhow::Result<()> {
        const CONCURRENCY: usize = 4;
        const CYCLES_PER_TASK: usize = 300;
        const BURST: usize = 3;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("bench_key".to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let start = std::time::Instant::now();
        let tasks: Vec<_> = (0..CONCURRENCY)
            .map(|t| {
                let db = db.clone();
                let settings = DebouncingSettings {
                    // distinct key space per task so bursts collapse independently
                    debounce_key: Some(format!("bench_key_{t}")),
                    ..settings.clone()
                };
                tokio::spawn(async move {
                    for c in 0..CYCLES_PER_TASK {
                        // Fresh key per cycle so each cycle is one full collapse+pull.
                        let key = format!("bench_{t}_{c}");
                        let settings = DebouncingSettings {
                            debounce_key: Some(key.clone()),
                            ..settings.clone()
                        };
                        let mut survivor = Uuid::new_v4();
                        for b in 0..BURST {
                            let id = Uuid::new_v4();
                            survivor = id;
                            let args_val = serde_json::json!({ "items": [b as i64] });
                            insert_script_job_with_args(
                                &db, id, "test-workspace", "f/test/script", &args_val,
                            )
                            .await;
                            sqlx::query!(
                                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                                rs_handle, id,
                            )
                            .execute(&db)
                            .await
                            .unwrap();
                            let hm: HashMap<String, Box<RawValue>> =
                                serde_json::from_value(args_val).unwrap();
                            let mut sf = None;
                            let mut tx = db.begin().await.unwrap();
                            windmill_queue::jobs_ee::maybe_debounce(
                                &settings, &mut sf, &Some("f/test/script".to_string()),
                                "test-workspace", JobKind::Script, id, &PushArgs::from(&hm), &mut tx,
                            )
                            .await
                            .unwrap();
                            tx.commit().await.unwrap();
                        }
                        // Survivor pull: claim + accumulate + consume.
                        let mut res = make_pulled_job_result(
                            survivor, "test-workspace", "f/test/script",
                            &serde_json::json!({ "items": [] }),
                            JobKind::Script, "deno", rs_handle,
                        );
                        res.maybe_apply_debouncing(&db).await.unwrap();
                    }
                })
            })
            .collect();
        for t in tasks {
            t.await.unwrap();
        }
        let elapsed = start.elapsed();
        let cycles = CONCURRENCY * CYCLES_PER_TASK;
        let jobs = cycles * BURST;
        eprintln!(
            "BENCH full debounce path: {cycles} cycles ({jobs} pushed jobs + {cycles} pulls) in {:.2?} | {:.0} pushes/s | {:.0} pulls/s",
            elapsed,
            jobs as f64 / elapsed.as_secs_f64(),
            cycles as f64 / elapsed.as_secs_f64(),
        );
        Ok(())
    }

    /// Test: Push-time (script) debounce with max_total_debounces_amount=2.
    /// 5 calls, each sending {x: [i]}. Expected:
    ///   Call 1: debounced (scheduled_for set)
    ///   Call 2: fires immediately (limit), accumulated x=[1,2]
    ///   Call 3: debounced (new batch)
    ///   Call 4: fires immediately (limit), accumulated x=[3,4]
    ///   Call 5: debounced (new batch), x=[5]
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_push_max_count_accumulation(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(50),
            debounce_key: Some("push_count_accum_key".to_string()),
            max_total_debounces_amount: Some(2),
            debounce_args_to_accumulate: Some(vec!["x".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let mut jobs = Vec::new();
        let mut scheduled_fors = Vec::new();

        for i in 0..5 {
            let id = Uuid::new_v4();
            let args_val = serde_json::json!({"x": [i + 1]});
            insert_script_job_with_args(&db, id, "test-workspace", "f/test/push_script", &args_val)
                .await;
            sqlx::query!(
                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                rs_handle,
                id,
            )
            .execute(&db)
            .await?;
            jobs.push((id, args_val.clone()));

            let args_hm: HashMap<String, Box<RawValue>> = serde_json::from_value(args_val).unwrap();
            let push_args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/push_script".to_string()),
                "test-workspace",
                JobKind::Script,
                id,
                &push_args,
                &mut tx,
            )
            .await?;
            tx.commit().await?;
            scheduled_fors.push(scheduled_for);
        }

        // Verify debounce behavior
        assert!(scheduled_fors[0].is_some(), "call 1 debounced");
        assert!(scheduled_fors[1].is_none(), "call 2 fires immediately");
        assert!(scheduled_fors[2].is_some(), "call 3 debounced");
        assert!(scheduled_fors[3].is_none(), "call 4 fires immediately");
        assert!(scheduled_fors[4].is_some(), "call 5 debounced");

        // Call 2: accumulate args
        let mut pulled_2 = make_pulled_job_result(
            jobs[1].0,
            "test-workspace",
            "f/test/push_script",
            &jobs[1].1,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        pulled_2.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled_2, &[1, 2], "x");

        // Call 4: accumulate args
        let mut pulled_4 = make_pulled_job_result(
            jobs[3].0,
            "test-workspace",
            "f/test/push_script",
            &jobs[3].1,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        pulled_4.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled_4, &[3, 4], "x");

        // Call 5: only its own args
        let mut pulled_5 = make_pulled_job_result(
            jobs[4].0,
            "test-workspace",
            "f/test/push_script",
            &jobs[4].1,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        pulled_5.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled_5, &[5], "x");

        Ok(())
    }

    /// Test: Push-time (script) debounce with max_total_debouncing_time=1s.
    ///   Call 1 (x=[10]): debounced
    ///   -- sleep past max time --
    ///   Call 2 (x=[20]): fires immediately (time exceeded), accumulated x=[10,20]
    ///   Call 3 (x=[30]): debounced (new batch)
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_push_max_time_accumulation(db: Pool<Postgres>) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(50),
            debounce_key: Some("push_time_accum_key".to_string()),
            max_total_debouncing_time: Some(1),
            debounce_args_to_accumulate: Some(vec!["x".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        // Call 1: debounced
        let id1 = Uuid::new_v4();
        let args1 = serde_json::json!({"x": [10]});
        insert_script_job_with_args(&db, id1, "test-workspace", "f/test/push_time", &args1).await;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            id1,
        )
        .execute(&db)
        .await?;

        let args_hm: HashMap<String, Box<RawValue>> =
            serde_json::from_value(args1.clone()).unwrap();
        let mut sf1 = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut sf1,
            &Some("f/test/push_time".to_string()),
            "test-workspace",
            JobKind::Script,
            id1,
            &PushArgs::from(&args_hm),
            &mut tx,
        )
        .await?;
        tx.commit().await?;
        assert!(sf1.is_some(), "call 1 should be debounced");

        // Wait for time to exceed
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Call 2: time exceeded, fires immediately
        let id2 = Uuid::new_v4();
        let args2 = serde_json::json!({"x": [20]});
        insert_script_job_with_args(&db, id2, "test-workspace", "f/test/push_time", &args2).await;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            id2,
        )
        .execute(&db)
        .await?;

        let args_hm2: HashMap<String, Box<RawValue>> =
            serde_json::from_value(args2.clone()).unwrap();
        let mut sf2 = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut sf2,
            &Some("f/test/push_time".to_string()),
            "test-workspace",
            JobKind::Script,
            id2,
            &PushArgs::from(&args_hm2),
            &mut tx,
        )
        .await?;
        tx.commit().await?;
        assert!(
            sf2.is_none(),
            "call 2 should fire immediately (time exceeded)"
        );

        // Verify accumulation
        let mut pulled = make_pulled_job_result(
            id2,
            "test-workspace",
            "f/test/push_time",
            &args2,
            JobKind::Script,
            "deno",
            rs_handle,
        );
        pulled.maybe_apply_debouncing(&db).await?;
        assert_accumulated_items(&pulled, &[10, 20], "x");

        // Call 3: new batch, debounced
        let id3 = Uuid::new_v4();
        let args3 = serde_json::json!({"x": [30]});
        insert_script_job_with_args(&db, id3, "test-workspace", "f/test/push_time", &args3).await;
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            id3,
        )
        .execute(&db)
        .await?;

        let args_hm3: HashMap<String, Box<RawValue>> =
            serde_json::from_value(args3.clone()).unwrap();
        let mut sf3 = None;
        let mut tx = db.begin().await?;
        windmill_queue::jobs_ee::maybe_debounce(
            &settings,
            &mut sf3,
            &Some("f/test/push_time".to_string()),
            "test-workspace",
            JobKind::Script,
            id3,
            &PushArgs::from(&args_hm3),
            &mut tx,
        )
        .await?;
        tx.commit().await?;
        assert!(sf3.is_some(), "call 3 should be debounced (new batch)");

        Ok(())
    }

    /// Test: Flow (without preprocessor) debounce accumulation via push-time maybe_debounce.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_flow_debounce_accumulation_no_preprocessor(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("flow_accum_key".to_string()),
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let jobs: Vec<(Uuid, serde_json::Value)> = vec![
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [10, 20], "tag": "a"}),
            ),
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [30], "tag": "a"}),
            ),
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [40, 50], "tag": "a"}),
            ),
        ];

        for (id, args) in &jobs {
            insert_flow_job_with_args(&db, *id, "test-workspace", "f/test/flow_no_pp", args).await;
            sqlx::query!(
                "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
                rs_handle,
                id,
            )
            .execute(&db)
            .await?;
        }

        // Push-time debounce
        for (id, args) in &jobs {
            let args_hm: HashMap<String, Box<RawValue>> =
                serde_json::from_value(args.clone()).unwrap();
            let push_args = PushArgs::from(&args_hm);
            let mut scheduled_for = None;
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce(
                &settings,
                &mut scheduled_for,
                &Some("f/test/flow_no_pp".to_string()),
                "test-workspace",
                JobKind::Flow,
                *id,
                &push_args,
                &mut tx,
            )
            .await?;
            tx.commit().await?;
        }

        let survivor_id = jobs[2].0;
        assert!(
            is_queued(&db, &survivor_id).await,
            "last job should survive"
        );
        assert!(
            is_completed(&db, &jobs[0].0).await,
            "job 0 should be debounced"
        );
        assert!(
            is_completed(&db, &jobs[1].0).await,
            "job 1 should be debounced"
        );

        let mut result = make_pulled_job_result(
            survivor_id,
            "test-workspace",
            "f/test/flow_no_pp",
            &jobs[2].1,
            JobKind::Flow,
            "flow",
            rs_handle,
        );
        result.maybe_apply_debouncing(&db).await?;

        assert_accumulated_items(&result, &[10, 20, 30, 40, 50], "items");

        // Verify accumulated args were persisted to v2_job
        let db_args: Option<serde_json::Value> =
            sqlx::query_scalar!("SELECT args FROM v2_job WHERE id = $1", survivor_id,)
                .fetch_one(&db)
                .await?;
        let db_items = db_args
            .expect("v2_job args should not be null")
            .get("items")
            .expect("persisted args should contain 'items'")
            .clone();
        let mut db_items: Vec<i64> = serde_json::from_value::<Vec<serde_json::Value>>(db_items)?
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        db_items.sort();
        assert_eq!(
            db_items,
            vec![10, 20, 30, 40, 50],
            "persisted args in v2_job should contain all accumulated items"
        );

        Ok(())
    }

    /// Test: Flow WITH preprocessor debounce accumulation via maybe_debounce_post_preprocessing.
    /// This is the bug case: after preprocessing completes, the worker must store the flow's
    /// debouncing settings in runnable_settings_handle so that maybe_apply_debouncing can find
    /// them when the surviving job is pulled.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_flow_debounce_accumulation_with_preprocessor(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None,
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };
        let rs_handle = setup_debouncing_settings(&db, &settings).await;

        let jobs: Vec<(Uuid, serde_json::Value)> = vec![
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [100, 200], "extra": "v"}),
            ),
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [300], "extra": "v"}),
            ),
            (
                Uuid::new_v4(),
                serde_json::json!({"items": [400, 500, 600], "extra": "v"}),
            ),
        ];

        // Insert flow jobs with preprocessor state (step=0, preprocessor=Success)
        for (id, args) in &jobs {
            insert_flow_job_with_preprocessor(
                &db,
                *id,
                "test-workspace",
                "f/test/flow_pp",
                true,
                0,
            )
            .await;
            sqlx::query!("UPDATE v2_job SET args = $2 WHERE id = $1", id, args)
                .execute(&db)
                .await?;
        }

        // Post-preprocessing debounce
        for (id, args) in &jobs {
            let args_hm: HashMap<String, Box<RawValue>> =
                serde_json::from_value(args.clone()).unwrap();
            let push_args = PushArgs::from(&args_hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow_pp".to_string()),
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

        // Simulate the fix: worker stores runnable_settings_handle on the surviving job
        sqlx::query!(
            "UPDATE v2_job_queue SET runnable_settings_handle = $1 WHERE id = $2",
            rs_handle,
            survivor_id,
        )
        .execute(&db)
        .await?;

        let mut result = make_pulled_job_result(
            survivor_id,
            "test-workspace",
            "f/test/flow_pp",
            &jobs[2].1,
            JobKind::Flow,
            "flow",
            rs_handle,
        );
        result.maybe_apply_debouncing(&db).await?;

        assert_accumulated_items(&result, &[100, 200, 300, 400, 500, 600], "items");

        // "extra" arg should be unchanged
        let job = result.job.as_ref().unwrap();
        let extra_raw = job.job.args.as_ref().unwrap().get("extra").unwrap();
        let extra: String = serde_json::from_str(extra_raw.get())?;
        assert_eq!(extra, "v", "non-accumulated arg should be unchanged");

        // Verify accumulated args were persisted to v2_job
        let db_args: Option<serde_json::Value> =
            sqlx::query_scalar!("SELECT args FROM v2_job WHERE id = $1", survivor_id,)
                .fetch_one(&db)
                .await?;
        let db_args = db_args.expect("v2_job args should not be null");
        let db_items = db_args
            .get("items")
            .expect("persisted args should contain 'items'")
            .clone();
        let mut db_items: Vec<i64> = serde_json::from_value::<Vec<serde_json::Value>>(db_items)?
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        db_items.sort();
        assert_eq!(
            db_items,
            vec![100, 200, 300, 400, 500, 600],
            "persisted args in v2_job should contain all accumulated items"
        );
        // "extra" should also be persisted unchanged
        let db_extra = db_args.get("extra").unwrap().as_str().unwrap();
        assert_eq!(
            db_extra, "v",
            "persisted non-accumulated arg should be unchanged"
        );

        Ok(())
    }

    /// Test: Flow WITH preprocessor but WITHOUT the runnable_settings_handle fix.
    /// Proves the bug: when runnable_settings_handle is NULL, accumulation does nothing.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_flow_debounce_accumulation_with_preprocessor_no_fix(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None,
            debounce_args_to_accumulate: Some(vec!["items".to_string()]),
            ..Default::default()
        };

        let jobs: Vec<(Uuid, serde_json::Value)> = vec![
            (Uuid::new_v4(), serde_json::json!({"items": [1, 2]})),
            (Uuid::new_v4(), serde_json::json!({"items": [3]})),
            (Uuid::new_v4(), serde_json::json!({"items": [4, 5]})),
        ];

        for (id, args) in &jobs {
            insert_flow_job_with_preprocessor(
                &db,
                *id,
                "test-workspace",
                "f/test/flow_pp_nofix",
                true,
                0,
            )
            .await;
            sqlx::query!("UPDATE v2_job SET args = $2 WHERE id = $1", id, args)
                .execute(&db)
                .await?;
        }

        for (id, args) in &jobs {
            let args_hm: HashMap<String, Box<RawValue>> =
                serde_json::from_value(args.clone()).unwrap();
            let push_args = PushArgs::from(&args_hm);
            windmill_queue::jobs_ee::maybe_debounce_post_preprocessing(
                &settings,
                &Some("f/test/flow_pp_nofix".to_string()),
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

        // DO NOT set runnable_settings_handle — simulating the bug (no fix applied)
        let mut result = make_pulled_job_result(
            survivor_id,
            "test-workspace",
            "f/test/flow_pp_nofix",
            &jobs[2].1,
            JobKind::Flow,
            "flow",
            None, // No runnable_settings_handle — this is the bug
        );
        result.maybe_apply_debouncing(&db).await?;

        // Without the fix, only the survivor's own items are present (no accumulation)
        let job = result.job.as_ref().expect("job should still exist");
        let args = job.job.args.as_ref().expect("args should be present");
        let items_raw = args.get("items").expect("items should exist");
        let items: Vec<serde_json::Value> = serde_json::from_str(items_raw.get())?;
        let item_nums: Vec<i64> = items.iter().map(|v| v.as_i64().unwrap()).collect();

        assert_eq!(
            item_nums,
            vec![4, 5],
            "Without the fix: only the survivor's own items should be present (no accumulation)"
        );

        Ok(())
    }

    // =========================================================================
    // Tests for maybe_debounce_flow_node (flow node debouncing)
    // =========================================================================

    /// Helper: insert a child job with a parent flow.
    async fn insert_child_job_with_parent(
        db: &Pool<Postgres>,
        child_id: Uuid,
        parent_id: Uuid,
        workspace_id: &str,
    ) {
        sqlx::query!(
            "INSERT INTO v2_job (id, kind, tag, created_by, permissioned_as, permissioned_as_email, workspace_id, parent_job)
             VALUES ($1, 'noop', 'deno', 'test-user', 'u/test-user', 'test@windmill.dev', $2, $3)",
            child_id,
            workspace_id,
            parent_id,
        )
        .execute(db)
        .await
        .expect("insert v2_job (child)");

        sqlx::query!(
            "INSERT INTO v2_job_queue (id, workspace_id, scheduled_for, tag)
             VALUES ($1, $2, now(), 'deno')",
            child_id,
            workspace_id,
        )
        .execute(db)
        .await
        .expect("insert v2_job_queue (child)");

        sqlx::query!("INSERT INTO v2_job_runtime (id) VALUES ($1)", child_id)
            .execute(db)
            .await
            .expect("insert v2_job_runtime (child)");
    }

    /// Test: First flow node job in a debounce batch should set scheduled_for
    /// and create a debounce_key entry. The parent flow should remain in queue.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_flow_node_debounce_first_job(db: Pool<Postgres>) -> anyhow::Result<()> {
        let flow_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        insert_flow_job(&db, flow_id, "test-workspace", "f/test/my_flow").await;
        insert_child_job_with_parent(&db, child_id, flow_id, "test-workspace").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("test_flow_node_first".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);
        let mut tx = db.begin().await?;

        windmill_queue::jobs_ee::maybe_debounce_flow_node(
            &settings,
            child_id,
            flow_id,
            "f/test/my_flow",
            "step_a",
            "test-workspace",
            &args,
            &mut tx,
            &db,
        )
        .await?;

        tx.commit().await?;

        // Child job should still be in queue with delayed scheduled_for
        assert!(is_queued(&db, &child_id).await, "child should be queued");
        let sf = sqlx::query_scalar!(
            "SELECT scheduled_for FROM v2_job_queue WHERE id = $1",
            child_id
        )
        .fetch_one(&db)
        .await?;
        let diff = (sf - Utc::now()).num_seconds();
        assert!(
            diff >= 0 && diff <= 6,
            "scheduled_for should be in the future (up to ~5s), got {diff}s"
        );

        // Parent flow should still be in queue
        assert!(
            is_queued(&db, &flow_id).await,
            "parent flow should still be queued"
        );
        assert!(
            !is_completed(&db, &flow_id).await,
            "parent flow should not be completed"
        );

        // debounce_key should exist
        let dk = get_debounce_key(&db, "test_flow_node_first").await;
        assert!(dk.is_some(), "debounce_key entry should exist");
        let (dk_job, dk_prev, dk_times) = dk.unwrap();
        assert_eq!(dk_job, child_id);
        assert!(dk_prev.is_none());
        assert_eq!(dk_times, 0);

        Ok(())
    }

    /// Test: Second flow node job with same key should cancel the first child and
    /// complete the first parent flow with "Debounced by" result.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_flow_node_debounce_second_cancels_first(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        // Flow 1 with child 1
        let flow1 = Uuid::new_v4();
        let child1 = Uuid::new_v4();
        insert_flow_job(&db, flow1, "test-workspace", "f/test/my_flow").await;
        insert_child_job_with_parent(&db, child1, flow1, "test-workspace").await;

        // Flow 2 with child 2
        let flow2 = Uuid::new_v4();
        let child2 = Uuid::new_v4();
        insert_flow_job(&db, flow2, "test-workspace", "f/test/my_flow").await;
        insert_child_job_with_parent(&db, child2, flow2, "test-workspace").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("test_flow_node_cancel".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        // Push child 1
        {
            let args = PushArgs::from(&args_hm);
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce_flow_node(
                &settings,
                child1,
                flow1,
                "f/test/my_flow",
                "step_a",
                "test-workspace",
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // Push child 2 — should cancel child 1 and complete flow 1
        {
            let args = PushArgs::from(&args_hm);
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce_flow_node(
                &settings,
                child2,
                flow2,
                "f/test/my_flow",
                "step_a",
                "test-workspace",
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // child1 should be completed (debounced/skipped)
        assert!(
            is_completed(&db, &child1).await,
            "child1 should be completed"
        );
        assert!(
            !is_queued(&db, &child1).await,
            "child1 should not be in queue"
        );

        // flow1 (parent of child1) should also be completed
        assert!(
            is_completed(&db, &flow1).await,
            "flow1 should be completed (debounced)"
        );
        assert!(
            !is_queued(&db, &flow1).await,
            "flow1 should not be in queue"
        );

        // Check flow1 result contains "Debounced by"
        let result = sqlx::query_scalar!(
            "SELECT result::text FROM v2_job_completed WHERE id = $1",
            flow1
        )
        .fetch_one(&db)
        .await?;
        assert!(
            result.as_ref().is_some_and(|r| r.contains("Debounced by")),
            "flow1 result should contain 'Debounced by', got: {:?}",
            result
        );

        // child2 should still be in queue (it's the winner)
        assert!(
            is_queued(&db, &child2).await,
            "child2 should still be queued"
        );
        assert!(
            !is_completed(&db, &child2).await,
            "child2 should not be completed"
        );

        // flow2 should still be in queue
        assert!(is_queued(&db, &flow2).await, "flow2 should still be queued");

        // debounce_key should point to child2
        let dk = get_debounce_key(&db, "test_flow_node_cancel").await;
        assert!(dk.is_some());
        let (dk_job, _, dk_times) = dk.unwrap();
        assert_eq!(dk_job, child2);
        assert_eq!(dk_times, 1);

        Ok(())
    }

    /// Test: Default debounce key for flow nodes uses $workspace/flow/$path-$step_id.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_flow_node_debounce_default_key(db: Pool<Postgres>) -> anyhow::Result<()> {
        let flow_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();
        insert_flow_job(&db, flow_id, "test-workspace", "f/test/my_flow").await;
        insert_child_job_with_parent(&db, child_id, flow_id, "test-workspace").await;

        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: None, // No custom key — use default
            ..Default::default()
        };
        let args_hm = empty_args();
        let args = PushArgs::from(&args_hm);
        let mut tx = db.begin().await?;

        windmill_queue::jobs_ee::maybe_debounce_flow_node(
            &settings,
            child_id,
            flow_id,
            "f/test/my_flow",
            "step_a",
            "test-workspace",
            &args,
            &mut tx,
            &db,
        )
        .await?;

        tx.commit().await?;

        // Default key should be: test-workspace/flow/f/test/my_flow-step_a
        let expected_key = "test-workspace/flow/f/test/my_flow-step_a";
        let dk = get_debounce_key(&db, expected_key).await;
        assert!(dk.is_some(), "debounce_key with default key should exist");

        Ok(())
    }

    /// Test: Flow node debounce tracks debounced_times counter correctly.
    /// Each debounce call increments the counter in the debounce_key table.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_flow_node_debounce_counter_tracking(db: Pool<Postgres>) -> anyhow::Result<()> {
        // No limits set so counter never resets
        let settings = DebouncingSettings {
            debounce_delay_s: Some(5),
            debounce_key: Some("test_flow_node_counter".to_string()),
            ..Default::default()
        };
        let args_hm = empty_args();

        // Push 4 jobs. Each subsequent one increments debounced_times.
        for _ in 0..4 {
            let flow_id = Uuid::new_v4();
            let child_id = Uuid::new_v4();
            insert_flow_job(&db, flow_id, "test-workspace", "f/test/flow").await;
            insert_child_job_with_parent(&db, child_id, flow_id, "test-workspace").await;

            let args = PushArgs::from(&args_hm);
            let mut tx = db.begin().await?;
            windmill_queue::jobs_ee::maybe_debounce_flow_node(
                &settings,
                child_id,
                flow_id,
                "f/test/flow",
                "step_a",
                "test-workspace",
                &args,
                &mut tx,
                &db,
            )
            .await?;
            tx.commit().await?;
        }

        // After 4 jobs, debounced_times should be 3 (first job creates the entry with 0,
        // subsequent 3 jobs each increment it)
        let debounced_times = sqlx::query_scalar!(
            "SELECT debounced_times FROM debounce_key WHERE key = $1",
            "test_flow_node_counter"
        )
        .fetch_one(&db)
        .await?;
        assert_eq!(
            debounced_times, 3,
            "debounced_times should be 3 after 4 jobs"
        );

        Ok(())
    }
}
