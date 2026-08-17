mod job_payload {
    use serde_json::json;
    use sqlx::{Pool, Postgres};
    use windmill_common::flow_status::RestartedFrom;
    use windmill_common::flows::{FlowModule, FlowModuleValue, FlowValue};
    use windmill_common::jobs::JobPayload;
    use windmill_common::scripts::{ScriptHash, ScriptLang};

    use windmill_common::min_version::{
        MIN_VERSION, MIN_VERSION_IS_AT_LEAST_1_427, MIN_VERSION_IS_AT_LEAST_1_432,
        MIN_VERSION_IS_AT_LEAST_1_440,
    };
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

    use windmill_common::cache;
    use windmill_common::flows::FlowNodeId;
    use windmill_common::min_version::VersionConstraint;

    const VERSION_FLAGS: [&VersionConstraint; 3] = [
        &MIN_VERSION_IS_AT_LEAST_1_427,
        &MIN_VERSION_IS_AT_LEAST_1_432,
        &MIN_VERSION_IS_AT_LEAST_1_440,
    ];

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_script_hash_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123412),
                path: "f/system/hello".to_string(),
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default().into(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                labels: None,
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language: ScriptLang::Deno,
                priority: None,
                apply_preprocessor: false,
            })
            .arg("world", json!("foo"))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Hello foo!"));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_script_hash_payload_with_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let db = &db;
            let job = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123413),
                path: "f/system/hello_with_preprocessor".to_string(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language: ScriptLang::Deno,
                priority: None,
                apply_preprocessor: true,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                labels: None,
            })
            .run_until_complete_with(db, false, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("bar")));
            assert_eq!(args.get("bar"), Some(&json!("baz")));
            assert_eq!(job.json_result().unwrap(), json!("Hello bar baz"));
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_script_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
            debouncing_settings: Default::default(),
        })
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

        let flow_data = cache::flow::fetch_version_lite(&db, 1443253234253454)
            .await
            .unwrap();
        let flow_value = flow_data.value();
        let flow_scripts = {
            async fn load(db: &Pool<Postgres>, modules: &[FlowModule]) -> Vec<FlowNodeId> {
                let mut res = vec![];
                for module in modules {
                    let value =
                        serde_json::from_str::<FlowModuleValue>(module.value.get()).unwrap();
                    match value {
                        FlowModuleValue::FlowScript { id, .. } => res.push(id),
                        FlowModuleValue::ForloopFlow { modules_node: Some(flow_node), .. } => {
                            let flow_data = cache::flow::fetch_flow(db, flow_node).await.unwrap();
                            res.extend(Box::pin(load(db, &flow_data.value().modules)).await);
                        }
                        _ => {}
                    }
                }
                res
            }

            load(&db, &flow_value.modules).await
        };
        assert_eq!(flow_scripts.len(), 2);

        let test = || async {
            let result = RunJob::from(JobPayload::FlowScript {
                id: flow_scripts[0],
                language: ScriptLang::Deno,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                path: "f/system/hello/test-0".into(),
            })
            .arg("world", json!("foo"))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Hello foo!"));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        let test = || async {
            let result = RunJob::from(JobPayload::FlowScript {
                id: flow_scripts[1],
                language: ScriptLang::Deno,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                path: "f/system/hello/test-0".into(),
            })
            .arg("hello", json!("You know nothing Jean Neige"))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result,
                json!("Did you just say \"You know nothing Jean Neige\"??!")
            );
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_node_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
            debouncing_settings: Default::default(),
        })
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

        let flow_data = cache::flow::fetch_version_lite(&db, 1443253234253454)
            .await
            .unwrap();
        let flow_value = flow_data.value();
        let forloop_module =
            serde_json::from_str::<FlowModuleValue>(flow_value.modules[0].value.get()).unwrap();
        let FlowModuleValue::ForloopFlow { modules_node: Some(id), .. } = forloop_module else {
            panic!("Expected a forloop module with a flow node");
        };

        let test = || async {
            let result = RunJob::from(JobPayload::FlowNode {
                id,
                path: "f/system/hello_with_nodes_flow/forloop-0".into(),
            })
            .arg("iter", json!({ "value": "tests", "index": 0 }))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Did you just say \"Hello tests!\"??!"));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    async fn test_dependencies_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let result = RunJob::from(JobPayload::Dependencies {
            path: "f/system/hello".to_string(),
            hash: ScriptHash(123412),
            language: ScriptLang::Deno,
            debouncing_settings: Default::default(),
            dedicated_worker: None,
        })
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();

        assert_eq!(
            result.get("status").unwrap(),
            &json!("Successful lock file generation")
        );
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_dependencies_payload_min_1_427(db: Pool<Postgres>) -> anyhow::Result<()> {
        MIN_VERSION.store(std::sync::Arc::new(
            MIN_VERSION_IS_AT_LEAST_1_427.version().clone(),
        ));
        test_dependencies_payload(db).await?;
        Ok(())
    }
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_dependencies_payload_min_1_432(db: Pool<Postgres>) -> anyhow::Result<()> {
        MIN_VERSION.store(std::sync::Arc::new(
            MIN_VERSION_IS_AT_LEAST_1_432.version().clone(),
        ));
        test_dependencies_payload(db).await?;
        Ok(())
    }
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_dependencies_payload_min_1_440(db: Pool<Postgres>) -> anyhow::Result<()> {
        MIN_VERSION.store(std::sync::Arc::new(
            MIN_VERSION_IS_AT_LEAST_1_440.version().clone(),
        ));
        test_dependencies_payload(db).await?;
        Ok(())
    }

    // Just test that deploying a flow work as expected.
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_dependencies_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::FlowDependencies {
                path: "f/system/hello_with_nodes_flow".to_string(),
                dedicated_worker: None,
                version: 1443253234253454,
                debouncing_settings: Default::default(),
            })
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result.get("status").unwrap(),
                &json!("Successful lock file generation")
            );
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_raw_flow_dependencies_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::RawFlowDependencies {
                path: "none".to_string(),
                flow_value: serde_json::from_value(json!({
                    "modules": [{
                        "id": "a",
                        "value": {
                            "type": "rawscript",
                            "content": r#"export function main(world: string) {
                                const greet = `Hello ${world}!`;
                                console.log(greet)
                                return greet
                            }"#,
                            "language": "deno",
                            "input_transforms": {
                                "world": { "type": "javascript", "expr": "flow_input.world" }
                            }
                        }
                    }],
                    "schema": {
                        "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "properties": { "world": { "type": "string" } },
                        "type": "object",
                        "order": [  "world" ]
                    }
                }))
                .unwrap(),
            })
            .arg("skip_flow_update", json!(true))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            let result = RunJob::from(JobPayload::RawFlow {
                value: serde_json::from_value::<FlowValue>(
                    result.get("updated_flow_value").unwrap().clone(),
                )
                .unwrap(),
                path: None,
                restarted_from: None,
            })
            .arg("world", json!("Jean Neige"))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Hello Jean Neige!"));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_raw_script_dependencies_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::RawScriptDependencies {
                script_path: "none".into(),
                content: r#"export function main(world: string) {
                    const greet = `Hello ${world}!`;
                    console.log(greet)
                    return greet
                }"#
                .into(),
                language: ScriptLang::Deno,
            })
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result,
                json!({ "lock": "", "status": "Successful lock file generation" })
            );
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::Flow {
                path: "f/system/hello_with_nodes_flow".to_string(),
                dedicated_worker: None,
                apply_preprocessor: false,
                version: 1443253234253454,
                labels: None,
            })
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result,
                json!([
                    "Did you just say \"Hello foo!\"??!",
                    "Did you just say \"Hello bar!\"??!",
                    "Did you just say \"Hello baz!\"??!",
                ])
            );
        };
        // Test the not "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
            debouncing_settings: Default::default(),
        })
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
        // Test the "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_flow_payload_with_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let db = &db;
        let test = || async {
            use windmill_common::flow_status::{FlowStatus, FlowStatusModule};

            let job = RunJob::from(JobPayload::Flow {
                path: "f/system/hello_with_preprocessor".to_string(),
                dedicated_worker: None,
                apply_preprocessor: true,
                version: 1443253234253456,
                labels: None,
            })
            .run_until_complete_with(db, false, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            let flow_status = job.flow_status.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("bar")));
            assert_eq!(args.get("bar"), Some(&json!("baz")));
            assert_eq!(job.json_result().unwrap(), json!("Hello bar-baz"));
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
            let flow_status = serde_json::from_value::<FlowStatus>(flow_status.clone()).unwrap();
            let FlowStatusModule::Success { job, .. } = flow_status.preprocessor_module.unwrap()
            else {
                panic!("Expected a success preprocessor module");
            };
            let pp_id = job;
            let job = sqlx::query!(
                "SELECT preprocessed, script_entrypoint_override FROM v2_job WHERE id = $1",
                pp_id
            )
            .fetch_one(db)
            .await
            .unwrap();
            assert_eq!(job.preprocessed, Some(true));
            assert_eq!(
                job.script_entrypoint_override.as_deref(),
                Some("preprocessor")
            );
        };
        // Test the not "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_preprocessor".to_string(),
            dedicated_worker: None,
            version: 1443253234253456,
            debouncing_settings: Default::default(),
        })
        .run_until_complete(db, false, port)
        .await
        .json_result()
        .unwrap();
        // Test the "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_restarted_flow_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let completed_job_id = RunJob::from(JobPayload::Flow {
                path: "f/system/hello_with_nodes_flow".to_string(),
                dedicated_worker: None,
                apply_preprocessor: true,
                version: 1443253234253454,
                labels: None,
            })
            .run_until_complete(&db, false, port)
            .await
            .id;

            let result = RunJob::from(JobPayload::RestartedFlow {
                completed_job_id,
                step_id: "a".into(),
                branch_or_iteration_n: None,
                flow_version: None,
                branch_chosen: None,
                nested: None,
            })
            .arg("iter", json!({ "value": "tests", "index": 0 }))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(
                result,
                json!([
                    "Did you just say \"Hello foo!\"??!",
                    "Did you just say \"Hello bar!\"??!",
                    "Did you just say \"Hello baz!\"??!",
                ])
            );
        };
        // Test the not "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        // Deploy the flow to produce the "lite" version.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
            debouncing_settings: Default::default(),
        })
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
        // Test the "lite" flow.
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_raw_flow_payload(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let result = RunJob::from(JobPayload::RawFlow {
                value: serde_json::from_value(json!({
                    "modules": [{
                        "id": "a",
                        "value": {
                            "type": "rawscript",
                            "content": r#"export function main(world: string) {
                                const greet = `Hello ${world}!`;
                                console.log(greet)
                                return greet
                            }"#,
                            "language": "deno",
                            "input_transforms": {
                                "world": { "type": "javascript", "expr": "flow_input.world" }
                            }
                        }
                    }],
                    "schema": {
                        "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "properties": { "world": { "type": "string" } },
                        "type": "object",
                        "order": [  "world" ]
                    }
                }))
                .unwrap(),
                path: None,
                restarted_from: None,
            })
            .arg("world", json!("Jean Neige"))
            .run_until_complete(&db, false, port)
            .await
            .json_result()
            .unwrap();

            assert_eq!(result, json!("Hello Jean Neige!"));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_raw_flow_payload_with_restarted_from(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let db = &db;
        let test = |restarted_from, arg, result| async move {
            let job = RunJob::from(JobPayload::RawFlow {
                value: serde_json::from_value(json!({
                    "modules": [{
                        "id": "a",
                        "value": {
                            "type": "rawscript",
                            "content": r#"export function main(world: string) {
                                return `Hello ${world}!`;
                            }"#,
                            "language": "deno",
                            "input_transforms": {
                                "world": { "type": "javascript", "expr": "flow_input.world" }
                            }
                        }
                    }, {
                        "id": "b",
                        "value": {
                            "type": "rawscript",
                            "content": r#"export function main(world: string, a: string) {
                                return `${a} ${world}!`;
                            }"#,
                            "language": "deno",
                            "input_transforms": {
                                "world": { "type": "javascript", "expr": "flow_input.world" },
                                "a": { "type": "javascript", "expr": "results.a" }
                            }
                        }
                    }, {
                        "id": "c",
                        "value": {
                            "type": "forloopflow",
                            "iterator": { "type": "javascript", "expr": "['a', 'b', 'c']" },
                            "modules": [{
                                "value": {
                                    "input_transforms": {
                                        "world": { "type": "javascript", "expr": "flow_input.world" },
                                        "b": { "type": "javascript", "expr": "results.b" },
                                        "x": { "type": "javascript", "expr": "flow_input.iter.value" }
                                    },
                                    "type": "rawscript",
                                    "language": "deno",
                                    "content": r#"export function main(world: string, b: string, x: string) {
                                        return `${x}: ${b} ${world}!`;
                                    }"#,
                                },
                            }],
                        }
                    }],
                    "schema": {
                        "$schema": "https://json-schema.org/draft/2020-12/schema",
                        "properties": { "world": { "type": "string" } },
                        "type": "object",
                        "order": [  "world" ]
                    }
                }))
                .unwrap(),
                path: None,
                restarted_from,
            })
            .arg("world", arg)
            .run_until_complete(db, false, port)
            .await;

            assert_eq!(job.json_result().unwrap(), result);
            job.id
        };
        let flow_job_id = test(
            None,
            json!("foo"),
            json!([
                "a: Hello foo! foo! foo!",
                "b: Hello foo! foo! foo!",
                "c: Hello foo! foo! foo!"
            ]),
        )
        .await;
        let flow_job_id = test(
            Some(RestartedFrom {
                flow_job_id,
                step_id: "a".into(),
                branch_or_iteration_n: None,
                flow_version: None,
                ..Default::default()
            }),
            json!("foo"),
            json!([
                "a: Hello foo! foo! foo!",
                "b: Hello foo! foo! foo!",
                "c: Hello foo! foo! foo!"
            ]),
        )
        .await;
        let flow_job_id = test(
            Some(RestartedFrom {
                flow_job_id,
                step_id: "b".into(),
                branch_or_iteration_n: None,
                flow_version: None,
                ..Default::default()
            }),
            json!("bar"),
            json!([
                "a: Hello foo! bar! bar!",
                "b: Hello foo! bar! bar!",
                "c: Hello foo! bar! bar!"
            ]),
        )
        .await;
        let _ = test(
            Some(RestartedFrom {
                flow_job_id,
                step_id: "c".into(),
                branch_or_iteration_n: Some(1),
                flow_version: None,
                ..Default::default()
            }),
            json!("yolo"),
            json!([
                "a: Hello foo! bar! bar!",
                "b: Hello foo! bar! yolo!",
                "c: Hello foo! bar! yolo!"
            ]),
        )
        .await;
        Ok(())
    }

    /// Walk a chain of `(step_id, iter)` hops through nested completed jobs to
    /// reach a deeper child UUID. Used by nested-restart tests to assert that
    /// preserved iterations / preserved siblings reuse the original child UUID
    /// (and that re-run ones get a new UUID).
    async fn nested_child_job_id(
        db: &Pool<Postgres>,
        mut job_id: uuid::Uuid,
        path: &[(&str, Option<usize>)],
    ) -> uuid::Uuid {
        for (step_id, iter) in path {
            job_id = child_job_id_for_step(db, job_id, step_id, *iter).await;
        }
        job_id
    }

    /// Look up a single child job UUID from a completed flow's `flow_status` for
    /// a given top-level step (and optional iteration index for ForLoop /
    /// BranchAll containers).
    async fn child_job_id_for_step(
        db: &Pool<Postgres>,
        flow_job_id: uuid::Uuid,
        step_id: &str,
        iter: Option<usize>,
    ) -> uuid::Uuid {
        let row = sqlx::query!(
            "SELECT flow_status FROM v2_job_completed WHERE id = $1",
            flow_job_id,
        )
        .fetch_one(db)
        .await
        .unwrap();
        let raw = row.flow_status.expect("flow_status missing");
        let status: windmill_common::flow_status::FlowStatus =
            serde_json::from_value(raw).expect("parse flow_status");
        let module = status
            .modules
            .iter()
            .find(|m| m.id() == step_id)
            .expect("step not found in completed flow_status");
        match iter {
            Some(i) => module
                .flow_jobs()
                .expect("expected flow_jobs on container module")
                .get(i)
                .copied()
                .expect("iteration not found in flow_jobs"),
            None => module.job().expect("expected single child job on module"),
        }
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_nested_restart_inside_branchone(db: Pool<Postgres>) -> anyhow::Result<()> {
        use windmill_common::flow_status::BranchChosen;

        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // BranchOne with TWO inner steps so we can verify that earlier siblings
        // inside the branch are preserved when restarting at a later one.
        let flow_value: FlowValue = serde_json::from_value(json!({
            "modules": [{
                "id": "a",
                "value": {
                    "type": "rawscript",
                    "language": "deno",
                    "input_transforms": {
                        "world": { "type": "javascript", "expr": "flow_input.world" }
                    },
                    "content": "export function main(world: string) { return `pre-${world}` }"
                }
            }, {
                "id": "branch",
                "value": {
                    "type": "branchone",
                    "default": [],
                    "branches": [{
                        "expr": "true",
                        "modules": [{
                            "id": "inner_first",
                            "value": {
                                "type": "rawscript",
                                "language": "deno",
                                "input_transforms": {
                                    "world": { "type": "javascript", "expr": "flow_input.world" }
                                },
                                "content": "export function main(world: string) { return `first-${world}` }"
                            }
                        }, {
                            "id": "inner_second",
                            "value": {
                                "type": "rawscript",
                                "language": "deno",
                                "input_transforms": {
                                    "world": { "type": "javascript", "expr": "flow_input.world" },
                                    "first": { "type": "javascript", "expr": "results.inner_first" }
                                },
                                "content": "export function main(world: string, first: string) { return `${first}|second-${world}` }"
                            }
                        }]
                    }]
                }
            }],
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": { "world": { "type": "string" } },
                "order": ["world"]
            }
        }))
        .unwrap();

        let test = || async {
            let db = &db;
            let first_run = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: None,
            })
            .arg("world", json!("foo"))
            .run_until_complete(db, false, port)
            .await;
            assert_eq!(
                first_run.json_result().unwrap(),
                json!("first-foo|second-foo")
            );

            let original_branch_child =
                child_job_id_for_step(db, first_run.id, "branch", None).await;
            // Capture the original inner_first job UUID so we can later assert the
            // restart-at-inner_second run preserves it byte-for-byte (i.e., the
            // step is reused, not silently re-executed with the same args).
            let original_inner_first =
                nested_child_job_id(db, first_run.id, &[("branch", None), ("inner_first", None)])
                    .await;
            let original_inner_second = nested_child_job_id(
                db,
                first_run.id,
                &[("branch", None), ("inner_second", None)],
            )
            .await;

            // Restart at the FIRST inner step. Both inner steps re-run with new args.
            let restarted_at_first = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: Some(RestartedFrom {
                    flow_job_id: first_run.id,
                    step_id: "branch".into(),
                    branch_or_iteration_n: None,
                    flow_version: None,
                    branch_chosen: Some(BranchChosen::Branch { branch: 0 }),
                    nested: Some(Box::new(RestartedFrom {
                        flow_job_id: original_branch_child,
                        step_id: "inner_first".into(),
                        branch_or_iteration_n: None,
                        flow_version: None,
                        branch_chosen: None,
                        nested: None,
                    })),
                }),
            })
            .arg("world", json!("bar"))
            .run_until_complete(db, false, port)
            .await;
            assert_eq!(
                restarted_at_first.json_result().unwrap(),
                json!("first-bar|second-bar")
            );

            // Restart at the SECOND inner step (the user-reported case). The first
            // inner step's original output ("first-foo") must be preserved as a
            // dependency for the second step's `results.inner_first` reference.
            let restarted_at_second = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: Some(RestartedFrom {
                    flow_job_id: first_run.id,
                    step_id: "branch".into(),
                    branch_or_iteration_n: None,
                    flow_version: None,
                    branch_chosen: Some(BranchChosen::Branch { branch: 0 }),
                    nested: Some(Box::new(RestartedFrom {
                        flow_job_id: original_branch_child,
                        step_id: "inner_second".into(),
                        branch_or_iteration_n: None,
                        flow_version: None,
                        branch_chosen: None,
                        nested: None,
                    })),
                }),
            })
            .arg("world", json!("baz"))
            .run_until_complete(db, false, port)
            .await;
            // `inner_first` keeps its original "first-foo" result; only `inner_second`
            // re-runs with the new `world=baz` arg.
            assert_eq!(
                restarted_at_second.json_result().unwrap(),
                json!("first-foo|second-baz")
            );

            // Identity proof: inner_first's UUID in the restarted run must be the
            // exact same UUID as in the original run (no re-execution), and
            // inner_second must be a fresh UUID (was re-executed).
            let new_inner_first = nested_child_job_id(
                db,
                restarted_at_second.id,
                &[("branch", None), ("inner_first", None)],
            )
            .await;
            let new_inner_second = nested_child_job_id(
                db,
                restarted_at_second.id,
                &[("branch", None), ("inner_second", None)],
            )
            .await;
            assert_eq!(
                new_inner_first, original_inner_first,
                "inner_first should reuse the original job UUID"
            );
            assert_ne!(
                new_inner_second, original_inner_second,
                "inner_second should be a fresh job"
            );
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_nested_restart_inside_forloop_iteration(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Sequential ForLoop with an inner step. We restart at the inner step inside
        // iteration 1, expecting iteration 0 to remain unchanged and only iteration
        // 1 to re-run with new input.
        let flow_value: FlowValue = serde_json::from_value(json!({
            "modules": [{
                "id": "loop",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "['x', 'y']" },
                    "skip_failures": false,
                    "modules": [{
                        "id": "inner",
                        "value": {
                            "type": "rawscript",
                            "language": "deno",
                            "input_transforms": {
                                "iter_val": { "type": "javascript", "expr": "flow_input.iter.value" },
                                "tag": { "type": "javascript", "expr": "flow_input.tag" }
                            },
                            "content": "export function main(iter_val: string, tag: string) { return `${tag}:${iter_val}` }"
                        }
                    }]
                }
            }],
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": { "tag": { "type": "string" } },
                "order": ["tag"]
            }
        }))
        .unwrap();

        let test = || async {
            let db = &db;
            let first_run = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: None,
            })
            .arg("tag", json!("first"))
            .run_until_complete(db, false, port)
            .await;
            assert_eq!(
                first_run.json_result().unwrap(),
                json!(["first:x", "first:y"])
            );

            // Original child jobs for both iterations.
            let original_iter0_child =
                child_job_id_for_step(db, first_run.id, "loop", Some(0)).await;
            let original_iter1_child =
                child_job_id_for_step(db, first_run.id, "loop", Some(1)).await;

            let restarted = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: Some(RestartedFrom {
                    flow_job_id: first_run.id,
                    step_id: "loop".into(),
                    branch_or_iteration_n: Some(1),
                    flow_version: None,
                    branch_chosen: None,
                    nested: Some(Box::new(RestartedFrom {
                        flow_job_id: original_iter1_child,
                        step_id: "inner".into(),
                        branch_or_iteration_n: None,
                        flow_version: None,
                        branch_chosen: None,
                        nested: None,
                    })),
                }),
            })
            .arg("tag", json!("second"))
            .run_until_complete(db, false, port)
            .await;
            // Iteration 0 keeps the original "first:x"; iteration 1 re-ran with "second"
            assert_eq!(
                restarted.json_result().unwrap(),
                json!(["first:x", "second:y"])
            );
            // Identity proof: iter 0's child UUID survives intact, iter 1 is fresh.
            let new_iter0_child = child_job_id_for_step(db, restarted.id, "loop", Some(0)).await;
            let new_iter1_child = child_job_id_for_step(db, restarted.id, "loop", Some(1)).await;
            assert_eq!(
                new_iter0_child, original_iter0_child,
                "iteration 0 should reuse the original child job"
            );
            assert_ne!(
                new_iter1_child, original_iter1_child,
                "iteration 1 should be a fresh job"
            );
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    /// Regression test for the FlowNode case: when a flow is deployed, container
    /// bodies (BranchOne branches, ForLoop iterations) get compiled into FlowNodes.
    /// A nested restart targeting a step inside such a container spawns a child as
    /// `JobPayload::RestartedFlow` against the original FlowNode-kind child. Without
    /// preserving the original `JobKind` (the bug we just fixed), the new spawn
    /// would land as `JobKind::Flow` with the FlowNode id misinterpreted as a
    /// flow_version id, failing with "Flow version not found".
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_nested_restart_inside_deployed_loop_flownode(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // The fixture's `hello_with_nodes_flow` is a deployed flow with a top-level
        // ForLoop iterating ['foo', 'bar', 'baz'], whose body has modules `b` (greet)
        // and `c` (echo greeting). After deployment the body is wrapped in a FlowNode.
        let test = || async {
            let db = &db;
            let first_run = RunJob::from(JobPayload::Flow {
                path: "f/system/hello_with_nodes_flow".to_string(),
                dedicated_worker: None,
                apply_preprocessor: true,
                version: 1443253234253454,
                labels: None,
            })
            .run_until_complete(db, false, port)
            .await;

            // Iteration 1's child job (kind=flownode after FlowDependencies has run).
            let original_iter1_child = child_job_id_for_step(db, first_run.id, "a", Some(1)).await;

            // Restart at inner step `c` of iteration 1. Iteration 0 keeps its original
            // result; iteration 1 re-runs starting at `c` (so `b` is preserved as
            // Success inside the iteration); iteration 2 fresh-runs.
            let restarted = RunJob::from(JobPayload::RestartedFlow {
                completed_job_id: first_run.id,
                step_id: "a".into(),
                branch_or_iteration_n: Some(1),
                flow_version: None,
                branch_chosen: None,
                nested: Some(Box::new(RestartedFrom {
                    flow_job_id: original_iter1_child,
                    step_id: "c".into(),
                    branch_or_iteration_n: None,
                    flow_version: None,
                    branch_chosen: None,
                    nested: None,
                })),
            })
            .run_until_complete(db, false, port)
            .await;
            // Same args, same output as original — the goal is to verify the spawn
            // doesn't 500 with "Flow version not found" when the inner child is a
            // FlowNode.
            assert_eq!(
                restarted.json_result().unwrap(),
                json!([
                    "Did you just say \"Hello foo!\"??!",
                    "Did you just say \"Hello bar!\"??!",
                    "Did you just say \"Hello baz!\"??!",
                ])
            );
        };
        // Exercise BOTH the pre-deployment path (raw flow inline, FlowPreview kind)
        // and the post-deployment path (FlowNode kind) — the latter is the case the
        // user originally hit.
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
            debouncing_settings: Default::default(),
        })
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    /// Two ForLoops nested inside each other. We restart at the leaf step inside
    /// outer iter K=1 and inner iter M=1. Iters before K stay frozen at the parent
    /// level; iter K's inner iters before M stay frozen at the inner level; the
    /// inner-M leaf re-runs with new input.
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_nested_restart_inside_nested_forloops(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        // Outer iterates [1,2]; for each outer iter, inner iterates ['x','y'];
        // leaf returns "<tag>:<inner>". Tag comes from flow_input so we can
        // observe which leaves re-ran.
        let flow_value: FlowValue = serde_json::from_value(json!({
            "modules": [{
                "id": "outer",
                "value": {
                    "type": "forloopflow",
                    "iterator": { "type": "javascript", "expr": "[1, 2]" },
                    "skip_failures": false,
                    "modules": [{
                        "id": "inner",
                        "value": {
                            "type": "forloopflow",
                            "iterator": { "type": "javascript", "expr": "['x', 'y']" },
                            "skip_failures": false,
                            "modules": [{
                                "id": "leaf",
                                "value": {
                                    "type": "rawscript",
                                    "language": "deno",
                                    "input_transforms": {
                                        "tag": { "type": "javascript", "expr": "flow_input.tag" },
                                        "iv": { "type": "javascript", "expr": "flow_input.iter.value" }
                                    },
                                    "content": "export function main(tag: string, iv: string) { return `${tag}:${iv}` }"
                                }
                            }]
                        }
                    }]
                }
            }],
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": { "tag": { "type": "string" } },
                "order": ["tag"]
            }
        }))
        .unwrap();

        let test = || async {
            let db = &db;
            let first_run = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: None,
            })
            .arg("tag", json!("first"))
            .run_until_complete(db, false, port)
            .await;
            assert_eq!(
                first_run.json_result().unwrap(),
                json!([["first:x", "first:y"], ["first:x", "first:y"]])
            );

            // Snapshot every original child UUID we'll later compare against.
            let original_outer_iter0 =
                child_job_id_for_step(db, first_run.id, "outer", Some(0)).await;
            let outer_iter1_child = child_job_id_for_step(db, first_run.id, "outer", Some(1)).await;
            let original_outer1_inner0 =
                child_job_id_for_step(db, outer_iter1_child, "inner", Some(0)).await;
            let inner_iter1_child =
                child_job_id_for_step(db, outer_iter1_child, "inner", Some(1)).await;

            let restarted = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: Some(RestartedFrom {
                    flow_job_id: first_run.id,
                    step_id: "outer".into(),
                    branch_or_iteration_n: Some(1),
                    flow_version: None,
                    branch_chosen: None,
                    nested: Some(Box::new(RestartedFrom {
                        flow_job_id: outer_iter1_child,
                        step_id: "inner".into(),
                        branch_or_iteration_n: Some(1),
                        flow_version: None,
                        branch_chosen: None,
                        nested: Some(Box::new(RestartedFrom {
                            flow_job_id: inner_iter1_child,
                            step_id: "leaf".into(),
                            branch_or_iteration_n: None,
                            flow_version: None,
                            branch_chosen: None,
                            nested: None,
                        })),
                    })),
                }),
            })
            .arg("tag", json!("second"))
            .run_until_complete(db, false, port)
            .await;
            // Outer iter 0 frozen; outer iter 1 has inner iter 0 frozen and inner
            // iter 1 re-run with new tag.
            assert_eq!(
                restarted.json_result().unwrap(),
                json!([["first:x", "first:y"], ["first:x", "second:y"]])
            );
            // Identity proof at every layer:
            //   * outer iter 0's child UUID survives (preserved iteration)
            //   * outer iter 1's child UUID is fresh (re-spawned as RestartedFlow)
            //   * inside that fresh outer-iter-1 child, inner iter 0's UUID equals
            //     the ORIGINAL outer-iter-1's inner-iter-0 UUID (preserved through
            //     the nested chain)
            //   * inner iter 1's UUID is fresh
            let new_outer_iter0 = child_job_id_for_step(db, restarted.id, "outer", Some(0)).await;
            let new_outer_iter1 = child_job_id_for_step(db, restarted.id, "outer", Some(1)).await;
            let new_outer1_inner0 =
                child_job_id_for_step(db, new_outer_iter1, "inner", Some(0)).await;
            let new_outer1_inner1 =
                child_job_id_for_step(db, new_outer_iter1, "inner", Some(1)).await;
            assert_eq!(
                new_outer_iter0, original_outer_iter0,
                "outer iter 0 should reuse original child"
            );
            assert_ne!(
                new_outer_iter1, outer_iter1_child,
                "outer iter 1 should be a fresh child"
            );
            assert_eq!(
                new_outer1_inner0, original_outer1_inner0,
                "inner iter 0 inside outer iter 1 should reuse original child"
            );
            assert_ne!(
                new_outer1_inner1, inner_iter1_child,
                "inner iter 1 inside outer iter 1 should be a fresh child"
            );
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    /// BranchOne nested inside another BranchOne. We restart at the deepest
    /// leaf with both branches locked to their original choices.
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_nested_restart_inside_nested_branchone(db: Pool<Postgres>) -> anyhow::Result<()> {
        use windmill_common::flow_status::BranchChosen;

        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let flow_value: FlowValue = serde_json::from_value(json!({
            "modules": [{
                "id": "outer_branch",
                "value": {
                    "type": "branchone",
                    "default": [],
                    "branches": [{
                        "expr": "true",
                        "modules": [{
                            "id": "inner_branch",
                            "value": {
                                "type": "branchone",
                                "default": [],
                                "branches": [{
                                    "expr": "true",
                                    "modules": [{
                                        "id": "leaf",
                                        "value": {
                                            "type": "rawscript",
                                            "language": "deno",
                                            "input_transforms": {
                                                "tag": { "type": "javascript", "expr": "flow_input.tag" }
                                            },
                                            "content": "export function main(tag: string) { return `leaf:${tag}` }"
                                        }
                                    }]
                                }]
                            }
                        }]
                    }]
                }
            }],
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": { "tag": { "type": "string" } },
                "order": ["tag"]
            }
        }))
        .unwrap();

        let test = || async {
            let db = &db;
            let first_run = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: None,
            })
            .arg("tag", json!("first"))
            .run_until_complete(db, false, port)
            .await;
            assert_eq!(first_run.json_result().unwrap(), json!("leaf:first"));

            let outer_branch_child =
                child_job_id_for_step(db, first_run.id, "outer_branch", None).await;
            let inner_branch_child =
                child_job_id_for_step(db, outer_branch_child, "inner_branch", None).await;

            let restarted = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: Some(RestartedFrom {
                    flow_job_id: first_run.id,
                    step_id: "outer_branch".into(),
                    branch_or_iteration_n: None,
                    flow_version: None,
                    branch_chosen: Some(BranchChosen::Branch { branch: 0 }),
                    nested: Some(Box::new(RestartedFrom {
                        flow_job_id: outer_branch_child,
                        step_id: "inner_branch".into(),
                        branch_or_iteration_n: None,
                        flow_version: None,
                        branch_chosen: Some(BranchChosen::Branch { branch: 0 }),
                        nested: Some(Box::new(RestartedFrom {
                            flow_job_id: inner_branch_child,
                            step_id: "leaf".into(),
                            branch_or_iteration_n: None,
                            flow_version: None,
                            branch_chosen: None,
                            nested: None,
                        })),
                    })),
                }),
            })
            .arg("tag", json!("second"))
            .run_until_complete(db, false, port)
            .await;
            assert_eq!(restarted.json_result().unwrap(), json!("leaf:second"));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    /// Restart at a step inside a SUBFLOW (`Flow{path}`) called from a parent flow.
    /// The parent invokes `f/system/hello_with_nodes_flow` (which has a top-level
    /// ForLoop). We restart at the inner step `c` of iteration 1 of that loop,
    /// from the parent's perspective. This exercises the cross-job-spawn chain
    /// crossing a subflow boundary AND descending into a ForLoop inside the
    /// subflow.
    #[cfg(feature = "deno_core")]
    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_nested_restart_inside_subflow(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let flow_value: FlowValue = serde_json::from_value(json!({
            "modules": [{
                "id": "h",
                "value": {
                    "path": "f/system/hello_with_nodes_flow",
                    "type": "flow",
                    "input_transforms": {}
                }
            }],
            "schema": {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {},
                "order": []
            }
        }))
        .unwrap();

        let test = || async {
            let db = &db;
            let first_run = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: None,
            })
            .run_until_complete(db, false, port)
            .await;
            // The subflow returns the loop's results; the parent flow returns the
            // subflow's result as its own output.
            assert_eq!(
                first_run.json_result().unwrap(),
                json!([
                    "Did you just say \"Hello foo!\"??!",
                    "Did you just say \"Hello bar!\"??!",
                    "Did you just say \"Hello baz!\"??!",
                ])
            );

            let subflow_child = child_job_id_for_step(db, first_run.id, "h", None).await;
            let original_subflow_iter0 =
                child_job_id_for_step(db, subflow_child, "a", Some(0)).await;
            let subflow_iter1_child = child_job_id_for_step(db, subflow_child, "a", Some(1)).await;
            // Capture the leaf "c" inside iter 0 of the original subflow, which is
            // a step we explicitly do NOT restart at — its UUID must survive into
            // the restarted run unchanged, despite crossing two parent layers
            // (parent's `h` → subflow's `a` → iter 0 → `c`).
            let original_subflow_iter0_c =
                nested_child_job_id(db, subflow_child, &[("a", Some(0)), ("c", None)]).await;

            let restarted = RunJob::from(JobPayload::RawFlow {
                value: flow_value.clone(),
                path: None,
                restarted_from: Some(RestartedFrom {
                    flow_job_id: first_run.id,
                    step_id: "h".into(),
                    branch_or_iteration_n: None,
                    flow_version: None,
                    branch_chosen: None,
                    nested: Some(Box::new(RestartedFrom {
                        flow_job_id: subflow_child,
                        step_id: "a".into(),
                        branch_or_iteration_n: Some(1),
                        flow_version: None,
                        branch_chosen: None,
                        nested: Some(Box::new(RestartedFrom {
                            flow_job_id: subflow_iter1_child,
                            step_id: "c".into(),
                            branch_or_iteration_n: None,
                            flow_version: None,
                            branch_chosen: None,
                            nested: None,
                        })),
                    })),
                }),
            })
            .run_until_complete(db, false, port)
            .await;
            // Same input → same output, produced via the cross-job restart chain.
            assert_eq!(
                restarted.json_result().unwrap(),
                json!([
                    "Did you just say \"Hello foo!\"??!",
                    "Did you just say \"Hello bar!\"??!",
                    "Did you just say \"Hello baz!\"??!",
                ])
            );
            // Identity proof across the subflow boundary:
            //   * the parent's `h` child UUID is fresh (subflow re-spawned as a
            //     RestartedFlow) — we don't compare it directly, but inside it…
            //   * iter 0 of the subflow's `a` loop reuses the original UUID
            //   * iter 1 is a fresh UUID
            //   * the inner step `c` inside the preserved iter 0 also reuses its
            //     original UUID (preservation propagates two layers deep)
            let new_subflow_child = child_job_id_for_step(db, restarted.id, "h", None).await;
            let new_subflow_iter0 =
                child_job_id_for_step(db, new_subflow_child, "a", Some(0)).await;
            let new_subflow_iter1 =
                child_job_id_for_step(db, new_subflow_child, "a", Some(1)).await;
            let new_subflow_iter0_c =
                nested_child_job_id(db, new_subflow_child, &[("a", Some(0)), ("c", None)]).await;
            assert_eq!(
                new_subflow_iter0, original_subflow_iter0,
                "subflow iter 0 should reuse original child"
            );
            assert_ne!(
                new_subflow_iter1, subflow_iter1_child,
                "subflow iter 1 should be a fresh child"
            );
            assert_eq!(
                new_subflow_iter0_c, original_subflow_iter0_c,
                "step `c` inside preserved iter 0 should reuse its original UUID"
            );
        };
        // Pre-deployment: subflow's iteration wrapper is a FlowPreview (inline
        // RawFlow); leaf `c` runs as a rawscript embedded in the wrapper.
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        // Post-deployment: subflow's body is compiled into a FlowNode; the iteration
        // wrapper jobs run with kind=FlowNode and the leaf becomes a FlowScript
        // referencing a flow_node id. Re-running the same scenario exercises the
        // kind-preservation path through the subflow boundary AND a level deeper.
        let _ = RunJob::from(JobPayload::FlowDependencies {
            path: "f/system/hello_with_nodes_flow".to_string(),
            dedicated_worker: None,
            version: 1443253234253454,
            debouncing_settings: Default::default(),
        })
        .run_until_complete(&db, false, port)
        .await
        .json_result()
        .unwrap();
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_dedicated_worker_preprocessor_bun(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let db = &db;
            let job = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123414),
                path: "f/system/hello_preprocessor_dedicated_bun".to_string(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language: ScriptLang::Bun,
                priority: None,
                apply_preprocessor: true,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                labels: None,
            })
            .arg("foo", json!("hello"))
            .arg("bar", json!("world"))
            .run_until_complete_with(db, false, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("hello_preprocessed")));
            assert_eq!(args.get("bar"), Some(&json!("world_preprocessed")));
            assert_eq!(
                job.json_result().unwrap(),
                json!("Hello hello_preprocessed world_preprocessed")
            );
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_dedicated_worker_preprocessor_python(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let db = &db;
            let job = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123415),
                path: "f/system/hello_preprocessor_dedicated_python".to_string(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language: ScriptLang::Python3,
                priority: None,
                apply_preprocessor: true,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                labels: None,
            })
            .arg("foo", json!("hello"))
            .arg("bar", json!("world"))
            .run_until_complete_with(db, false, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("hello_preprocessed")));
            assert_eq!(args.get("bar"), Some(&json!("world_preprocessed")));
            assert_eq!(
                job.json_result().unwrap(),
                json!("Hello hello_preprocessed world_preprocessed")
            );
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_dedicated_worker_preprocessor_deno(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let db = &db;
            let job = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123416),
                path: "f/system/hello_preprocessor_dedicated_deno".to_string(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language: ScriptLang::Deno,
                priority: None,
                apply_preprocessor: true,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                labels: None,
            })
            .arg("foo", json!("hello"))
            .arg("bar", json!("world"))
            .run_until_complete_with(db, false, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("hello_preprocessed")));
            assert_eq!(args.get("bar"), Some(&json!("world_preprocessed")));
            assert_eq!(
                job.json_result().unwrap(),
                json!("Hello hello_preprocessed world_preprocessed")
            );
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_bunnative_preprocessor(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let db = &db;
            let job = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123417),
                path: "f/system/hello_preprocessor_bunnative".to_string(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language: ScriptLang::Bunnative,
                priority: None,
                apply_preprocessor: true,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                labels: None,
            })
            .arg("foo", json!("hello"))
            .arg("bar", json!("world"))
            .run_until_complete_with(db, false, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("hello_preprocessed")));
            assert_eq!(args.get("bar"), Some(&json!("world_preprocessed")));
            assert_eq!(
                job.json_result().unwrap(),
                json!("Hello hello_preprocessed world_preprocessed")
            );
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }

    #[sqlx::test(fixtures("base", "hello"))]
    async fn test_dedicated_worker_preprocessor_bunnative(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();

        let test = || async {
            let db = &db;
            let job = RunJob::from(JobPayload::ScriptHash {
                hash: ScriptHash(123418),
                path: "f/system/hello_preprocessor_dedicated_bunnative".to_string(),
                cache_ttl: None,
                cache_ignore_s3_path: None,
                dedicated_worker: None,
                language: ScriptLang::Bunnative,
                priority: None,
                apply_preprocessor: true,
                concurrency_settings:
                    windmill_common::runnable_settings::ConcurrencySettings::default(),
                debouncing_settings:
                    windmill_common::runnable_settings::DebouncingSettings::default(),
                labels: None,
            })
            .arg("foo", json!("hello"))
            .arg("bar", json!("world"))
            .run_until_complete_with(db, false, port, |id| async move {
                let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", id)
                    .fetch_one(db)
                    .await
                    .unwrap();
                assert_eq!(job.preprocessed, Some(false));
            })
            .await;

            let args = job.args.as_ref().unwrap();
            assert_eq!(args.get("foo"), Some(&json!("hello_preprocessed")));
            assert_eq!(args.get("bar"), Some(&json!("world_preprocessed")));
            assert_eq!(
                job.json_result().unwrap(),
                json!("Hello hello_preprocessed world_preprocessed")
            );
            let job = sqlx::query!("SELECT preprocessed FROM v2_job WHERE id = $1", job.id)
                .fetch_one(db)
                .await
                .unwrap();
            assert_eq!(job.preprocessed, Some(true));
        };
        test_for_versions(VERSION_FLAGS.iter().copied(), test).await;
        Ok(())
    }
}
