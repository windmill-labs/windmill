#[cfg(all(feature = "private", feature = "enterprise"))]
mod dedicated_worker_tests {
    use serde_json::json;
    use serial_test::serial;
    use sqlx::{Pool, Postgres};
    use windmill_common::jobs::JobPayload;
    use windmill_common::worker::WorkspacedPath;
    use windmill_test_utils::*;

    pub async fn initialize_tracing() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let _ = windmill_common::tracing_init::initialize_tracing(
                "test",
                &windmill_common::utils::Mode::Standalone,
                "test",
            );
        });
    }

    fn wp(workspace: &str, path: &str) -> WorkspacedPath {
        WorkspacedPath { workspace_id: workspace.to_string(), path: path.to_string() }
    }

    /// Test a dedicated flow with a single inline RawScript bun step.
    /// This is the regression test for the "Script not found" bug.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_rawscript(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_rawscript_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000001,
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp(
                "test-workspace",
                "flow/f/system/dedicated_rawscript_flow",
            )],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // x + 10 = 15
        assert_eq!(job.json_result().unwrap(), json!(15));
        Ok(())
    }

    /// Test a dedicated flow with a Script step referencing an external workspace script.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_workspace_script(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_script_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000002,
        })
        .arg("x", json!(7))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        // Need both the flow tag (for parent job) and the script tag (for step job)
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![
                wp("test-workspace", "flow/f/system/dedicated_script_flow"),
                wp("test-workspace", "f/system/dedicated_double"),
            ],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // x * 2 = 14
        assert_eq!(job.json_result().unwrap(), json!(14));
        Ok(())
    }

    /// Test a dedicated flow with multiple inline RawScript steps.
    /// Validates that each step gets its own wrapper key and no collisions occur.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_multiple_steps(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_multi_step_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000003,
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp(
                "test-workspace",
                "flow/f/system/dedicated_multi_step_flow",
            )],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // step a: x + 1 = 6, step b: 6 * 3 = 18
        assert_eq!(job.json_result().unwrap(), json!(18));
        Ok(())
    }

    /// Test a standalone dedicated script (not in a flow).
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_standalone_script(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300001),
            path: "f/system/dedicated_double".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Bun,
            priority: None,
            apply_preprocessor: false,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            ),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        })
        .arg("x", json!(21))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "f/system/dedicated_double")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // x * 2 = 42
        assert_eq!(job.json_result().unwrap(), json!(42));
        Ok(())
    }

    /// Test runner groups: two scripts sharing a workspace dependency are auto-grouped
    /// into a single runner process. Both should execute correctly.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_runner_group(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid_a = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300010),
            path: "f/system/rg_script_a".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Bun,
            priority: None,
            apply_preprocessor: false,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            ),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let uuid_b = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300011),
            path: "f/system/rg_script_b".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Bun,
            priority: None,
            apply_preprocessor: false,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            ),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            async {
                // Wait for both jobs to complete
                listener.find(&uuid_a).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_b).await;
            },
            port,
            vec![
                wp("test-workspace", "f/system/rg_script_a"),
                wp("test-workspace", "f/system/rg_script_b"),
            ],
        )
        .await;

        let job_a = completed_job(uuid_a, &db).await;
        let job_b = completed_job(uuid_b, &db).await;
        // rg_script_a: x + 100 = 105
        assert_eq!(job_a.json_result().unwrap(), json!(105));
        // rg_script_b: x + 200 = 205
        assert_eq!(job_b.json_result().unwrap(), json!(205));
        Ok(())
    }

    /// Test flow runners: a squashed for-loop spawns dedicated subprocesses.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_runners(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        // Reset dedicated_workers in WORKER_CONFIG to avoid pollution from previous tests.
        {
            let mut wc = windmill_common::worker::WORKER_CONFIG.write().await;
            wc.dedicated_worker = None;
            wc.dedicated_workers = None;
            wc.worker_tags = windmill_common::worker::DEFAULT_TAGS.clone();
            wc.priority_tags_sorted = vec![windmill_common::worker::PriorityTags {
                priority: 0,
                tags: wc.worker_tags.clone(),
            }];
            windmill_common::worker::store_suspended_pull_query(&wc).await;
            windmill_common::worker::store_pull_query(&wc).await;
        }
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let result = in_test_worker(
            db.clone(),
            async {
                RunJob::from(JobPayload::Flow {
                    path: "f/system/dedicated_flow_runners".to_string(),
                    dedicated_worker: None,
                    apply_preprocessor: false,
                    version: 3000000000000004,
                })
                .run_until_complete(&db, false, port)
                .await
                .json_result()
                .unwrap()
            },
            port,
        )
        .await;

        // for-loop over [1, 2, 3], each * 10 = [10, 20, 30]
        assert_eq!(result, json!([10, 20, 30]));
        Ok(())
    }
}
