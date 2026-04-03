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

    /// Assert that a completed job (or any descendant job) was executed
    /// by a dedicated worker process with the expected mode by checking job logs
    /// for the "dedicated worker {mode}:" prefix.
    /// Recurses through all descendants (children, grandchildren, etc.) to handle
    /// nested flow structures like branches and loops.
    async fn assert_ran_on_dedicated_worker(
        db: &Pool<Postgres>,
        job_id: uuid::Uuid,
        expected_mode: &str,
    ) {
        let marker = format!("dedicated worker {expected_mode}:");

        // BFS through job tree to find any descendant with the marker
        let mut queue = vec![job_id];
        let mut all_logs = Vec::new();

        while let Some(id) = queue.pop() {
            let logs: Option<String> =
                sqlx::query_scalar("SELECT logs FROM job_logs WHERE job_id = $1")
                    .bind(id)
                    .fetch_optional(db)
                    .await
                    .unwrap()
                    .flatten();

            if let Some(ref l) = logs {
                if l.contains(&marker) {
                    return;
                }
            }
            all_logs.push((id, logs));

            let child_ids: Vec<uuid::Uuid> =
                sqlx::query_scalar("SELECT id FROM v2_job WHERE parent_job = $1")
                    .bind(id)
                    .fetch_all(db)
                    .await
                    .unwrap();
            queue.extend(child_ids);
        }

        panic!(
            "No job in tree had '{marker}' in logs. All logs: {:?}",
            all_logs,
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

        let job = in_test_worker(
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
            },
            port,
        )
        .await;

        // for-loop over [1, 2, 3], each * 10 = [10, 20, 30]
        assert_eq!(job.json_result().unwrap(), json!([10, 20, 30]));
        // Verify the loop iterations ran on dedicated worker subprocesses
        assert_ran_on_dedicated_worker(&db, job.id, "bun").await;
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

    /// Test that a flow's Script step referencing a standalone dedicated script
    /// correctly routes through the standalone worker (first-registered-wins collision).
    /// Both the flow and the script are configured as dedicated workers.
    /// The standalone dedicated_double worker handles the job from both contexts.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_script_standalone_conflict(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Run as standalone script first
        let uuid_standalone = RunJob::from(JobPayload::ScriptHash {
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

        // Run via flow that references the same script
        let uuid_flow = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_conflict_standalone_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000011,
        })
        .arg("x", json!(7))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            async {
                listener.find(&uuid_standalone).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_flow).await;
            },
            port,
            vec![
                wp("test-workspace", "f/system/dedicated_double"),
                wp(
                    "test-workspace",
                    "flow/f/system/dedicated_conflict_standalone_flow",
                ),
            ],
        )
        .await;

        let job_standalone = completed_job(uuid_standalone, &db).await;
        let job_flow = completed_job(uuid_flow, &db).await;
        // Both should compute x * 2 = 14
        assert_eq!(job_standalone.json_result().unwrap(), json!(14));
        assert_eq!(job_flow.json_result().unwrap(), json!(14));
        assert_ran_on_dedicated_worker(&db, uuid_standalone, "bun").await;
        assert_ran_on_dedicated_worker(&db, uuid_flow, "bun").await;
        Ok(())
    }

    /// Test a dedicated flow with mixed languages: bun step (dedicated) + bash step (normal).
    /// The bash step should fall back to normal execution while the bun step uses dedicated worker.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_mixed_lang_fallback(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_mixed_lang_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000012,
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
                "flow/f/system/dedicated_mixed_lang_flow",
            )],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // bun: 5 + 10 = 15, bash: echo done = "done"
        assert_eq!(job.json_result().unwrap(), json!("done"));
        // The bun step ran on dedicated worker
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
        Ok(())
    }

    /// Test a squashed for-loop with a bash step (unsupported for flow runners).
    /// The bash step can't spawn a flow runner, so each iteration falls back to normal execution.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_runners_unsupported_lang(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
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

        let job = in_test_worker(
            db.clone(),
            async {
                RunJob::from(JobPayload::Flow {
                    path: "f/system/dedicated_flow_runners_bash".to_string(),
                    dedicated_worker: None,
                    apply_preprocessor: false,
                    version: 3000000000000013,
                })
                .run_until_complete(&db, false, port)
                .await
            },
            port,
        )
        .await;

        // for-loop over [1, 2, 3], bash echo done = ["done", "done", "done"]
        assert_eq!(job.json_result().unwrap(), json!(["done", "done", "done"]));
        Ok(())
    }

    /// Test Python runner group: two Python scripts sharing a workspace dep annotation.
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_python_runner_group(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid_a = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300050),
            path: "f/system/py_rg_script_a".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Python3,
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
            hash: windmill_common::scripts::ScriptHash(300051),
            path: "f/system/py_rg_script_b".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Python3,
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
                listener.find(&uuid_a).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_b).await;
            },
            port,
            vec![
                wp("test-workspace", "f/system/py_rg_script_a"),
                wp("test-workspace", "f/system/py_rg_script_b"),
            ],
        )
        .await;

        let job_a = completed_job(uuid_a, &db).await;
        let job_b = completed_job(uuid_b, &db).await;
        assert_eq!(job_a.json_result().unwrap(), json!(105));
        assert_eq!(job_b.json_result().unwrap(), json!(205));
        assert_ran_on_dedicated_worker(&db, uuid_a, "python3").await;
        assert_ran_on_dedicated_worker(&db, uuid_b, "python3").await;
        Ok(())
    }

    /// Test preprocessor in a runner group. Script with preprocessor shares a workspace dep
    /// with another script, forming a runner group. Tests exec_preprocess:{path}:{args} protocol.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_runner_group_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Run the preprocessor script with apply_preprocessor=true
        let uuid_pre = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300052),
            path: "f/system/rg_preprocess_script".to_string(),
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

        // Also run the other script in the group (no preprocessor)
        let uuid_other = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300053),
            path: "f/system/rg_preprocess_other".to_string(),
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
                listener.find(&uuid_pre).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_other).await;
            },
            port,
            vec![
                wp("test-workspace", "f/system/rg_preprocess_script"),
                wp("test-workspace", "f/system/rg_preprocess_other"),
            ],
        )
        .await;

        let job_pre = completed_job(uuid_pre, &db).await;
        let job_other = completed_job(uuid_other, &db).await;
        // preprocessor(5) → {x: 10}, main(10) → 110
        assert_eq!(job_pre.json_result().unwrap(), json!(110));
        // main(5) → 305
        assert_eq!(job_other.json_result().unwrap(), json!(305));
        assert_ran_on_dedicated_worker(&db, uuid_pre, "bun").await;
        assert_ran_on_dedicated_worker(&db, uuid_other, "bun").await;

        // Verify preprocessed flag was set on the preprocessor job
        let preprocessed: Option<bool> =
            sqlx::query_scalar("SELECT preprocessed FROM v2_job WHERE id = $1")
                .bind(uuid_pre)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(preprocessed, Some(true));

        Ok(())
    }

    /// Test a dedicated flow with a BranchOne module.
    /// When x > 10, branch path (x + 100) is taken; otherwise default (x + 200).
    /// Tests that dedicated workers are spawned for modules inside branches.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_branchone(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Test with x=20 (branch taken: x + 100 = 120)
        let uuid_branch = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_branch_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000014,
        })
        .arg("x", json!(20))
        .push(&db)
        .await;

        // Test with x=5 (default taken: x + 200 = 205)
        let uuid_default = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_branch_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000014,
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            async {
                listener.find(&uuid_branch).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_default).await;
            },
            port,
            vec![wp("test-workspace", "flow/f/system/dedicated_branch_flow")],
        )
        .await;

        let job_branch = completed_job(uuid_branch, &db).await;
        let job_default = completed_job(uuid_default, &db).await;
        assert_eq!(job_branch.json_result().unwrap(), json!(120));
        assert_eq!(job_default.json_result().unwrap(), json!(205));
        assert_ran_on_dedicated_worker(&db, uuid_branch, "bun").await;
        assert_ran_on_dedicated_worker(&db, uuid_default, "bun").await;
        Ok(())
    }

    /// Test Python standalone preprocessor (execd_preprocess: protocol).
    /// preprocessor doubles x, main adds 100: preprocessor(5) → {x:10}, main(10) → 110.
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_python_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300054),
            path: "f/system/py_preprocess_script".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Python3,
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
            vec![wp("test-workspace", "f/system/py_preprocess_script")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        assert_eq!(job.json_result().unwrap(), json!(110));
        assert_ran_on_dedicated_worker(&db, uuid, "python").await;

        let preprocessed: Option<bool> =
            sqlx::query_scalar("SELECT preprocessed FROM v2_job WHERE id = $1")
                .bind(uuid)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(preprocessed, Some(true));
        Ok(())
    }

    /// Test Deno standalone preprocessor (execd_preprocess: protocol).
    /// preprocessor doubles x, main adds 100: preprocessor(5) → {x:10}, main(10) → 110.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_deno_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300055),
            path: "f/system/deno_preprocess_script".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Deno,
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
            vec![wp("test-workspace", "f/system/deno_preprocess_script")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        assert_eq!(job.json_result().unwrap(), json!(110));
        assert_ran_on_dedicated_worker(&db, uuid, "deno").await;

        let preprocessed: Option<bool> =
            sqlx::query_scalar("SELECT preprocessed FROM v2_job WHERE id = $1")
                .bind(uuid)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(preprocessed, Some(true));
        Ok(())
    }

    /// Test Bunnative standalone preprocessor (V8 isolate path).
    /// preprocessor doubles x, main adds 100: preprocessor(5) → {x:10}, main(10) → 110.
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_bunnative_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300058),
            path: "f/system/bunnative_preprocess_script".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Bunnative,
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
            vec![wp("test-workspace", "f/system/bunnative_preprocess_script")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        assert_eq!(job.json_result().unwrap(), json!(110));
        assert_ran_on_dedicated_worker(&db, uuid, "nativets").await;

        let preprocessed: Option<bool> =
            sqlx::query_scalar("SELECT preprocessed FROM v2_job WHERE id = $1")
                .bind(uuid)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(preprocessed, Some(true));
        Ok(())
    }

    /// Test Python runner group preprocessor (exec_preprocess:{path}:{args} protocol).
    /// Two Python scripts sharing a workspace dep, one has a preprocessor.
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_python_runner_group_preprocessor(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid_pre = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300056),
            path: "f/system/py_rg_preprocess_a".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Python3,
            priority: None,
            apply_preprocessor: true,
            concurrency_settings: windmill_common::runnable_settings::ConcurrencySettings::default(
            ),
            debouncing_settings: windmill_common::runnable_settings::DebouncingSettings::default(),
        })
        .arg("x", json!(5))
        .push(&db)
        .await;

        let uuid_other = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300057),
            path: "f/system/py_rg_preprocess_b".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Python3,
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
                listener.find(&uuid_pre).await;
                let listener2 = listen_for_completed_jobs(&db).await;
                listener2.find(&uuid_other).await;
            },
            port,
            vec![
                wp("test-workspace", "f/system/py_rg_preprocess_a"),
                wp("test-workspace", "f/system/py_rg_preprocess_b"),
            ],
        )
        .await;

        let job_pre = completed_job(uuid_pre, &db).await;
        let job_other = completed_job(uuid_other, &db).await;
        // preprocessor(5) → {x: 10}, main(10) → 110
        assert_eq!(job_pre.json_result().unwrap(), json!(110));
        // main(5) → 305
        assert_eq!(job_other.json_result().unwrap(), json!(305));
        assert_ran_on_dedicated_worker(&db, uuid_pre, "python3").await;
        assert_ran_on_dedicated_worker(&db, uuid_other, "python3").await;

        let preprocessed: Option<bool> =
            sqlx::query_scalar("SELECT preprocessed FROM v2_job WHERE id = $1")
                .bind(uuid_pre)
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

    /// Test a dedicated worker script that uses a relative import to another workspace script.
    /// Verifies that build_loader's CURRENT_PATH resolves imports correctly.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_bun_relative_import(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300061),
            path: "f/system/dedicated_with_import".to_string(),
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
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "f/system/dedicated_with_import")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // helper(5) = 50, main = 50 + 1 = 51
        assert_eq!(job.json_result().unwrap(), json!(51));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
        Ok(())
    }

    /// Test a dedicated Python worker script that uses a relative import.
    /// Verifies that the Python loader resolves relative imports correctly.
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_python_relative_import(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::ScriptHash {
            hash: windmill_common::scripts::ScriptHash(300063),
            path: "f/system/py_dedicated_with_import".to_string(),
            cache_ttl: None,
            cache_ignore_s3_path: None,
            dedicated_worker: Some(true),
            language: windmill_common::scripts::ScriptLang::Python3,
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
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "f/system/py_dedicated_with_import")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // helper(5) = 50, main = 50 + 1 = 51
        assert_eq!(job.json_result().unwrap(), json!(51));
        assert_ran_on_dedicated_worker(&db, uuid, "python").await;
        Ok(())
    }

    /// Test a dedicated flow with a simple non-squashed for-loop (single step).
    /// is_simple_modules optimization inlines the step into the sub-flow job.
    /// The dedicated worker map uses node_id (parent loop id) as key.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_forloop_simple(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_forloop_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000015,
        })
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "flow/f/system/dedicated_forloop_flow")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // for-loop over [1, 2, 3], each * 10 = [10, 20, 30]
        assert_eq!(job.json_result().unwrap(), json!([10, 20, 30]));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
        Ok(())
    }

    /// Test a dedicated flow with a non-simple for-loop (multi-step).
    /// Two inner steps make is_simple_modules false, so each step runs as a
    /// separate job with runnable_path including /forloop-N/ nesting segments.
    /// Tests the extract_flow_root dispatch path.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_forloop_multi_step(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_forloop_multi_step_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000019,
        })
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp(
                "test-workspace",
                "flow/f/system/dedicated_forloop_multi_step_flow",
            )],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // for-loop over [1, 2, 3]: step b = value+1, step c = b*10
        // iter 1: 1+1=2, 2*10=20; iter 2: 2+1=3, 3*10=30; iter 3: 3+1=4, 4*10=40
        assert_eq!(job.json_result().unwrap(), json!([20, 30, 40]));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
        Ok(())
    }

    /// Test a dedicated flow with a while-loop.
    /// Inner step (x + 1) runs until result >= 3, with early stop.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_whileloop(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_whileloop_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000016,
        })
        .arg("x", json!(1))
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp(
                "test-workspace",
                "flow/f/system/dedicated_whileloop_flow",
            )],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // x=1: n=1→2 (continue), n=2→3 (stop). While-loop returns array of results.
        assert_eq!(job.json_result().unwrap(), json!([2, 3]));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
        Ok(())
    }

    /// Test a dedicated flow with BranchAll (parallel branches).
    /// Two branches: x+100 and x+200, both should dispatch to dedicated workers.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_branchall(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_branchall_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000017,
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
                "flow/f/system/dedicated_branchall_flow",
            )],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // BranchAll returns array of branch results: [105, 205]
        assert_eq!(job.json_result().unwrap(), json!([105, 205]));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
        Ok(())
    }

    /// Test a dedicated flow with deeply nested structure: for-loop containing a branchone.
    /// Tests dispatch through forloop-N/branchone-0/step_id path segments.
    #[sqlx::test(fixtures("base", "dedicated_flows"))]
    #[serial]
    async fn test_dedicated_flow_nested_branch_in_loop(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let uuid = RunJob::from(JobPayload::Flow {
            path: "f/system/dedicated_nested_flow".to_string(),
            dedicated_worker: Some(true),
            apply_preprocessor: false,
            version: 3000000000000018,
        })
        .push(&db)
        .await;

        let listener = listen_for_completed_jobs(&db).await;
        in_test_worker_dedicated(
            db.clone(),
            listener.find(&uuid),
            port,
            vec![wp("test-workspace", "flow/f/system/dedicated_nested_flow")],
        )
        .await;

        let job = completed_job(uuid, &db).await;
        // for-loop [1, 2]:
        //   iter 1 (value=1): branchone condition 1>1 false → default: 1+200=201
        //   iter 2 (value=2): branchone condition 2>1 true → branch: 2+100=102
        assert_eq!(job.json_result().unwrap(), json!([201, 102]));
        assert_ran_on_dedicated_worker(&db, uuid, "bun").await;
        Ok(())
    }
}
