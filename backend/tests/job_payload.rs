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

    /// Walk a completed flow's `flow_status` to find the child job UUID for a given
    /// container step at the given iteration. Used by nested-restart tests to
    /// reconstruct the `nested` chain that the API would resolve normally.
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
                            "id": "inner",
                            "value": {
                                "type": "rawscript",
                                "language": "deno",
                                "input_transforms": {
                                    "world": { "type": "javascript", "expr": "flow_input.world" }
                                },
                                "content": "export function main(world: string) { return `inner-${world}` }"
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
            assert_eq!(first_run.json_result().unwrap(), json!("inner-foo"));

            let original_branch_child =
                child_job_id_for_step(db, first_run.id, "branch", None).await;

            // Restart at the nested `inner` step inside `branch`. The new run should
            // re-execute step `a` (since it's before the BranchOne) AND `inner`
            // (the restart point). The chosen branch is locked to branch 0.
            let restarted = RunJob::from(JobPayload::RawFlow {
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
                        step_id: "inner".into(),
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
            assert_eq!(restarted.json_result().unwrap(), json!("inner-bar"));
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

            // Original child job for iteration 1 of `loop`.
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
        };
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
