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

    /// Assert that a completed job (or one of its child step jobs) was executed
    /// by a dedicated worker process with the expected mode by checking job logs
    /// for the "dedicated worker {mode}:" prefix.
    async fn assert_ran_on_dedicated_worker(
        db: &Pool<Postgres>,
        job_id: uuid::Uuid,
        expected_mode: &str,
    ) {
        let marker = format!("dedicated worker {expected_mode}:");

        // Check the job's logs (stored in job_logs table)
        let logs: Option<String> =
            sqlx::query_scalar("SELECT logs FROM job_logs WHERE job_id = $1")
                .bind(job_id)
                .fetch_optional(db)
                .await
                .unwrap()
                .flatten();

        if let Some(ref l) = logs {
            if l.contains(&marker) {
                return;
            }
        }

        // For flow jobs, check child step jobs' logs
        let child_ids: Vec<uuid::Uuid> =
            sqlx::query_scalar("SELECT id FROM v2_job WHERE parent_job = $1")
                .bind(job_id)
                .fetch_all(db)
                .await
                .unwrap();

        let mut child_logs = Vec::new();
        for child_id in &child_ids {
            let cl: Option<String> =
                sqlx::query_scalar("SELECT logs FROM job_logs WHERE job_id = $1")
                    .bind(child_id)
                    .fetch_optional(db)
                    .await
                    .unwrap()
                    .flatten();
            child_logs.push(cl);
        }

        assert!(
            child_logs
                .iter()
                .any(|l| l.as_ref().is_some_and(|l| l.contains(&marker))),
            "No job or child job had '{marker}' in logs. Job logs: {:?}, Child logs: {:?}",
            logs,
            child_logs,
        );
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
        assert_eq!(job.json_result().unwrap(), json!(15));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
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
        assert_eq!(job.json_result().unwrap(), json!(14));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
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
        assert_eq!(job.json_result().unwrap(), json!(18));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
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
        assert_eq!(job.json_result().unwrap(), json!(42));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
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
        assert_eq!(job_a.json_result().unwrap(), json!(105));
        assert_eq!(job_b.json_result().unwrap(), json!(205));
        assert_ran_on_dedicated_worker(&db, uuid_a, "bun").await;
        assert_ran_on_dedicated_worker(&db, uuid_b, "bun").await;
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

    /// Test a dedicated flow with a Deno inline RawScript step.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_deno(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_deno_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000005,
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "flow/f/system/dedicated_deno_flow")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        assert_eq!(job.json_result().unwrap(), json!(105));
        assert_ran_on_dedicated_worker(&db, uuid, "deno").await;
        Ok(())
    }

    /// Test a dedicated flow with a Python inline RawScript step.
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_python(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_python_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000006,
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "flow/f/system/dedicated_python_flow")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        assert_eq!(job.json_result().unwrap(), json!(105));
        assert_ran_on_dedicated_worker(&db, uuid, "python").await;
        Ok(())
    }

    /// Test a dedicated flow with a Bunnative (//native) inline RawScript step.
    /// Bunnative uses the V8 PrewarmedIsolate path instead of a subprocess wrapper.
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_bunnative(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_bunnative_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000007,
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
                "flow/f/system/dedicated_bunnative_flow",
            )],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        assert_eq!(job.json_result().unwrap(), json!(105));
        assert_ran_on_dedicated_worker(&db, uuid, "nativets").await;
        Ok(())
    }

    /// Test a dedicated flow with a Bun script using the //nodejs annotation.
    /// Validates that the annotation routes execution through Node.js instead of Bun.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_bun_nodejs(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_nodejs_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000008,
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "flow/f/system/dedicated_nodejs_flow")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        assert_eq!(job.json_result().unwrap(), json!(105));
        assert_ran_on_dedicated_worker(&db, uuid, "nodejs").await;
        Ok(())
    }

    /// Test that multiple dedicated worker subprocesses serialize job execution.
    /// Two scripts each sleep 500ms. If they ran concurrently, total wall time would be ~500ms.
    /// With serialization, it should be >= 1000ms and their execution intervals must not overlap.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_serialization(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid_a = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300020),
            path: "f/system/serial_a".to_string(),
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
        .arg("x", json!(10))
        .push(&db)
        .await;

        let uuid_b = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300021),
            path: "f/system/serial_b".to_string(),
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
        .arg("x", json!(20))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            async {
                listener.find(&uuid_a).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_b).await;
            },
            port,
            vec![
                wp("test-workspace", "f/system/serial_a"),
                wp("test-workspace", "f/system/serial_b"),
            ],
        )
        .await;

        let job_a = completed_job(uuid_a, &db).await;
        let job_b = completed_job(uuid_b, &db).await;
        let result_a = job_a.json_result().unwrap();
        let result_b = job_b.json_result().unwrap();
        assert_eq!(result_a["value"], json!(11));
        assert_eq!(result_b["value"], json!(22));

        // Both should have run on dedicated workers
        assert_ran_on_dedicated_worker(&db, uuid_a, "bun").await;
        assert_ran_on_dedicated_worker(&db, uuid_b, "bun").await;

        // Verify serialization: execution intervals must not overlap.
        // Each script records its own start_ms/end_ms via Date.now(), so these
        // reflect actual JS execution time, not queue pull time.
        let start_a = result_a["start_ms"].as_i64().unwrap();
        let end_a = result_a["end_ms"].as_i64().unwrap();
        let start_b = result_b["start_ms"].as_i64().unwrap();
        let end_b = result_b["end_ms"].as_i64().unwrap();

        let overlap = start_a < end_b && start_b < end_a;
        assert!(
            !overlap,
            "Dedicated worker jobs should not overlap! A: {}..{}, B: {}..{}",
            start_a, end_a, start_b, end_b,
        );

        Ok(())
    }

    /// Test that two dedicated flows with conflicting step IDs (both "a") are correctly
    /// disambiguated via runnable_path-based lookup (not step ID).
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_conflicting_step_ids(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid_a = RunJob::from(JobPayload::Flow {
            path: "f/system/conflict_flow_a".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000009,
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let uuid_b = RunJob::from(JobPayload::Flow {
            path: "f/system/conflict_flow_b".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000010,
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            async {
                listener.find(&uuid_a).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_b).await;
            },
            port,
            vec![
                wp("test-workspace", "flow/f/system/conflict_flow_a"),
                wp("test-workspace", "flow/f/system/conflict_flow_b"),
            ],
        )
        .await;

        let job_a = completed_job(uuid_a, &db).await;
        let job_b = completed_job(uuid_b, &db).await;
        // Flow A: x + 1000 = 1005, Flow B: x + 2000 = 2005
        assert_eq!(job_a.json_result().unwrap(), json!(1005));
        assert_eq!(job_b.json_result().unwrap(), json!(2005));
        assert_ran_on_dedicated_worker(&db, uuid_a, "bun").await;
        assert_ran_on_dedicated_worker(&db, uuid_b, "bun").await;
        Ok(())
    }

    /// Test that a dedicated worker script with a preprocessor function works correctly.
    /// The preprocessor doubles x, then main adds 100: preprocessor(5) → {x:10}, main(10) → 110.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300030),
            path: "f/system/preprocess_script".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Bun,
            priority: None,
            apply_preprocessor: true,
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
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "f/system/preprocess_script")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // preprocessor(5) → {x: 10}, main(10) → 110
        assert_eq!(job.json_result().unwrap(), json!(110));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;

        // Verify preprocessed flag was set
        let preprocessed: Option<bool> =
            sqlx::query_scalar("SELECT preprocessed FROM v2_job WHERE id = $1")
                .bind(uuid)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(preprocessed, Some(true));

        Ok(())
    }

    /// Test that two workspaces with the same script path are correctly isolated.
    /// Workspace 1: dedicated_double returns x * 2, Workspace 2: returns x * 3.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_cross_workspace_isolation(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid_ws1 = RunJob::from(JobPayload::ScriptHash {
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
        .arg("x", json!(7))
        .push(&db)
        .await;

        let uuid_ws2 = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300040),
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
        .workspace("test-workspace-2")
        .arg("x", json!(7))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            async {
                listener.find(&uuid_ws1).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_ws2).await;
            },
            port,
            vec![
                wp("test-workspace", "f/system/dedicated_double"),
                wp("test-workspace-2", "f/system/dedicated_double"),
            ],
        )
        .await;

        let job_ws1 = completed_job(uuid_ws1, &db).await;
        let job_ws2 = completed_job(uuid_ws2, &db).await;
        // Workspace 1: x * 2 = 14, Workspace 2: x * 3 = 21
        assert_eq!(job_ws1.json_result().unwrap(), json!(14));
        assert_eq!(job_ws2.json_result().unwrap(), json!(21));
        assert_ran_on_dedicated_worker(&db, uuid_ws1, "bun").await;
        assert_ran_on_dedicated_worker(&db, uuid_ws2, "bun").await;
        Ok(())
    }
}
