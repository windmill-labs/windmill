mod common;

#[cfg(feature = "test_job_debouncing")]
use windmill_api_client::types::NewScript;

#[cfg(feature = "test_job_debouncing")]
fn quick_ns(
    content: &str,
    language: windmill_api_client::types::ScriptLang,
    path: &str,
    lock: Option<String>,
    parent_hash: Option<String>,
) -> NewScript {
    NewScript {
        content: content.into(),
        language,
        lock,
        parent_hash,
        path: path.into(),
        concurrent_limit: None,
        concurrency_time_window_s: None,
        cache_ttl: None,
        dedicated_worker: None,
        description: "".to_string(),
        draft_only: None,
        envs: vec![],
        is_template: None,
        kind: None,
        summary: "".to_string(),
        tag: None,
        schema: std::collections::HashMap::new(),
        ws_error_handler_muted: Some(false),
        priority: None,
        delete_after_use: None,
        timeout: None,
        restart_unless_cancelled: None,
        deployment_message: None,
        concurrency_key: None,
        visible_to_runner_only: None,
        no_main_func: None,
        codebase: None,
        has_preprocessor: None,
        on_behalf_of_email: None,
        assets: vec![],
    }
}

#[cfg(feature = "test_job_debouncing")]
mod dependency_job_debouncing {
    async fn trigger_djob_for(
        client: &windmill_api_client::Client,
        path: &str,
        parent_hash: &str,
        content: Option<String>,
    ) {
        use super::quick_ns;
        use windmill_api_client::types::ScriptLang;
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    &content.unwrap_or(
                        "
def main():
    pass
                        "
                        .into(),
                    ),
                    ScriptLang::Python3,
                    path,
                    None,
                    Some(parent_hash.into()),
                ),
            )
            .await
            .unwrap();
    }
    // TODO: test workspaces specific things,

    /// # Double referenced even
    /// It follows this topology:
    ///
    ///   ┌─FLOW──────────┐
    ///   │┌───┐┌───┐┌───┐│
    ///   ││ A ││ B ││ C ││
    ///   │└─▲─┘▲───▲└─▲─┘│
    ///   └──┼──┼───┼──┼──┘
    ///     ┌┴──┴┐ ┌┴──┴┐
    ///     │L_LF│ │R_LF│
    ///     └────┘ └────┘
    ///
    /// p.s: "LF" stands for "Leaf", "L" - "Left", "R" - "Right"
    mod flows {
        use crate::common::{in_test_worker, init_client, listen_for_completed_jobs};
        use crate::dependency_job_debouncing::trigger_djob_for;
        use std::time::Duration;
        use tokio::time::sleep;
        use tokio_stream::StreamExt;

        /// 1. LLF and RLF create two djobs for flow at the same and fall into single debounce
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_1(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            // This tests if debouncing and consolidation works.
            // Also makes sures that dependency job does not create new flow version
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            let (client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            // Verify locks are empty
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT jsonb_array_elements(value->'modules')->'value'->>'lock' AS lock FROM flow")
                    .fetch_all(&db)
                    .await
                    .unwrap(),
                    vec![
                        Some("# py: 3.11\n".into()),
                        Some("# py: 3.11\n".into()),
                        Some("# py: 3.11\n".into())
                    ]
                );
            }

            // Trigger both at the same time.
            {
                trigger_djob_for(
                    &client,
                    "f/dre/leaf_left",
                    "0000000000051658",
                    Some("#requirements:\n#bottle==0.13.2\ndef main():\npass".into()),
                )
                .await;

                trigger_djob_for(
                    &client,
                    "f/dre/leaf_right",
                    "000000000005165B",
                    Some("#requirements:\n#tiny==0.1.3\ndef main():\npass".into()),
                )
                .await;
            }

            in_test_worker(
                &db,
                async {
                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre/leaf_left"
                    );

                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre/leaf_right"
                    );

                    // Let jobs propagate
                    sleep(Duration::from_secs(2)).await;

                    // Verify there is only one queued job that is scheduled for atleast 3s ahead.
                    {
                        let q = sqlx::query_scalar!(
                            "SELECT (scheduled_for - created_at) FROM v2_job_queue"
                        )
                        .fetch_all(&db)
                        .await
                        .unwrap();

                        assert_eq!(1, q.len());
                        assert!(dbg!(q[0].unwrap().microseconds) > 1_000_000 /* 1 second */);
                    }

                    // Verify debounce_stale_data and debounce_key
                    {
                        let q = sqlx::query!(
                            "SELECT
                        dsd.to_relock,
                        dk.key
                    FROM debounce_key dk
                    JOIN debounce_stale_data dsd ON dk.job_id = dsd.job_id"
                        )
                        .fetch_all(&db)
                        .await
                        .unwrap();

                        // Should be single entry
                        assert!(q.len() == 1);

                        // This verifies that all nodes_to_relock are consolidated correctly
                        // AND there is no doublicats
                        assert_eq!(
                            q[0].to_relock.clone().unwrap(),
                            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
                        );

                        // Should be workspace specific and these specific tests cover only dependency job debouncing
                        assert_eq!(
                            q[0].key.clone(),
                            "test-workspace:f/dre/flow:dependency".to_owned(),
                        );
                    }

                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre/flow"
                    );
                },
                port,
            )
            .await;

            // Verify latest flow.version property
            {
                // Latest flow version should not be initial one
                assert_eq!(
                    1, // Automatically assigned
                    dbg!(sqlx::query_scalar!(
                        "SELECT versions[2] FROM flow WHERE path = 'f/dre/flow'"
                    )
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap())
                );

                // Only second element should be our initial version
                assert_eq!(
                    1443253234253454, // < Predefined in fixture
                    dbg!(sqlx::query_scalar!(
                        "SELECT versions[1] FROM flow WHERE path = 'f/dre/flow'"
                    )
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap())
                );
            }

            // Verify that there is only two versions of flow in global flow_version
            {
                assert_eq!(
                    2,
                    sqlx::query_scalar!(
                        "SELECT COUNT(*) FROM flow_version WHERE path = 'f/dre/flow'"
                    )
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap()
                );
            }

            // Verify locks
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT jsonb_array_elements(value->'modules')->'value'->>'lock' AS lock FROM flow")
                    .fetch_all(&db)
                    .await
                    .unwrap(),
                    vec![
                        Some("# py: 3.11\nbottle==0.13.2".into()),
                        Some("# py: 3.11\nbottle==0.13.2\ntiny==0.1.3".into()),
                        Some("# py: 3.11\ntiny==0.1.3".into())
                    ]
                );
            }

            // TODO:
            // tracing_assertions::assert_has_events!([info("This is supposed to be called")]);
            // 2025-10-06T14:31:10.832469Z  WARN windmill-worker/src/worker.rs:1593: pull took more than 0.1s (0.222477345) this is a sign that the database is undersized for this load. empty: true, err: true worker=wk-default-nixos-EzDEL hostname=nixos

            // Verify cleanup
            {
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from debounce_stale_data")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
            }

            Ok(())
        }

        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_left(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            use crate::common::RunJob;

            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }
            // TODO: We don't care about timer. If there is no timer, it will be set automatically for djobs??
            let (_client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            // Trigger both at the same time.
            {
                let mut args = std::collections::HashMap::new();
                args.insert(
                    "dbg_djob_sleep".to_owned(),
                    // Execution should take this seconds
                    windmill_common::worker::to_raw_value(&20),
                );

                args.insert(
                    "triggered_by_relative_import".to_owned(),
                    // Execution should take this seconds
                    windmill_common::worker::to_raw_value(&()),
                );

                let (_flow_id, new_tx) = windmill_queue::push(
                    &db,
                    windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone()),
                    "test-workspace",
                    windmill_common::jobs::JobPayload::FlowDependencies {
                        path: "f/dre/flow".to_owned(),
                        dedicated_worker: None,
                        version: 1443253234253454,
                    },
                    windmill_queue::PushArgs { args: &args, extra: None },
                    "admin",
                    "admin@windmill.dev",
                    "admin".to_owned(),
                    Some("trigger.dependents.to.recompute.dependencies"),
                    // Debounce period
                    Some(chrono::Utc::now() + chrono::Duration::seconds(5)),
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                    false,
                    None,
                    true,
                    Some("dependency".into()),
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
                .unwrap();
                new_tx.commit().await.unwrap();

                // let handle = {
                //     // let mut completed = listen_for_completed_jobs(&db).await;
                //     let db2 = db.clone();
                //     // let uuid = flow_id.clone();
                //     tokio::spawn(async move {
                //         in_test_worker(
                //             &db2,
                //             tokio::time::sleep(tokio::time::Duration::from_secs(60)),
                //             // completed.find(&uuid),
                //             port,
                //         )
                //         .await;
                //     })
                // };

                let db2 = db.clone();
                in_test_worker(
                    &db2,
                    async {
                        // This job should execute and then try to start another job that will get debounced.
                        RunJob::from(windmill_common::jobs::JobPayload::Dependencies {
                            path: "f/dre/leaf_right".to_owned(),
                            hash: 333403.into(),
                            language: windmill_common::scripts::ScriptLang::Python3,
                            dedicated_worker: None,
                        })
                        // .arg("dbg_djob_sleep", serde_json::json!(10))
                        .run_until_complete(&db, false, port)
                        .await;

                        RunJob::from(windmill_common::jobs::JobPayload::Dependencies {
                            path: "f/dre/leaf_left".to_owned(),
                            hash: 333400.into(),
                            language: windmill_common::scripts::ScriptLang::Python3,
                            dedicated_worker: None,
                        })
                        // So set it to this long
                        .arg("dbg_djob_sleep", serde_json::json!(10))
                        .run_until_complete(&db, false, port)
                        .await;

                        completed.next().await; // leaf_right
                        completed.next().await; // leaf_left
                        completed.next().await; // importer
                        completed.next().await; // importer
                    },
                    port,
                )
                .await;
            }

            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            let r = sqlx::query_scalar!("SELECT runnable_id FROM v2_job ORDER BY created_at DESC")
                .fetch_all(&db)
                .await
                .unwrap();

            assert_eq!(r.len(), 4);
            assert!(r.contains(&Some(1)));
            assert!(r.contains(&Some(333400)));
            assert!(r.contains(&Some(333403)));
            assert!(r.contains(&Some(1443253234253454)));

            Ok(())
        }

        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_2(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            let (_client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_IS_AT_LEAST_1_440
                    .write()
                    .await;
                *mvsd = true;
            }

            // Function to create a dependency job
            let create_dependency_job =
                |delay,
                 nodes_to_relock,
                 db: sqlx::Pool<sqlx::Postgres>,
                 version,
                 debounce_job_id_o| async move {
                    let mut args = std::collections::HashMap::new();
                    args.insert(
                        "dbg_sleep_between_pull_and_debounce_key_removal".to_owned(),
                        windmill_common::worker::to_raw_value(&delay),
                    );

                    args.insert(
                        "nodes_to_relock".to_owned(),
                        windmill_common::worker::to_raw_value(&nodes_to_relock),
                    );

                    args.insert(
                        "triggered_by_relative_import".to_string(),
                        windmill_common::worker::to_raw_value(&()),
                    );

                    args.insert(
                        "dbg_create_job_for_unexistant_flow_version".to_string(),
                        windmill_common::worker::to_raw_value(&()),
                    );

                    let (job_uuid, new_tx) = windmill_queue::push(
                        &db,
                        windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone()),
                        "test-workspace",
                        windmill_common::jobs::JobPayload::FlowDependencies {
                            path: "f/dre/flow".to_owned(),
                            dedicated_worker: None,
                            version,
                        },
                        windmill_queue::PushArgs { args: &args, extra: None },
                        "admin",
                        "admin@windmill.dev",
                        "admin".to_owned(),
                        Some("trigger.dependents.to.recompute.dependencies"),
                        Some(chrono::Utc::now()), // Schedule immediately
                        None,
                        None,
                        None,
                        None,
                        None,
                        false,
                        false,
                        None,
                        true,
                        Some("dependency".into()),
                        None,
                        None,
                        None,
                        None,
                        false,
                        None,
                        debounce_job_id_o,
                        None,
                        None,
                    )
                    .await
                    .unwrap();

                    new_tx.commit().await.unwrap();
                    job_uuid
                };

            // Push the first dependency job
            let job1 =
                create_dependency_job(2, vec!["a", "b"], db.clone(), 1443253234253454, None).await;

            let db2 = db.clone();
            in_test_worker(
                &db2,
                async {
                    // Small delay to ensure the job is marked as running
                    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

                    // Now is the time when the job is pulled, but debounce_key is not yet cleared.
                    {
                        assert!(sqlx::query_scalar!(
                            "SELECT running FROM v2_job_queue WHERE id = $1",
                            job1
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap());

                        assert_eq!(
                            sqlx::query_scalar!("SELECT COUNT(*) FROM debounce_key")
                                .fetch_one(&db)
                                .await
                                .unwrap()
                                .unwrap(),
                            1
                        );

                        dbg!(sqlx::query_scalar!("SELECT kind::text FROM v2_job")
                            .fetch_one(&db)
                            .await
                            .unwrap()
                            .unwrap());
                    }

                    let job2 =
                        create_dependency_job(0, vec!["b", "c"], db.clone(), 1, Some(job1)).await;

                    // Process the first job completion, and the second job should also get debounced by this one
                    completed.next().await;

                    // Verify that both jobs were created and processed
                    assert_eq!(job1, job2, "Second job should be debounced");
                },
                port,
            )
            .await;

            assert_eq!(
                vec![1443253234253454, 1],
                sqlx::query_scalar!("SELECT versions FROM flow WHERE path = 'f/dre/flow'")
                    .fetch_one(&db)
                    .await
                    .unwrap()
            );

            // Verify cleanup - all debounce entries should be cleaned up
            assert_eq!(
                0,
                sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap(),
                "All debounce_key entries should be cleaned up after job completion"
            );

            assert_eq!(
                0,
                sqlx::query_scalar!("SELECT COUNT(*) from debounce_stale_data")
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap(),
                "All debounce_stale_data entries should be cleaned up after job completion"
            );

            // Verify locks
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT jsonb_array_elements(value->'modules')->'value'->>'lock' AS lock FROM flow")
                    .fetch_all(&db)
                    .await
                    .unwrap(),
                    vec![
                        Some("# py: 3.11\nbottle==0.13.2".into()),
                        Some("# py: 3.11\nbottle==0.13.2\ntiny==0.1.3".into()),
                        Some("# py: 3.11\ntiny==0.1.3".into())
                    ]
                );
            }

            Ok(())
        }
        /// 2. Same as second test, however first flow djob will take longer than second debounce.
        /// NOTE: This test should be ran in debug mode with `private` features enabled. In release it will not work properly.
        #[cfg(all(feature = "python", feature = "private"))]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        // #[windmill::all_min_versions]
        async fn test_3(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            // This tests checks if concurrency limit works correcly and there is no race conditions.
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            let (_client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            // At this point we should have two
            let mut job_ids = vec![];
            let push_job = |delay, version, db, nodes_to_relock, debounce_job_id_o| async move {
                let mut args = std::collections::HashMap::new();
                args.insert(
                    "dbg_djob_sleep".to_owned(),
                    // First one will create delay for 5 seconds
                    // The second will have no delay at all.
                    windmill_common::worker::to_raw_value(&delay),
                );

                args.insert(
                    "nodes_to_relock".to_owned(),
                    windmill_common::worker::to_raw_value(&nodes_to_relock),
                );

                args.insert(
                    "triggered_by_relative_import".to_string(),
                    windmill_common::worker::to_raw_value(&()),
                );

                let (job_uuid, new_tx) = windmill_queue::push(
                    &db,
                    windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone()),
                    "test-workspace",
                    windmill_common::jobs::JobPayload::FlowDependencies {
                        path: "f/dre/flow".to_owned(),
                        dedicated_worker: None,
                        // In newest versions we pass the current version to the djob
                        // version: 1443253234253454,
                        version,
                    },
                    windmill_queue::PushArgs { args: &args, extra: None },
                    "admin",
                    "admin@windmill.dev",
                    "admin".to_owned(),
                    Some("trigger.dependents.to.recompute.dependencies"),
                    // Schedule for now.
                    Some(chrono::Utc::now()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                    false,
                    None,
                    true,
                    Some("dependency".into()),
                    None,
                    None,
                    None,
                    None,
                    false,
                    None,
                    debounce_job_id_o,
                    None,
                    None,
                    None,
                )
                .await
                .unwrap();

                new_tx.commit().await.unwrap();

                job_uuid
            };

            // Push first
            job_ids.push(push_job(5, 1443253234253454, db.clone(), ["a", "b"], None).await);

            // Verify debounce_stale_data and debounce_key
            {
                let q = sqlx::query!("SELECT COUNT(*) FROM debounce_key")
                    .fetch_all(&db)
                    .await
                    .unwrap();

                // Should be single entry
                assert_eq!(q.len(), 1);
            }

            // Start the first one in the background
            let handle = {
                let mut completed = listen_for_completed_jobs(&db).await;
                let db2 = db.clone();
                tokio::spawn(async move {
                    in_test_worker(
                        &db2,
                        // sleep(Duration::from_secs(7)),
                        completed.next(), // Only wait for the single job. We are going to spawn another worker for second one.
                        port,
                    )
                    .await;
                })
            };

            // Wait for the job to be created and started
            // This way next job is not going to be consumed by the first one.
            sleep(Duration::from_secs(2)).await;

            // Push second
            job_ids.push(push_job(0, 1, db.clone(), ["b", "c"], None).await);

            // Wait for the second one to finish in separate worker.
            // in_test_worker(&db, completed.next(), port).await;
            in_test_worker(
                &db,
                async {
                    // First job will be pulled
                    completed.next().await;
                    // However since we have concurrency limit enabled it will get rescheduled by creation of new djob.
                    // So we have to wait for that one as well.
                    completed.next().await;
                },
                port,
            )
            .await;

            // Wait for the first one
            handle.await.unwrap();

            // Verify locks
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT jsonb_array_elements(value->'modules')->'value'->>'lock' AS lock FROM flow")
                    .fetch_all(&db)
                    .await
                    .unwrap(),
                    vec![
                        Some("# py: 3.11\nbottle==0.13.2".into()),
                        Some("# py: 3.11\nbottle==0.13.2\ntiny==0.1.3".into()),
                        Some("# py: 3.11\ntiny==0.1.3".into())
                    ]
                );
            }
            // Verify that we have expected outcome
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job",)
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                    2
                );

                assert_eq!(
                    sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_completed",)
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                    2
                );
                // Check that two jobs were executed sequentially
                assert!(sqlx::query_scalar!(
                    "
SELECT
    j1.completed_at < j2.started_at
FROM
    v2_job_completed j1,
    v2_job_completed j2
WHERE
    j1.id = $1  
    AND j2.id = $2",
                    job_ids[0],
                    job_ids[1],
                )
                .fetch_one(&db)
                .await
                .unwrap()
                .unwrap());
            }
            Ok(())
        }

        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_min_version_supports_debouncing(
            db: sqlx::Pool<sqlx::Postgres>,
        ) -> anyhow::Result<()> {
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            let (_client, port, _s) = init_client(db.clone()).await;
            let db = &db;

            crate::common::in_test_worker(
                db,
                async {
                    let job_template = crate::common::RunJob::from(
                        windmill_common::jobs::JobPayload::FlowDependencies {
                            path: "f/dre/flow".to_owned(),
                            dedicated_worker: None,
                            version: 1443253234253454,
                        },
                    )
                    .push_arg_scheduled_for_o(Some(chrono::Utc::now()))
                    .arg("triggered_by_relative_import", serde_json::json!(()));

                    // This will push to the top level worker
                    let debounce_job_id = job_template.clone().push(db).await;

                    // Will have space to run in parallel but in it's own worker
                    job_template
                        .push_arg_debounce_job_id_o(Some(debounce_job_id))
                        .run_until_complete(db, false, port)
                        .await;
                },
                port,
            )
            .await;

            // Verify there is not jobs running
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            // And there is only supposed to be one job.
            assert_eq!(
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM v2_job_completed WHERE status = 'success'"
                )
                .fetch_one(db)
                .await
                .unwrap()
                .unwrap(),
                1
            );

            Ok(())
        }
        // NOTE: Don't run in parallel with other tests
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        #[ignore]
        async fn test_min_version_does_not_support_debouncing(
            db: sqlx::Pool<sqlx::Postgres>,
        ) -> anyhow::Result<()> {
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = false;
            }

            let (_client, port, _s) = init_client(db.clone()).await;
            let db = &db;

            crate::common::in_test_worker(
                db,
                async {
                    let job_template = crate::common::RunJob::from(
                        windmill_common::jobs::JobPayload::FlowDependencies {
                            path: "f/dre/flow".to_owned(),
                            dedicated_worker: None,
                            version: 1443253234253454,
                        },
                    )
                    .push_arg_scheduled_for_o(Some(chrono::Utc::now()))
                    .arg("triggered_by_relative_import", serde_json::json!(()));

                    // This will push to the top level worker
                    let debounce_job_id = job_template.clone().push(db).await;

                    // Will have space to run in parallel but in it's own worker
                    job_template
                        .push_arg_debounce_job_id_o(Some(debounce_job_id))
                        .run_until_complete(db, false, port)
                        .await;
                },
                port,
            )
            .await;

            // Verify there is not jobs running
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            // There are supposed to be two jobs, since debouncing is disabled.
            assert_eq!(
                sqlx::query_scalar!(
                    "SELECT COUNT(*) FROM v2_job_completed WHERE status = 'success'"
                )
                .fetch_one(db)
                .await
                .unwrap()
                .unwrap(),
                2
            );

            Ok(())
        }
        // TODO:
        // test that update or create flow that should bypass debouncing
    }

    /// ## Testing for Apps
    /// For apps we are going to do similar tests that we did for flows
    mod apps {
        use crate::common::{in_test_worker, init_client, listen_for_completed_jobs};
        use crate::dependency_job_debouncing::trigger_djob_for;
        use std::time::Duration;
        use tokio::time::sleep;
        use tokio_stream::StreamExt;

        /// 1. LLF and RLF create two djobs for flow at the same and fall into single debounce
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_1(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }
            // This tests if debouncing and consolidation works.
            // Also makes sures that dependency job does not create new flow version

            let (client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            // Trigger both at the same time.
            // It will create two immediate dependency jobs
            {
                trigger_djob_for(
                    &client,
                    "f/dre_app/leaf_left",
                    "0000000000069CF8",
                    Some("#requirements:\n#bottle==0.13.2\ndef main():\npass".into()),
                )
                .await;

                trigger_djob_for(
                    &client,
                    "f/dre_app/leaf_right",
                    "0000000000069CFB",
                    Some("#requirements:\n#tiny==0.1.3\ndef main():\npass".into()),
                )
                .await;
            }

            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap(),
                2
            );

            // Spawn single worker.
            in_test_worker(
                &db,
                async {
                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre_app/leaf_left"
                    );

                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre_app/leaf_right"
                    );

                    // Verify there is only one queued job that is scheduled for atleast 3s ahead.
                    {
                        let q = sqlx::query_scalar!(
                            "SELECT (scheduled_for - created_at) FROM v2_job_queue"
                        )
                        .fetch_all(&db)
                        .await
                        .unwrap();

                        assert_eq!(1, q.len());
                        assert!(dbg!(q[0].unwrap().microseconds) > 2_000_000);
                    }

                    // Verify debounce_stale_data and debounce_key
                    {
                        let q = sqlx::query!(
                            "SELECT
                        dsd.to_relock,
                        dk.key
                    FROM debounce_key dk
                    JOIN debounce_stale_data dsd ON dk.job_id = dsd.job_id"
                        )
                        .fetch_all(&db)
                        .await
                        .unwrap();

                        // Should be single entry
                        assert!(q.len() == 1);

                        // This verifies that all nodes_to_relock are consolidated correctly
                        // AND there is no doublicats
                        assert_eq!(
                            q[0].to_relock.clone().unwrap(),
                            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
                        );

                        // Should be workspace specific and these specific tests cover only dependency job debouncing
                        assert_eq!(
                            q[0].key.clone(),
                            "test-workspace:f/dre_app/app:dependency".to_owned(),
                        );
                    }

                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre_app/app"
                    );
                },
                port,
            )
            .await;

            // Verify App states
            {
                let q = dbg!(sqlx::query_scalar!(
                    "SELECT versions FROM app WHERE path = 'f/dre_app/app'"
                )
                .fetch_one(&db)
                .await
                .unwrap());

                assert_eq!(2, q.len());

                // There is also supposed to be this amount of app_versions
                assert_eq!(
                    2,
                    sqlx::query_scalar!("SELECT COUNT(*) FROM app_version WHERE app_id = '2'")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
            }

            // Verify cleanup
            {
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from debounce_stale_data")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
            }

            Ok(())
        }

        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_left(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            use crate::common::RunJob;
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            // TODO: We don't care about timer. If there is no timer, it will be set automatically for djobs??
            let (_client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            let mut args = std::collections::HashMap::new();
            args.insert(
                "dbg_djob_sleep".to_owned(),
                // Execution should take this seconds
                windmill_common::worker::to_raw_value(&20),
            );
            args.insert(
                "triggered_by_relative_import".to_owned(),
                // Execution should take this seconds
                windmill_common::worker::to_raw_value(&()),
            );

            let (_flow_id, new_tx) = windmill_queue::push(
                &db,
                windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone()),
                "test-workspace",
                windmill_common::jobs::JobPayload::AppDependencies {
                    path: "f/dre_app/app".to_owned(),
                    version: 0,
                },
                windmill_queue::PushArgs { args: &args, extra: None },
                "admin",
                "admin@windmill.dev",
                "admin".to_owned(),
                Some("trigger.dependents.to.recompute.dependencies"),
                // Debounce period
                Some(chrono::Utc::now() + chrono::Duration::seconds(5)),
                None,
                None,
                None,
                None,
                None,
                false,
                false,
                None,
                true,
                Some("dependency".into()),
                None,
                None,
                None,
                None,
                false,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
            new_tx.commit().await.unwrap();

            // let mut handle = {
            //     let mut completed = listen_for_completed_jobs(&db).await;
            //     let db2 = db.clone();
            //     let uuid = flow_id.clone();
            //     tokio::spawn(async move {
            //         in_test_worker(
            //             &db2,
            //             // tokio::time::sleep(tokio::time::Duration::from_secs(60)),
            //             async move {
            //                 completed.find(&uuid).await;
            //             },
            //             port,
            //         )
            //         .await;
            //     })
            // };

            let db2 = db.clone();
            in_test_worker(
                &db2,
                async {
                    // This job should execute and then try to start another job that will get debounced.
                    RunJob::from(windmill_common::jobs::JobPayload::Dependencies {
                        path: "f/dre_app/leaf_right".to_owned(),
                        hash: 433403.into(),
                        language: windmill_common::scripts::ScriptLang::Python3,
                        dedicated_worker: None,
                    })
                    // .arg("dbg_djob_sleep", serde_json::json!(10))
                    .run_until_complete(&db, false, port)
                    .await;

                    RunJob::from(windmill_common::jobs::JobPayload::Dependencies {
                        path: "f/dre_app/leaf_left".to_owned(),
                        hash: 433400.into(),
                        language: windmill_common::scripts::ScriptLang::Python3,
                        dedicated_worker: None,
                    })
                    // So set it to this long
                    .arg("dbg_djob_sleep", serde_json::json!(10))
                    .run_until_complete(&db, false, port)
                    .await;

                    completed.next().await; // leaf_right
                    completed.next().await; // leaf_left
                    completed.next().await; // importer
                    completed.next().await; // importer
                },
                port,
            )
            .await;

            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            let r = sqlx::query_scalar!("SELECT runnable_id FROM v2_job ORDER BY created_at DESC")
                .fetch_all(&db)
                .await
                .unwrap();

            assert_eq!(r.len(), 4);
            assert!(r.contains(&Some(9)));
            assert!(r.contains(&Some(433400)));
            assert!(r.contains(&Some(433403)));
            assert!(r.contains(&Some(0)));

            // handle.await.unwrap();

            Ok(())
        }
        /// 2. Same as second test, however first app djob will take longer than second debounce.
        /// NOTE: This test should be ran in debug mode. In release it will not work properly.
        #[cfg(all(feature = "python", feature = "private"))]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_3(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            // This tests checks if concurrency limit works correcly and there is no race conditions.
            let (_client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            // At this point we should have two
            let mut job_ids = vec![];
            let push_job = |delay, db| async move {
                let mut args = std::collections::HashMap::new();
                args.insert(
                    "dbg_djob_sleep".to_owned(),
                    // First one will create delay for 5 seconds
                    // The second will have no delay at all.
                    windmill_common::worker::to_raw_value(&delay),
                );

                args.insert(
                    "triggered_by_relative_import".to_string(),
                    windmill_common::worker::to_raw_value(&()),
                );

                let (job_uuid, new_tx) = windmill_queue::push(
                    &db,
                    windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone()),
                    "test-workspace",
                    windmill_common::jobs::JobPayload::AppDependencies {
                        path: "f/dre_app/app".to_owned(),
                        // In newest versions we pass the current version to the djob
                        version: 0,
                    },
                    windmill_queue::PushArgs { args: &args, extra: None },
                    "admin",
                    "admin@windmill.dev",
                    "admin".to_owned(),
                    Some("trigger.dependents.to.recompute.dependencies"),
                    // Schedule for now.
                    Some(chrono::Utc::now()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                    false,
                    None,
                    true,
                    Some("dependency".into()),
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
                .unwrap();

                new_tx.commit().await.unwrap();

                job_uuid
            };

            // TODO: Verify concurrency key.
            // Push first
            job_ids.push(push_job(5, db.clone()).await);

            // Verify debounce_stale_data and debounce_key
            {
                let q = sqlx::query!("SELECT COUNT(*) FROM debounce_key")
                    .fetch_all(&db)
                    .await
                    .unwrap();

                // Should be single entry
                assert_eq!(q.len(), 1);
            }

            // Start the first one in the background
            let handle = {
                let mut completed = listen_for_completed_jobs(&db).await;
                let db2 = db.clone();
                tokio::spawn(async move {
                    in_test_worker(
                        &db2,
                        // sleep(Duration::from_secs(7)),
                        completed.next(), // Only wait for the single job. We are going to spawn another worker for second one.
                        port,
                    )
                    .await;
                })
            };

            // Wait for the job to be created and started
            // This way next job is not going to be consumed by the first one.
            sleep(Duration::from_secs(2)).await;

            // Push second
            job_ids.push(push_job(0, db.clone()).await);

            // Wait for the second one to finish in separate worker.
            // in_test_worker(&db, completed.next(), port).await;
            in_test_worker(
                &db,
                async {
                    // First job will be pulled
                    completed.next().await;
                    // However since we have concurrency limit enabled it will get rescheduled by creation of new djob.
                    // So we have to wait for that one as well.
                    completed.next().await;
                },
                port,
            )
            .await;

            // Wait for the first one
            handle.await.unwrap();

            // Verify that we have expected outcome
            {
                assert_eq!(
                    sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job",)
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                    2
                );

                assert_eq!(
                    sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_completed",)
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                    2
                );
                // Check that two jobs were executed sequentially
                assert!(sqlx::query_scalar!(
                    "
SELECT
    j1.completed_at < j2.started_at
FROM
    v2_job_completed j1,
    v2_job_completed j2
WHERE
    j1.id = $1  
    AND j2.id = $2",
                    job_ids[0],
                    job_ids[1],
                )
                .fetch_one(&db)
                .await
                .unwrap()
                .unwrap());
            }
            Ok(())
        }
    }

    // TODO: Test debounce reassignment works

    /// ## Testing for Scripts
    mod scripts {
        use crate::common::{in_test_worker, init_client, listen_for_completed_jobs};
        use crate::dependency_job_debouncing::trigger_djob_for;
        use std::time::Duration;
        use tokio::time::sleep;
        use tokio_stream::StreamExt;

        /// 1. LLF and RLF create two djobs for flow at the same and fall into single debounce
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        // TODO: Same test_but script fails.
        async fn test_1(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }
            // This tests if debouncing and consolidation works.
            // Also makes sures that dependency job does not create new flow version
            let (client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            // Verify lock is empty
            {
                assert_eq!(
                    sqlx::query_scalar!(
                        "SELECT lock FROM script WHERE path = 'f/dre_script/script'"
                    )
                    .fetch_one(&db)
                    .await
                    .unwrap(),
                    Some("".into())
                );
            }

            // Trigger both at the same time.
            {
                trigger_djob_for(
                    &client,
                    "f/dre_script/leaf_left",
                    "0000000000082398",
                    Some("#requirements:\n#bottle==0.13.2\ndef main():\npass".into()),
                )
                .await;
                trigger_djob_for(
                    &client,
                    "f/dre_script/leaf_right",
                    "000000000008239B",
                    Some("#requirements:\n#tiny==0.1.3\ndef main():\npass".into()),
                )
                .await;
            }

            sleep(Duration::from_secs(1)).await;

            in_test_worker(
                &db,
                async {
                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre_script/leaf_left"
                    );

                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre_script/leaf_right"
                    );

                    // handle.await.unwrap();

                    // Let jobs propagate

                    tokio::select!(
                        _ = async {
                            while sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue WHERE running = false")
                                .fetch_one(&db)
                                .await
                                .unwrap()
                                .unwrap()
                                == 0
                            {
                                sleep(Duration::from_secs(1)).await;
                            }
                        } => {},
                        _ = sleep(Duration::from_secs(60)) => { panic!("Timeout") }
                    );
                    // Verify there is only one queued job that is scheduled for atleast 3s ahead.
                    {
                        for r in
                            sqlx::query_scalar!("SELECT id FROM v2_job_queue WHERE running = false")
                                .fetch_all(&db)
                                .await
                                .unwrap()
                        {
                            dbg!(
                                sqlx::query!("SELECT runnable_path FROM v2_job WHERE id = $1", r)
                                    .fetch_all(&db)
                                    .await
                                    .unwrap()
                            );
                        }
                        for r in sqlx::query_scalar!("SELECT id FROM v2_job_completed")
                            .fetch_all(&db)
                            .await
                            .unwrap()
                        {
                            dbg!(
                                sqlx::query!("SELECT runnable_path FROM v2_job WHERE id = $1", r)
                                    .fetch_all(&db)
                                    .await
                                    .unwrap()
                            );
                        }

                        dbg!(sqlx::query!("SELECT runnable_path FROM v2_job")
                            .fetch_all(&db)
                            .await
                            .unwrap());

                        let q = sqlx::query_scalar!(
                            "SELECT (scheduled_for - created_at) FROM v2_job_queue WHERE running = false"
                        )
                        .fetch_all(&db)
                        .await
                        .unwrap();

                        assert_eq!(1, q.len());
                        assert!(dbg!(q[0].unwrap().microseconds) > 1_000_000 /* 1 second */);
                    }

                    // Verify debounce_stale_data and debounce_key
                    {
                        let q = sqlx::query_scalar!("SELECT key FROM debounce_key")
                            .fetch_all(&db)
                            .await
                            .unwrap();

                        assert_eq!(q.len(), 1);

                        assert_eq!(
                            q[0].clone(),
                            "test-workspace:f/dre_script/script:dependency".to_owned(),
                        );

                        // Stale data is empty for scripts
                        assert_eq!(
                            sqlx::query_scalar!("SELECT COUNT(*) FROM debounce_stale_data")
                                .fetch_one(&db)
                                .await
                                .unwrap()
                                .unwrap(),
                            0
                        );
                    }

                    // Wait until debounce delay is complete
                    // sleep(Duration::from_secs(6)).await;

                    assert_eq!(
                        &sqlx::query_scalar!(
                            "SELECT runnable_path FROM v2_job WHERE id = $1",
                            completed.next().await.unwrap()
                        )
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap(),
                        "f/dre_script/script"
                    );
                },
                port,
            )
            .await;

            // completed.next().await.unwrap();

            // Verify
            {
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from v2_job_queue")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
                assert_eq!(
                    vec![533404],
                    dbg!(sqlx::query_scalar!(
                        "SELECT hash FROM script WHERE path = 'f/dre_script/script' AND archived = true"
                    )
                    .fetch_all(&db)
                    .await
                    .unwrap())
                );

                assert_ne!(
                    533404,
                    sqlx::query_scalar!(
                        "SELECT hash FROM script WHERE path = 'f/dre_script/script' AND archived = false"
                    )
                    .fetch_one(&db)
                    .await
                    .unwrap()
                );

                assert_eq!(
                    vec![533404],
                    sqlx::query_scalar!(
                        "SELECT parent_hashes FROM script WHERE path = 'f/dre_script/script' AND archived = false"
                    )
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap()
                );
            }

            // Verify cleanup
            {
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
                assert_eq!(
                    0,
                    sqlx::query_scalar!("SELECT COUNT(*) from debounce_stale_data")
                        .fetch_one(&db)
                        .await
                        .unwrap()
                        .unwrap()
                );
            }

            // handle.await.unwrap();
            Ok(())
        }

        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        async fn test_left(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            use crate::common::RunJob;
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            // TODO: We don't care about timer. If there is no timer, it will be set automatically for djobs??
            let (_client, port, _s) = init_client(db.clone()).await;
            let mut completed = listen_for_completed_jobs(&db).await;

            // Trigger both at the same time.
            {
                let mut args = std::collections::HashMap::new();
                args.insert(
                    "dbg_djob_sleep".to_owned(),
                    // Execution should take this seconds
                    windmill_common::worker::to_raw_value(&20),
                );

                args.insert(
                    "triggered_by_relative_import".to_owned(),
                    // Execution should take this seconds
                    windmill_common::worker::to_raw_value(&()),
                );

                let (_flow_id, new_tx) = windmill_queue::push(
                    &db,
                    windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone()),
                    "test-workspace",
                    windmill_common::jobs::JobPayload::Dependencies {
                        path: "f/dre_script/script".to_owned(),
                        dedicated_worker: None,
                        language: windmill_common::scripts::ScriptLang::Python3,
                        hash: 533404.into(),
                    },
                    windmill_queue::PushArgs { args: &args, extra: None },
                    "admin",
                    "admin@windmill.dev",
                    "admin".to_owned(),
                    Some("trigger.dependents.to.recompute.dependencies"),
                    // Debounce period
                    Some(chrono::Utc::now() + chrono::Duration::seconds(5)),
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                    false,
                    None,
                    true,
                    Some("dependency".into()),
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
                .unwrap();
                new_tx.commit().await.unwrap();

                let db2 = db.clone();
                in_test_worker(
                    &db2,
                    async {
                        // This job should execute and then try to start another job that will get debounced.
                        RunJob::from(windmill_common::jobs::JobPayload::Dependencies {
                            path: "f/dre_script/leaf_right".to_owned(),
                            hash: 533403.into(),
                            language: windmill_common::scripts::ScriptLang::Python3,
                            dedicated_worker: None,
                        })
                        .run_until_complete(&db, false, port)
                        .await;

                        // This one is supposed to be started after flow djob has debounced and started but haven't finished yet.
                        RunJob::from(windmill_common::jobs::JobPayload::Dependencies {
                            path: "f/dre_script/leaf_left".to_owned(),
                            hash: 533400.into(),
                            language: windmill_common::scripts::ScriptLang::Python3,
                            dedicated_worker: None,
                        })
                        // So set it to this long
                        .arg("dbg_djob_sleep", serde_json::json!(10))
                        .run_until_complete(&db, false, port)
                        .await;

                        completed.next().await; // leaf_right
                        completed.next().await; // leaf_left
                        completed.next().await; // importer
                        completed.next().await; // importer
                    },
                    port,
                )
                .await;
            }

            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(&db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            let r = sqlx::query_scalar!("SELECT runnable_id FROM v2_job ORDER BY created_at DESC")
                .fetch_all(&db)
                .await
                .unwrap();

            dbg!(&r);
            assert_eq!(r.len(), 4);
            assert!(r.contains(&Some(-221349019907577876)));
            assert!(r.contains(&Some(533400)));
            assert!(r.contains(&Some(533403)));
            assert!(r.contains(&Some(533404)));

            Ok(())
        }

        //         // TODO: we don't need scripts to have concurrency limit
        //         /// 3. Same as second test, however first app djob will take longer than second debounce.
        //         /// NOTE: This test should be ran in debug mode. In release it will not work properly.
        //         #[cfg(all(feature = "python", feature = "private"))]
        //         #[sqlx::test(fixtures("base", "djob_debouncing"))]
        //         async fn test_3(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
        //             // This tests checks if concurrency limit works correcly and there is no race conditions.
        //             let (client, port, _s) = init_client(db.clone()).await;
        //             let mut completed = listen_for_completed_jobs(&db).await;

        //             // At this point we should have two
        //             let mut job_ids = vec![];
        //             let push_job = |delay, db| async move {
        //                 let mut args = std::collections::HashMap::new();
        //                 args.insert(
        //                     "dbg_djob_sleep".to_owned(),
        //                     // First one will create delay for 5 seconds
        //                     // The second will have no delay at all.
        //                     windmill_common::worker::to_raw_value(&delay),
        //                 );

        //                 args.insert(
        //                     "triggered_by_relative_import".to_string(),
        //                     windmill_common::worker::to_raw_value(&()),
        //                 );

        //                 let (job_uuid, new_tx) = windmill_queue::push(
        //                     &db,
        //                     windmill_queue::PushIsolationLevel::IsolatedRoot(db.clone()),
        //                     "test-workspace",
        //                     windmill_common::jobs::JobPayload::Dependencies {
        //                         path: "f/dre_script/script".to_owned(),
        //                         language: windmill_common::scripts::ScriptLang::Python3,
        //                         dedicated_worker: None,
        //                         hash: windmill_common::scripts::ScriptHash(533404),
        //                     },
        //                     windmill_queue::PushArgs { args: &args, extra: None },
        //                     "admin",
        //                     "admin@windmill.dev",
        //                     "admin".to_owned(),
        //                     Some("trigger.dependents.to.recompute.dependencies"),
        //                     // Schedule for now.
        //                     Some(chrono::Utc::now()),
        //                     None,
        //                     None,
        //                     None,
        //                     None,
        //                     None,
        //                     false,
        //                     false,
        //                     None,
        //                     true,
        //                     Some("dependency".into()),
        //                     None,
        //                     None,
        //                     None,
        //                     None,
        //                     false,
        //                     None,
        //                     None,
        //                     None,
        //                     None,
        //                 )
        //                 .await
        //                 .unwrap();

        //                 new_tx.commit().await.unwrap();

        //                 job_uuid
        //             };

        //             // Push first
        //             job_ids.push(push_job(5, db.clone()).await);
        //             sleep(Duration::from_millis(300)).await;

        //             // Verify debounce_stale_data and debounce_key
        //             {
        //                 let q = sqlx::query_scalar!("SELECT key FROM debounce_key")
        //                     .fetch_all(&db)
        //                     .await
        //                     .unwrap();

        //                 assert_eq!(q.len(), 1);

        //                 assert_eq!(
        //                     q[0].clone(),
        //                     "test-workspace:f/dre_script/script:dependency".to_owned(),
        //                 );

        //                 // Stale data is empty for scripts
        //                 assert_eq!(
        //                     sqlx::query_scalar!("SELECT COUNT(*) FROM debounce_stale_data")
        //                         .fetch_one(&db)
        //                         .await
        //                         .unwrap()
        //                         .unwrap(),
        //                     0
        //                 );
        //             }

        //             // Start the first one in the background
        //             let handle = {
        //                 let mut completed = listen_for_completed_jobs(&db).await;
        //                 let db2 = db.clone();
        //                 tokio::spawn(async move {
        //                     in_test_worker(
        //                         &db2,
        //                         // sleep(Duration::from_secs(7)),
        //                         completed.next(), // Only wait for the single job. We are going to spawn another worker for second one.
        //                         port,
        //                     )
        //                     .await;
        //                 })
        //             };

        //             // Wait for the job to be created and started
        //             // This way next job is not going to be consumed by the first one.
        //             sleep(Duration::from_secs(1)).await;

        //             // Push second
        //             job_ids.push(push_job(0, db.clone()).await);

        //             // Wait for the second one to finish in separate worker.
        //             in_test_worker(
        //                 &db,
        //                 async {
        //                     // First job will be pulled
        //                     completed.next().await;
        //                     // However since we have concurrency limit enabled it will get rescheduled by creation of new djob.
        //                     // So we have to wait for that one as well.
        //                     completed.next().await;
        //                 },
        //                 port,
        //             )
        //             .await;

        //             // Wait for the first one
        //             handle.await.unwrap();

        //             // Verify that we have expected outcome
        //             {
        //                 assert_eq!(
        //                     sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue",)
        //                         .fetch_one(&db)
        //                         .await
        //                         .unwrap()
        //                         .unwrap(),
        //                     0
        //                 );
        //                 // Verify lock
        //                 {
        //                     assert_eq!(
        //                         sqlx::query_scalar!(
        //                             "SELECT lock FROM script WHERE path = 'f/dre_script/script'"
        //                         )
        //                         .fetch_one(&db)
        //                         .await
        //                         .unwrap(),
        //                         Some("# py: 3.11\nbottle==0.13.2\ntiny==0.1.3".into())
        //                     );
        //                 }
        //                 assert_eq!(
        //                     sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job",)
        //                         .fetch_one(&db)
        //                         .await
        //                         .unwrap()
        //                         .unwrap(),
        //                     2
        //                 );

        //                 assert_eq!(
        //                     sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_completed",)
        //                         .fetch_one(&db)
        //                         .await
        //                         .unwrap()
        //                         .unwrap(),
        //                     2
        //                 );
        //                 // Check that two jobs were executed sequentially
        //                 assert!(sqlx::query_scalar!(
        //                     "
        // SELECT
        //     j1.completed_at < j2.started_at
        // FROM
        //     v2_job_completed j1,
        //     v2_job_completed j2
        // WHERE
        //     j1.id = $1
        //     AND j2.id = $2",
        //                     job_ids[0],
        //                     job_ids[1],
        //                 )
        //                 .fetch_one(&db)
        //                 .await
        //                 .unwrap()
        //                 .unwrap());
        //             }
        //             Ok(())
        //         }
    }
    // TODO: Test git sync
}
#[cfg(feature = "test_job_debouncing")]
mod normal_job_debouncing {
    mod scripts {
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "job_debouncing"))]
        async fn test_default_debounce_key(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            use serde_json::json;
            use windmill_common::scripts::ScriptHash;

            use crate::common::{in_test_worker, init_client, listen_for_completed_jobs, RunJob};
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            let (_client, port, _s) = init_client(db.clone()).await;
            let db = &db;

            in_test_worker(
                db,
                async {
                    // This job should execute and then try to start another job that will get debounced.
                    RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                        hash: ScriptHash(533400),
                        path: "f/scripts/script_1".into(),
                        // Do not supply with custom debounce key.
                        // We will test if the debounce_key is created correctly.
                        custom_debounce_key: None,
                        debounce_delay_s: Some(2),
                        custom_concurrency_key: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                        cache_ttl: None,
                        dedicated_worker: None,
                        language: windmill_common::scripts::ScriptLang::Python3,
                        priority: None,
                        apply_preprocessor: false,
                    })
                    .arg("x", json!("ey"))
                    .arg("b", json!("33"))
                    // Start another worker, so we have two workers at the same time.
                    // We don't know which will execute the job, but we do know that if the job is executed, this worker will exit.
                    .run_until_complete_with(db, false, port, |id| async move {

                        // Verify debounce_key
                        assert_eq!(
                            sqlx::query_scalar!(
                                "SELECT key FROM debounce_key WHERE job_id = $1",
                                id.clone()
                            )
                            .fetch_one(db)
                            .await
                            .unwrap(),
                                "test-workspace/script/f/scripts/script_1#args:\"33\":\"ey\"".to_owned()
                        );

                        // Verify it is scheduled for future and not now.
                        {
                            assert!(
                                dbg!(
                                    sqlx::query_scalar!(
                                        "SELECT (scheduled_for - created_at) FROM v2_job_queue WHERE running = false"
                                    )
                                    .fetch_one(db)
                                    .await
                                    .unwrap()
                                    .unwrap()
                                    .microseconds
                                ) > 1_000_000 /* 1 second */
                            );
                        }

                        // Start another job.
                        RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                            hash: ScriptHash(533400),
                            path: "f/scripts/script_1".into(),
                            // Do not supply with custom debounce key.
                            // We will test if the debounce_key is created correctly.
                            custom_debounce_key: None,
                            debounce_delay_s: Some(2),
                            custom_concurrency_key: None,
                            concurrent_limit: None,
                            concurrency_time_window_s: None,
                            cache_ttl: None,
                            dedicated_worker: None,
                            language: windmill_common::scripts::ScriptLang::Python3,
                            priority: None,
                            apply_preprocessor: false,
                        })
                        .arg("x", json!("ey"))
                        .arg("b", json!("33"))
                        // But we only push it, one of the jobs should be debounced.
                        .push(db)
                        .await;
                    })
                    .await;
                },
                port,
            )
            .await;

            // Verify there is not jobs running
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            // And there is only supposed to be one job.
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                1
            );

            // Verify debounce key clean up
            assert_eq!(
                0,
                sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap()
            );

            Ok(())
        }

        #[cfg(all(feature = "python", feature = "agent_worker_server"))]
        #[sqlx::test(fixtures("base", "job_debouncing"))]
        async fn test_default_debounce_key_agent_wk(
            db: sqlx::Pool<sqlx::Postgres>,
        ) -> anyhow::Result<()> {
            use serde_json::json;
            use windmill_common::scripts::ScriptHash;

            use crate::common::{init_client_agent_mode, RunJob};
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            let (_client, port, _s) = init_client_agent_mode(db.clone()).await;
            let db = &db;
            RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                hash: ScriptHash(533400),
                path: "f/scripts/script_1".into(),
                // Do not supply with custom debounce key.
                // We will test if the debounce_key is created correctly.
                custom_debounce_key: None,
                debounce_delay_s: Some(2),
                custom_concurrency_key: None,
                concurrent_limit: None,
                concurrency_time_window_s: None,
                cache_ttl: None,
                dedicated_worker: None,
                language: windmill_common::scripts::ScriptLang::Python3,
                priority: None,
                apply_preprocessor: false,
            })
            .arg("x", json!("ey"))
            .arg("b", json!("33"))
            .run_until_complete(db, true, port)
            .await;

            // Verify there is not jobs running
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            // And there is only supposed to be one job.
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                1
            );

            // And that job execute successfully
            assert_eq!(
                sqlx::query_scalar!("SELECT status::text FROM v2_job_completed")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                "success"
            );

            // Verify debounce key clean up
            assert_eq!(
                0,
                sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap()
            );

            Ok(())
        }

        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "job_debouncing"))]
        async fn test_custom_debounce_key(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            use serde_json::json;
            use windmill_common::scripts::ScriptHash;

            use crate::common::{in_test_worker, init_client, listen_for_completed_jobs, RunJob};
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            let (_client, port, _s) = init_client(db.clone()).await;
            let completed = listen_for_completed_jobs(&db).await;
            let db = &db;

            in_test_worker(
                db,
                async {
                    // This job should execute and then try to start another job that will get debounced.
                    RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                        hash: ScriptHash(533400),
                        path: "f/scripts/script_1".into(),
                        // Do not supply with custom debounce key.
                        // We will test if the debounce_key is created correctly.
                        custom_debounce_key: Some("$workspace:my-custom-debounce-key:$args[x]".to_owned()),
                        debounce_delay_s: Some(2),
                        custom_concurrency_key: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                        cache_ttl: None,
                        dedicated_worker: None,
                        language: windmill_common::scripts::ScriptLang::Python3,
                        priority: None,
                        apply_preprocessor: false,
                    })
                    .arg("x", json!("ey"))
                    .arg("b", json!("1")) // 1
                    // Start another worker, so we have two workers at the same time.
                    // We don't know which will execute the job, but we do know that if the job is executed, this worker will exit.
                    .run_until_complete_with(db, false, port, |id| async move {

                        // Verify debounce_key
                        assert_eq!(
                            sqlx::query_scalar!(
                                "SELECT key FROM debounce_key WHERE job_id = $1",
                                id.clone()
                            )
                            .fetch_one(db)
                            .await
                            .unwrap(),
                                "test-workspace:my-custom-debounce-key:ey".to_owned()
                        );

                        // Verify it is scheduled for future and not now.
                        {
                            assert!(
                                dbg!(
                                    sqlx::query_scalar!(
                                        "SELECT (scheduled_for - created_at) FROM v2_job_queue WHERE running = false"
                                    )
                                    .fetch_one(db)
                                    .await
                                    .unwrap()
                                    .unwrap()
                                    .microseconds
                                ) > 1_000_000 /* 1 second */
                            );
                        }

                        // Start another job.
                        RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                            hash: ScriptHash(533400),
                            path: "f/scripts/script_1".into(),
                            custom_debounce_key: Some("$workspace:my-custom-debounce-key:$args[x]".to_owned()),
                            debounce_delay_s: Some(2),
                            custom_concurrency_key: None,
                            concurrent_limit: None,
                            concurrency_time_window_s: None,
                            cache_ttl: None,
                            dedicated_worker: None,
                            language: windmill_common::scripts::ScriptLang::Python3,
                            priority: None,
                            apply_preprocessor: false,
                        })
                        .arg("x", json!("ey"))
                        // We will pass different argument. but it should still get debounced.
                        .arg("b", json!("2")) // 2
                        // But we only push it, one of the jobs should be debounced.
                        .push(db)
                        .await;
                    })
                    .await;
                },
                port,
            )
            .await;

            // Verify there is not jobs running
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            // And there is only supposed to be one job.
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                1
            );

            // And that job execute successfully
            assert_eq!(
                sqlx::query_scalar!("SELECT status::text FROM v2_job_completed")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                "success"
            );

            // Verify debounce key clean up
            assert_eq!(
                0,
                sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap()
            );

            Ok(())
        }

        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "job_debouncing"))]
        async fn test_no_debounce(db: sqlx::Pool<sqlx::Postgres>) -> anyhow::Result<()> {
            use serde_json::json;
            use windmill_common::scripts::ScriptHash;

            use crate::common::{in_test_worker, init_client, listen_for_completed_jobs, RunJob};
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            let (_client, port, _s) = init_client(db.clone()).await;
            let completed = listen_for_completed_jobs(&db).await;
            let db = &db;

            // different args
            in_test_worker(
                db,
                async {
                    // This job should execute and then try to start another job that will get debounced.
                    RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                        hash: ScriptHash(533400),
                        path: "f/scripts/script_1".into(),
                        // Do not supply with custom debounce key.
                        // We will test if the debounce_key is created correctly.
                        custom_debounce_key: None,
                        debounce_delay_s: Some(2),
                        custom_concurrency_key: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                        cache_ttl: None,
                        dedicated_worker: None,
                        language: windmill_common::scripts::ScriptLang::Python3,
                        priority: None,
                        apply_preprocessor: false,
                    })
                    .arg("x", json!("ey"))
                    .arg("b", json!("33"))
                    // Start another worker, so we have two workers at the same time.
                    // We don't know which will execute the job, but we do know that if the job is executed, this worker will exit.
                    .run_until_complete_with(db, false, port, |_id| async move {
                        // Start another job.
                        RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                            hash: ScriptHash(533400),
                            path: "f/scripts/script_1".into(),
                            // Do not supply with custom debounce key.
                            // We will test if the debounce_key is created correctly.
                            custom_debounce_key: None,
                            debounce_delay_s: Some(2),
                            custom_concurrency_key: None,
                            concurrent_limit: None,
                            concurrency_time_window_s: None,
                            cache_ttl: None,
                            dedicated_worker: None,
                            language: windmill_common::scripts::ScriptLang::Python3,
                            priority: None,
                            apply_preprocessor: false,
                        })
                        // Different args.
                        .arg("x", json!("ey"))
                        .arg("b", json!("34")) // Different arg
                        .push(db)
                        .await;
                    })
                    .await;
                },
                port,
            )
            .await;

            // no debounce delay on second
            in_test_worker(
                db,
                async {
                    // This job should execute and then try to start another job that will get debounced.
                    RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                        hash: ScriptHash(533400),
                        path: "f/scripts/script_1".into(),
                        // Do not supply with custom debounce key.
                        // We will test if the debounce_key is created correctly.
                        custom_debounce_key: None,
                        debounce_delay_s: Some(2),
                        custom_concurrency_key: None,
                        concurrent_limit: None,
                        concurrency_time_window_s: None,
                        cache_ttl: None,
                        dedicated_worker: None,
                        language: windmill_common::scripts::ScriptLang::Python3,
                        priority: None,
                        apply_preprocessor: false,
                    })
                    .arg("x", json!("ey"))
                    .arg("b", json!("33"))
                    // Start another worker, so we have two workers at the same time.
                    // We don't know which will execute the job, but we do know that if the job is executed, this worker will exit.
                    .run_until_complete_with(db, false, port, |_id| async move {
                        // Start another job.
                        RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                            hash: ScriptHash(533400),
                            path: "f/scripts/script_1".into(),
                            custom_debounce_key: None,
                            debounce_delay_s: None, // Set to none to skip debouncing
                            custom_concurrency_key: None,
                            concurrent_limit: None,
                            concurrency_time_window_s: None,
                            cache_ttl: None,
                            dedicated_worker: None,
                            language: windmill_common::scripts::ScriptLang::Python3,
                            priority: None,
                            apply_preprocessor: false,
                        })
                        .arg("x", json!("ey"))
                        .arg("b", json!("33"))
                        .push(db)
                        .await;
                    })
                    .await;
                },
                port,
            )
            .await;

            // Verify there is not jobs running
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            // And there is supposed to be four jobs and no debouncing.
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                4
            );

            // Verify debounce key clean up
            assert_eq!(
                0,
                sqlx::query_scalar!("SELECT COUNT(*) from debounce_key")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap()
            );

            Ok(())
        }
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "djob_debouncing"))]
        #[ignore = "modifies global env variable that is used by other tests"]
        async fn test_min_version_does_not_support_debouncing(
            db: sqlx::Pool<sqlx::Postgres>,
        ) -> anyhow::Result<()> {
            use serde_json::json;
            use windmill_common::scripts::ScriptHash;

            use crate::common::{in_test_worker, init_client, listen_for_completed_jobs, RunJob};

            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = false;
            }

            let (_client, port, _s) = init_client(db.clone()).await;
            let db = &db;

            crate::common::in_test_worker(
                db,
                async {
                    let job_template =
                        RunJob::from(windmill_common::jobs::JobPayload::ScriptHash {
                            hash: ScriptHash(533400),
                            path: "f/scripts/script_1".into(),
                            custom_debounce_key: None,
                            debounce_delay_s: None, // Set to none to skip debouncing
                            custom_concurrency_key: None,
                            concurrent_limit: None,
                            concurrency_time_window_s: None,
                            cache_ttl: None,
                            dedicated_worker: None,
                            language: windmill_common::scripts::ScriptLang::Python3,
                            priority: None,
                            apply_preprocessor: false,
                        });

                    // This will push to the top level worker
                    job_template.clone().push(db).await;

                    // Will have space to run in parallel but in it's own worker
                    job_template.run_until_complete(db, false, port).await;
                },
                port,
            )
            .await;

            // Verify there is not jobs running
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );

            // There are supposed to be two jobs, since debouncing is disabled.
            assert_eq!(
                sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
                    .fetch_one(db)
                    .await
                    .unwrap()
                    .unwrap(),
                2
            );

            Ok(())
        }
    }

    mod flows {
        #[cfg(feature = "python")]
        #[sqlx::test(fixtures("base", "job_debouncing"))]
        async fn test_different_kinds_top_level(
            db: sqlx::Pool<sqlx::Postgres>,
        ) -> anyhow::Result<()> {
            use crate::common::{init_client, listen_for_completed_jobs, RunJob};
            use serde_json::json;
            {
                let mut mvsd = windmill_common::worker::MIN_VERSION_SUPPORTS_DEBOUNCING
                    .write()
                    .await;
                *mvsd = true;
            }

            let (_client, port, _s) = init_client(db.clone()).await;
            let db = &db;

            // We want to run this for all tables related to flow be created.
            RunJob::from(windmill_common::jobs::JobPayload::FlowDependencies {
                path: "f/flows/flow".into(),
                dedicated_worker: None,
                version: 1443253234253454,
            })
            .run_until_complete(db, false, port)
            .await;

            dbg!(sqlx::query!("SELECT * FROM flow_node",)
                .fetch_all(db)
                .await
                .unwrap());

            let (j1, j2, j3) = tokio::join!(
                RunJob::from(windmill_common::jobs::JobPayload::Flow {
                    version: 1443253234253454,
                    path: "f/flows/flow".into(),
                    dedicated_worker: None,
                    apply_preprocessor: false,
                })
                .push(db),
                RunJob::from(windmill_common::jobs::JobPayload::SingleStepFlow {
                    hash: None,
                    path: "f/flows/flow".into(),
                    custom_debounce_key: None,
                    debounce_delay_s: Some(2),
                    custom_concurrency_key: None,
                    concurrent_limit: None,
                    concurrency_time_window_s: None,
                    flow_version: Some(1443253234253454),
                    args: std::collections::HashMap::new(),
                    retry: None,
                    error_handler_path: None,
                    error_handler_args: None,
                    skip_handler: None,
                    cache_ttl: None,
                    priority: None,
                    tag_override: None,
                    trigger_path: None,
                    apply_preprocessor: false,
                })
                .push(db),
                RunJob::from(windmill_common::jobs::JobPayload::RawFlow {
                    value: windmill_common::flows::FlowValue {
                        debounce_delay_s: Some(2),
                        modules: vec![windmill_common::flows::FlowModule {
                            id: "a".into(),
                            value: windmill_common::worker::to_raw_value(&json!({
                                "lock": "# py: 3.11\n",
                                "type": "rawscript",
                                "assets": [],
                                "content": "def main(x: str, y: str):\n    return x",
                                "language": "python3",
                                "debounce_delay_s": 15,
                                "input_transforms": {
                                    "x": {
                                        "type": "static",
                                        "value": ""
                                    },
                                    "y": {
                                        "type": "static",
                                        "value": ""
                                    }
                                }
                            })),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                    path: Some("f/flows/flow".into()),
                    restarted_from: None,
                })
                .push(db),
                // RunJob::from(windmill_common::jobs::JobPayload::Code(
                //     windmill_common::jobs::RawCode {
                //         content: "
                // def main(n: int):
                //     pass
                //                                     "
                //         .into(),
                //         path: Some("f/flows/flow".into()),
                //         hash: None,
                //         language: windmill_common::scripts::ScriptLang::Python3,
                //         custom_debounce_key: None,
                //         debounce_delay_s: Some(2),
                //         ..Default::default()
                //     },
                // ))
                // .push(db)
            );

            assert_eq!(j1, j2);
            assert_eq!(j1, j3);
            // assert_eq!(j1, j4);
            Ok(())
        }
    }

    // TODO(ALL):
    // - Check if all jobs were sucessfull.
    //
    // TODO:
    // - [x] FlowNode (Script)
    // - [x] FlowNode (Flow) - has no debouncing nor concurrency limits
    // - [x] RawCode (Flow as code)
    // - [x] RawFlow
    // - [x] Flow
    // - [x] FlowScript
    //
    // TODO(imperatively):
    // - [x] Creation of flow
    //   - [x] Check entire flow
    //   - [x] Check it's inline scripts
    // - [x] Creation of script
    //
    // TODO: [x] Agent workers. (and tests)
    // TODO: [x] Backwards compat (and tests)
    // TODO: [x] Concurrency limit is disabled if preprocessor is enabled. Investigate.
    // TODO: [x] Last resort - monitor.rs to clean up debounce_keys
    // TODO: [x] Catch debounce values by server if debouncing is disabled. (and tests)
}
