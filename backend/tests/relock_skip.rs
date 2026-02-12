use sqlx::{Pool, Postgres};
use tokio_stream::StreamExt;
use windmill_api_client::types::NewScript;
use windmill_test_utils::*;

mod relock_skip {
    use super::*;

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

    async fn init(db: Pool<Postgres>) -> (windmill_api_client::Client, u16, ApiServer) {
        init_client(db).await
    }

    /// Counts occurrences of a pattern in job logs for all jobs created after a given time
    async fn count_pattern_in_job_logs(
        db: &Pool<Postgres>,
        pattern: &str,
        after: chrono::DateTime<chrono::Utc>,
    ) -> i64 {
        let logs = sqlx::query_scalar!(
            "SELECT logs FROM job_logs WHERE created_at > $1",
            after
        )
        .fetch_all(db)
        .await
        .unwrap();

        logs.iter()
            .filter_map(|l| l.as_ref())
            .map(|l| l.matches(pattern).count() as i64)
            .sum()
    }

    /// Waits for exactly N jobs to complete. Returns the timestamp before waiting.
    async fn wait_for_jobs(
        completed: &mut (impl futures::Stream<Item = uuid::Uuid> + Unpin),
        count: usize,
    ) -> chrono::DateTime<chrono::Utc> {
        let before = chrono::Utc::now();
        for _ in 0..count {
            completed.next().await;
        }
        before
    }

