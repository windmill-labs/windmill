#[cfg(all(feature = "python", feature = "private"))]
mod ci_tests {
    use std::collections::HashMap;

    use sqlx::{Pool, Postgres};
    use tokio_stream::StreamExt;

    use windmill_api_client::types::{NewScript, ScriptLang};
    use windmill_test_utils::{in_test_worker, init_client, listen_for_completed_jobs};

    fn quick_ns(content: &str, path: &str, parent_hash: Option<String>) -> NewScript {
        NewScript {
            draft_only: None,
            content: content.into(),
            language: ScriptLang::Python3,
            lock: None,
            parent_hash,
            path: path.into(),
            concurrent_limit: None,
            concurrency_time_window_s: None,
            cache_ttl: None,
            dedicated_worker: None,
            description: "".to_string(),
            envs: vec![],
            is_template: None,
            kind: None,
            summary: "".to_string(),
            tag: None,
            schema: HashMap::new(),
            ws_error_handler_muted: Some(false),
            priority: None,
            delete_after_secs: None,
            timeout: None,
            restart_unless_cancelled: None,
            deployment_message: None,
            concurrency_key: None,
            visible_to_runner_only: None,
            auto_kind: None,
            codebase: None,
            has_preprocessor: None,
            on_behalf_of_email: None,
            assets: vec![],
            modules: None,
        }
    }

    /// Test 2: Deploying a script automatically triggers CI test jobs for test scripts
    /// that reference it via the `# test:` annotation.
    #[sqlx::test(fixtures("base"))]
    async fn test_ci_test_trigger_on_deploy(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, port, _s) = init_client(db.clone()).await;

        // Step 1: Create the test script with a CI annotation targeting deploy_target.
        // This inserts a ci_test_reference row.
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "# test: script/u/test-user/deploy_target\ndef main():\n    return True",
                    "u/test-user/ci_test_for_deploy",
                    None,
                ),
            )
            .await
            .unwrap();

        // Process the test script's dependency (lock generation) job
        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        // Step 2: Create the target script. Its dependency job, once processed,
        // will call trigger_ci_tests_for_item which finds our test script.
        client
            .create_script(
                "test-workspace",
                &quick_ns(
                    "def main():\n    return 42",
                    "u/test-user/deploy_target",
                    None,
                ),
            )
            .await
            .unwrap();

        // Process the target script's dependency job → CI trigger fires in tokio::spawn
        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        // Give the spawned trigger task time to push the CI test job
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Verify a CI test job was queued
        let ci_jobs = sqlx::query!(
            "SELECT j.id, j.runnable_path, j.trigger_kind::text as trigger_kind \
             FROM v2_job j \
             WHERE j.workspace_id = 'test-workspace' \
               AND j.trigger_kind = 'ci_test'"
        )
        .fetch_all(&db)
        .await?;

        assert_eq!(ci_jobs.len(), 1, "expected exactly 1 CI test job");
        assert_eq!(
            ci_jobs[0].runnable_path.as_deref(),
            Some("u/test-user/ci_test_for_deploy"),
            "CI test job should run the test script"
        );
        assert_eq!(
            ci_jobs[0].trigger_kind.as_deref(),
            Some("ci_test"),
            "trigger_kind should be ci_test"
        );

        Ok(())
    }

    /// A deploy made with `skip_ci_tests` queues no test, even from its dependency job, and
    /// the batch trigger holds the tests back until that lock has landed.
    #[sqlx::test(fixtures("base"))]
    async fn test_ci_tests_deferred_to_batch(db: Pool<Postgres>) -> anyhow::Result<()> {
        let (client, port, _s) = init_client(db.clone()).await;
        let bun_ns = |content: &str, path: &str| NewScript {
            language: ScriptLang::Bun,
            ..quick_ns(content, path, None)
        };
        client
            .create_script(
                "test-workspace",
                &bun_ns(
                    "// test: script/u/test-user/batch_target\nexport function main() { return true }",
                    "u/test-user/ci_test_for_batch",
                ),
            )
            .await
            .unwrap();
        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;

        let http = reqwest::Client::new();
        let api = format!("http://localhost:{port}/api/w/test-workspace/scripts");
        http.post(format!("{api}/create?skip_ci_tests=true"))
            .bearer_auth("SECRET_TOKEN")
            .json(&bun_ns(
                "export function main() { return 42 }",
                "u/test-user/batch_target",
            ))
            .send()
            .await?
            .error_for_status()?;

        let trigger_batch = || async {
            http.post(format!("{api}/trigger_ci_tests_batch"))
                .bearer_auth("SECRET_TOKEN")
                .json(&serde_json::json!({
                    "items": [{ "kind": "script", "path": "u/test-user/batch_target" }]
                }))
                .send()
                .await?
                .error_for_status()?
                .json::<serde_json::Value>()
                .await
        };
        let ci_jobs = || {
            sqlx::query_scalar!(
                "SELECT count(*) FROM v2_job WHERE workspace_id = 'test-workspace' \
                 AND trigger_kind = 'ci_test'"
            )
            .fetch_one(&db)
        };

        let pending = trigger_batch().await?;
        assert_eq!(
            pending["pending"],
            serde_json::json!(["u/test-user/batch_target"])
        );
        assert_eq!(pending["job_ids"], serde_json::json!([]));

        let mut completed = listen_for_completed_jobs(&db).await;
        in_test_worker(&db, completed.next(), port).await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert_eq!(
            ci_jobs().await?,
            Some(0),
            "the lock generation queued a CI test"
        );

        let triggered = trigger_batch().await?;
        assert_eq!(triggered["pending"], serde_json::json!([]));
        assert_eq!(triggered["job_ids"].as_array().map(|a| a.len()), Some(1));
        assert_eq!(ci_jobs().await?, Some(1));

        Ok(())
    }
}
