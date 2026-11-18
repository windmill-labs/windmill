//! The concurrency limiter's fail mode on a full key (`apply_concurrency_limit`,
//! windmill-queue/jobs_ee.rs), driven through `pull` without a worker.
//!
//! Run with:
//!   cargo test -p windmill-queue --test concurrency_on_limit_test --features private,enterprise

#[cfg(all(feature = "private", feature = "enterprise"))]
mod concurrency_on_limit {
    use std::collections::HashMap;

    use sqlx::{Pool, Postgres};
    use uuid::Uuid;
    use windmill_common::jobs::{JobPayload, RawCode};
    use windmill_common::runnable_settings::{ConcurrencySettings, OnConcurrencyLimit};
    use windmill_common::scripts::ScriptLang;
    use windmill_common::worker::make_pull_query;
    use windmill_queue::{
        get_mini_pulled_job, pull, push, try_admit_owned_job, PulledJobResult, PushArgs,
        PushIsolationLevel,
    };

    const WS: &str = "test-workspace";

    async fn push_job(db: &Pool<Postgres>, on_limit: Option<OnConcurrencyLimit>) -> Uuid {
        let payload = JobPayload::Code(RawCode {
            content: "echo".to_string(),
            path: Some("u/test-user/limited".to_string()),
            hash: None,
            language: ScriptLang::Bash,
            lock: None,
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: None,
            tag: None,
            concurrency_settings: ConcurrencySettings {
                concurrent_limit: Some(1),
                on_concurrency_limit: on_limit,
                ..Default::default()
            }
            .into(),
            debouncing_settings: Default::default(),
            modules: None,
        });
        let args = HashMap::new();
        let (id, tx) = push(
            db,
            PushIsolationLevel::IsolatedRoot(db.clone()),
            WS,
            payload,
            PushArgs { extra: None, args: &args },
            "test-user",
            "test@windmill.dev",
            "u/test-user".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            false,
            None,
            true,
            Some("bash".to_string()),
            None,
            None,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
        )
        .await
        .expect("push");
        tx.commit().await.expect("commit push");
        id
    }

    async fn pull_one(db: &Pool<Postgres>) -> PulledJobResult {
        let query = (String::new(), make_pull_query(&["bash".to_string()]));
        pull(db, false, "test-worker", Some(&query))
            .await
            .expect("pull")
    }

    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn fail_mode_fails_the_run_over_the_limit(db: Pool<Postgres>) {
        let first = push_job(&db, Some(OnConcurrencyLimit::Fail)).await;
        let second = push_job(&db, Some(OnConcurrencyLimit::Fail)).await;

        let admitted = pull_one(&db).await;
        assert_eq!(admitted.job.as_ref().map(|j| j.id), Some(first));
        assert!(admitted.error_while_preprocessing.is_none());

        let rejected = pull_one(&db).await;
        assert_eq!(rejected.job.as_ref().map(|j| j.id), Some(second));
        let err = rejected
            .error_while_preprocessing
            .expect("run over the limit fails");
        assert_eq!(err.name, "ConcurrencyLimitReached");
        assert!(err.message.contains("1 run(s) allowed"), "{}", err.message);

        // It never held a slot, so its end must not count against the key's window.
        let key_rows =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM concurrency_key WHERE job_id = $1")
                .bind(second)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(key_rows, 0);
    }

    // A run reserved by its owner (an AI agent's first tool) is admitted outside `pull` and must
    // fail the same way instead of waiting for a slot.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn fail_mode_applies_to_owned_admission(db: Pool<Postgres>) {
        let first = push_job(&db, Some(OnConcurrencyLimit::Fail)).await;
        let reserved = push_job(&db, Some(OnConcurrencyLimit::Fail)).await;
        assert_eq!(pull_one(&db).await.job.map(|j| j.id), Some(first));

        let job = get_mini_pulled_job(&db, &reserved)
            .await
            .unwrap()
            .expect("reserved job");
        let err = try_admit_owned_job(&db, &job)
            .await
            .expect_err("a reserved run over the limit fails");
        assert_eq!(
            windmill_common::worker::error_to_value(&err)["name"],
            "ConcurrencyLimitReached"
        );
    }

    /// Defers `id` an hour out, first in its line, as a job refused by its limit is left.
    async fn defer_first_in_line(db: &Pool<Postgres>, id: Uuid) {
        sqlx::query(
            "WITH place AS (
                UPDATE concurrency_key ck
                SET original_scheduled_for = q.scheduled_for,
                    place = q.scheduled_for - interval '1 minute',
                    tag = q.tag, priority = COALESCE(q.priority, -32768)
                FROM v2_job_queue q
                WHERE ck.job_id = $1 AND q.id = $1
            )
            UPDATE v2_job_queue SET scheduled_for = now() + interval '1 hour' WHERE id = $1",
        )
        .bind(id)
        .execute(db)
        .await
        .unwrap();
    }

    // With a slot free, a job behind another in its line waits its turn: it is not over its
    // limit, so fail mode does not apply.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn fail_mode_spares_a_run_waiting_its_turn(db: Pool<Postgres>) {
        let ahead = push_job(&db, Some(OnConcurrencyLimit::Fail)).await;
        let reserved = push_job(&db, Some(OnConcurrencyLimit::Fail)).await;
        let behind = push_job(&db, Some(OnConcurrencyLimit::Fail)).await;
        defer_first_in_line(&db, ahead).await;

        let job = get_mini_pulled_job(&db, &reserved)
            .await
            .unwrap()
            .expect("reserved job");
        let admitted = try_admit_owned_job(&db, &job)
            .await
            .expect("a reserved run waiting its turn is not failed");
        assert!(
            !admitted,
            "a reserved run behind another in line took the slot"
        );

        // Each pull takes one due run; the runs behind are refused for their turn, which wakes
        // `ahead`, the only one to get the slot.
        let mut started = None;
        for _ in 0..3 {
            let pulled = pull_one(&db).await;
            assert!(
                pulled.error_while_preprocessing.is_none(),
                "a run waiting its turn was failed"
            );
            if let Some(j) = pulled.job {
                started = Some(j.id);
                break;
            }
        }
        assert_eq!(started, Some(ahead));

        let completed = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM v2_job_completed WHERE id = ANY($1)",
        )
        .bind(vec![behind, reserved])
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(completed, 0, "a run waiting its turn was failed");
    }
}