    /// Waits for at least N jobs to complete, then drains any additional jobs
    /// that complete within a short timeout. Returns the timestamp before waiting.
    async fn wait_for_jobs_ge(
        completed: &mut (impl futures::Stream<Item = uuid::Uuid> + Unpin),
        min_count: usize,
    ) -> chrono::DateTime<chrono::Utc> {
        let before = chrono::Utc::now();
        for _ in 0..min_count {
            completed.next().await;
        }
        // Drain any additional jobs that complete within 5 seconds
        loop {
            match tokio::time::timeout(std::time::Duration::from_secs(1), completed.next()).await {
                Ok(Some(_)) => continue,
                _ => break,
            }
        }
        before
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relock_skip_on_script_redeployment(db: Pool<Postgres>) -> anyhow::Result<()> {
        std::env::set_var("DEPENDENCY_JOB_DEBOUNCE_DELAY", "0");
        let (client, port, _s) = init(db.clone()).await;
        let mut completed = listen_for_completed_jobs(&db).await;

        in_test_worker(&db, async {
            // Step 1: Redeploy leaf_1 - first time, no hashes exist, all should relock
            let before = chrono::Utc::now();
            client
                .create_script(
                    "test-workspace",
                    &quick_ns(
                        "
def main():
    return 'leaf1'
                        ",
                        windmill_api_client::types::ScriptLang::Python3,
                        "f/rel/leaf_1",
                        None,
                        Some("0000000000051658".into()),
                    ),
                )
                .await
                .unwrap();

            // leaf_1(1) + branch(1) + root_script(2: leaf_1+branch) + root_flow(2) + root_app(2) = 8 jobs
            wait_for_jobs(&mut completed, 8).await;

            let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
            let relocking_count = count_pattern_in_job_logs(&db, "Relocking", before).await;
            assert_eq!(skipping_count, 0, "First deployment should not skip");
            assert!(relocking_count > 0, "First deployment should have relocking jobs");

            // Step 2: Redeploy leaf_2 - first time for leaf_2, should relock
            let before = chrono::Utc::now();
            client
                .create_script(
                    "test-workspace",
                    &quick_ns(
                        "
def main():
    return 'leaf2'
                        ",
                        windmill_api_client::types::ScriptLang::Python3,
                        "f/rel/leaf_2",
                        None,
                        Some("0000000000051659".into()),
                    ),
                )
                .await
                .unwrap();

            // leaf_2(1) + root_script(1) + root_flow(1) + root_app(1) = 4 jobs (no cascade, branch doesn't depend on leaf_2)
            wait_for_jobs(&mut completed, 4).await;

            let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
            let relocking_count = count_pattern_in_job_logs(&db, "Relocking", before).await;
            assert_eq!(skipping_count, 0, "leaf_2 first deployment should not skip");
            assert!(relocking_count > 0, "leaf_2 first deployment should have relocking jobs");

            // Step 3: Redeploy leaf_2 with trivial change (comment) - lock stays same, should SKIP
            // Get current parent hash for leaf_2
            let leaf2_hash = sqlx::query_scalar!(
                "SELECT hash FROM script WHERE path = 'f/rel/leaf_2' AND workspace_id = 'test-workspace' AND archived = false ORDER BY created_at DESC LIMIT 1"
            )
            .fetch_one(&db)
            .await
            .unwrap();

            let before = chrono::Utc::now();
            client
                .create_script(
                    "test-workspace",
                    &quick_ns(
                        "
# comment to change hash but not lock
def main():
    return 'leaf2'
                        ",
                        windmill_api_client::types::ScriptLang::Python3,
                        "f/rel/leaf_2",
                        None,
                        Some(format!("{:016X}", leaf2_hash)),
                    ),
                )
                .await
                .unwrap();

            // Same as leaf_2 first deployment: 4 jobs
            wait_for_jobs(&mut completed, 4).await;

            let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
            assert!(skipping_count > 0, "Trivial change (comment only) should skip - lock unchanged");

            // Step 4: Redeploy leaf_2 with actual dependency change (add tiny via comment) - should NOT skip
            let leaf2_hash = sqlx::query_scalar!(
                "SELECT hash FROM script WHERE path = 'f/rel/leaf_2' AND workspace_id = 'test-workspace' AND archived = false ORDER BY created_at DESC LIMIT 1"
            )
            .fetch_one(&db)
            .await
            .unwrap();

            let before = chrono::Utc::now();
            client
                .create_script(
                    "test-workspace",
                    &quick_ns(
                        "
# requirements:
# tiny

def main():
    return 'leaf2 with tiny'
                        ",
                        windmill_api_client::types::ScriptLang::Python3,
                        "f/rel/leaf_2",
                        None,
                        Some(format!("{:016X}", leaf2_hash)),
                    ),
                )
                .await
                .unwrap();

            // Same as leaf_2 first deployment: 4 jobs
            wait_for_jobs(&mut completed, 4).await;

            let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
            assert_eq!(skipping_count, 0, "Changed dependencies should not skip");
        }, port).await;

        Ok(())
    }

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "dependency_map"))]
    async fn relock_skip_on_workspace_deps_redeployment(db: Pool<Postgres>) -> anyhow::Result<()> {
        use windmill_common::scripts::ScriptLang;
        use windmill_dep_map::workspace_dependencies::NewWorkspaceDependencies;

        std::env::set_var("DEPENDENCY_JOB_DEBOUNCE_DELAY", "0");
        std::env::set_var("EXISTS_CACHE_TIMEOUT_MS", "0");

        let (_client, port, _s) = init(db.clone()).await;
        let mut completed = listen_for_completed_jobs(&db).await;

        // Step 1: Redeploy default (unnamed) workspace deps - first time, should relock
        let before = chrono::Utc::now();
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: "".into(),
            name: None, // Default/unnamed
            description: None,
        }
        .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
        .await
        .unwrap();

        in_test_worker(&db, wait_for_jobs_ge(&mut completed, 10), port).await;

        // Note: within a cascade, the same script may be triggered multiple times.
        // After the first trigger relocks and stores the hash, subsequent triggers skip.
        // We allow up to 3 skips from cascade re-triggers.
        let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
        let relocking_count = count_pattern_in_job_logs(&db, "Relocking", before).await;
        assert!(skipping_count <= 3, "First deployment should have at most 3 skips from cascade");
        assert!(relocking_count >= 3, "First deployment should have at least 3 relocking jobs");

        // Step 2: Redeploy default workspace deps again - should SKIP
        let before = chrono::Utc::now();
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: "".into(),
            name: None,
            description: None,
        }
        .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
        .await
        .unwrap();

        in_test_worker(&db, wait_for_jobs_ge(&mut completed, 10), port).await;

        let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
        assert!(skipping_count >= 3, "Second deployment of same content should skip at least 3 times");

        // Step 3: Redeploy default workspace deps with different content - should NOT skip
        let before = chrono::Utc::now();
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: "tiny".into(),
            name: None,
            description: None,
        }
        .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
        .await
        .unwrap();

        in_test_worker(&db, wait_for_jobs_ge(&mut completed, 10), port).await;

        let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
        let relocking_count = count_pattern_in_job_logs(&db, "Relocking", before).await;
        assert!(skipping_count <= 4, "Changed content should have at most 3 skips from cascade");
        assert!(relocking_count >= 3, "Changed content should trigger at least 3 relocking jobs");

        // Step 4: Deploy named workspace deps first time - should relock (no hash exists yet)
        // Named deps trigger exactly 3 independent objects with no cascade
        let before = chrono::Utc::now();
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: "".into(),
            name: Some("test".to_owned()),
            description: None,
        }
        .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
        .await
        .unwrap();

        in_test_worker(&db, wait_for_jobs(&mut completed, 3), port).await;

        let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
        let relocking_count = count_pattern_in_job_logs(&db, "Relocking", before).await;
        assert_eq!(skipping_count, 0, "Named workspace deps first deployment should not skip");
        assert_eq!(relocking_count, 1, "Named workspace deps first deployment should relock exactly 3 times");

        // Step 5: Deploy named workspace deps again with no change - should SKIP
        let before = chrono::Utc::now();
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: "".into(),
            name: Some("test".to_owned()),
            description: None,
        }
        .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
        .await
        .unwrap();

        in_test_worker(&db, wait_for_jobs(&mut completed, 3), port).await;

        let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
        let relocking_count = count_pattern_in_job_logs(&db, "Relocking", before).await;

        assert_eq!(skipping_count, 1, "Named workspace deps second deployment should skip exactly 3 times");
        assert_eq!(relocking_count, 0);

        // Step 6: Deploy named workspace deps with small change - should NOT skip
        let before = chrono::Utc::now();
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: "tiny".into(),
            name: Some("test".to_owned()),
            description: None,
        }
        .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
        .await
        .unwrap();

        in_test_worker(&db, wait_for_jobs(&mut completed, 3), port).await;

        let skipping_count = count_pattern_in_job_logs(&db, "Skipping relock", before).await;
        let relocking_count = count_pattern_in_job_logs(&db, "Relocking", before).await;
        assert_eq!(skipping_count, 0, "Named workspace deps with change should not skip");
        assert_eq!(relocking_count, 1, "Named workspace deps with change should relock exactly 3 times");

        Ok(())
    }
}
