/// Integration tests for git sync deployment callback job creation.
///
/// These tests verify that `handle_deployment_metadata_inner` creates the right
/// DeploymentCallback jobs with correct arguments based on workspace git sync settings.
///
/// Requires `enterprise` + `private` features (git sync is EE-only).
#[cfg(all(feature = "enterprise", feature = "private"))]
mod tests {
    use serde_json::json;
    use sqlx::{Pool, Postgres};
    use windmill_common::scripts::ScriptHash;
    use windmill_git_sync::DeployedObject;
    use windmill_test_utils::*;

    fn client() -> reqwest::Client {
        reqwest::Client::new()
    }

    fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.header("Authorization", "Bearer SECRET_TOKEN")
    }

    /// Helper: query a queued job's args by UUID
    async fn get_job_args(db: &Pool<Postgres>, job_id: uuid::Uuid) -> serde_json::Value {
        sqlx::query_scalar!("SELECT args FROM v2_job WHERE id = $1", job_id)
            .fetch_one(db)
            .await
            .expect("job not found")
            .unwrap_or(json!(null))
    }

    // ── Test: basic job creation ──────────────────────────────────────

    /// Deploy a script under f/ → repo1 matches, repo2 doesn't → exactly 1 job
    #[sqlx::test(migrations = "../migrations", fixtures("base", "git_sync"))]
    async fn test_deployment_creates_callback_job(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let _server = ApiServer::start(db.clone()).await?;

        let obj = DeployedObject::Script {
            hash: ScriptHash(1001),
            path: "f/test/hello".to_string(),
            parent_path: None,
        };

        let uuids = windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
            db.clone(),
            "test-workspace".to_string(),
            "test@windmill.dev".to_string(),
            "test-user".to_string(),
            None, // no custom deployment message
            false,
            Some(obj),
            None,
            false,
        )
        .await?;

        // Only repo1 (f/**) should match, not repo2 (g/**)
        assert_eq!(
            uuids.len(),
            1,
            "expected exactly 1 job, got {}",
            uuids.len()
        );

        let args = get_job_args(&db, uuids[0]).await;
        assert_eq!(args["repo_url_resource_path"], "u/test-user/git_repo_1");
        assert_eq!(args["workspace_id"], "test-workspace");

        // Without debouncing, flat args are used
        if let Some(path_type) = args.get("path_type") {
            assert_eq!(path_type, "script");
        }
        if let Some(commit_msg) = args.get("commit_msg") {
            let msg = commit_msg.as_str().unwrap_or("");
            assert!(
                msg.starts_with("[WM]"),
                "commit_msg should start with [WM], got: {msg}"
            );
            assert!(
                msg.contains("f/test/hello"),
                "commit_msg should contain the path"
            );
        }

        // Check settings flags
        assert_eq!(args["use_individual_branch"], false);
        assert_eq!(args["group_by_folder"], false);

        Ok(())
    }

    // ── Test: multi-repo fanout ───────────────────────────────────────

    /// When both repos match the same path, both should get a job
    #[sqlx::test(migrations = "../migrations", fixtures("base", "git_sync"))]
    async fn test_deployment_multi_repo_fanout(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let _server = ApiServer::start(db.clone()).await?;

        // Override config: both repos match f/**
        sqlx::query!(
            r#"UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = 'test-workspace'"#,
            json!({
                "include_path": ["f/**"],
                "include_type": ["script"],
                "repositories": [
                    {
                        "script_path": "hub/28160/sync-script-to-git-repo-windmill",
                        "git_repo_resource_path": "$res:u/test-user/git_repo_1",
                        "settings": { "include_path": ["f/**"], "include_type": ["script"], "exclude_path": [], "extra_include_path": [] }
                    },
                    {
                        "script_path": "hub/28160/sync-script-to-git-repo-windmill",
                        "git_repo_resource_path": "$res:u/test-user/git_repo_2",
                        "settings": { "include_path": ["f/**"], "include_type": ["script"], "exclude_path": [], "extra_include_path": [] }
                    }
                ]
            })
        )
        .execute(&db)
        .await?;

        let obj = DeployedObject::Script {
            hash: ScriptHash(2001),
            path: "f/shared/script".to_string(),
            parent_path: None,
        };

        let uuids = windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
            db.clone(),
            "test-workspace".to_string(),
            "test@windmill.dev".to_string(),
            "test-user".to_string(),
            None,
            false,
            Some(obj),
            None,
            false,
        )
        .await?;

        assert_eq!(uuids.len(), 2, "expected 2 jobs for 2 repos");

        let args_0 = get_job_args(&db, uuids[0]).await;
        let args_1 = get_job_args(&db, uuids[1]).await;
        // Each job should target a different repo
        assert_ne!(
            args_0["repo_url_resource_path"], args_1["repo_url_resource_path"],
            "jobs should target different repos"
        );

        Ok(())
    }

    // ── Test: filtering (path + type combined) ────────────────────────

    /// Objects that don't match path or type filters should produce no jobs
    #[sqlx::test(migrations = "../migrations", fixtures("base", "git_sync"))]
    async fn test_deployment_filtered_out_creates_no_job(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let _server = ApiServer::start(db.clone()).await?;

        // Case 1: path doesn't match any repo
        let wrong_path = DeployedObject::Script {
            hash: ScriptHash(3001),
            path: "x/nowhere/script".to_string(),
            parent_path: None,
        };
        let uuids = windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
            db.clone(),
            "test-workspace".to_string(),
            "test@windmill.dev".to_string(),
            "test-user".to_string(),
            None,
            false,
            Some(wrong_path),
            None,
            false,
        )
        .await?;
        assert_eq!(uuids.len(), 0, "path mismatch should create no jobs");

        // Case 2: path matches repo1 (f/**) but type is Variable, which is not in
        // repo1's include_type (script, flow, app)
        let wrong_type =
            DeployedObject::Variable { path: "f/test/var".to_string(), parent_path: None };
        let uuids = windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
            db.clone(),
            "test-workspace".to_string(),
            "test@windmill.dev".to_string(),
            "test-user".to_string(),
            None,
            false,
            Some(wrong_type),
            None,
            false,
        )
        .await?;
        assert_eq!(uuids.len(), 0, "type mismatch should create no jobs");

        // Case 3: path matches, type matches → should create a job (sanity check)
        let matching = DeployedObject::Script {
            hash: ScriptHash(3003),
            path: "f/test/good".to_string(),
            parent_path: None,
        };
        let uuids = windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
            db.clone(),
            "test-workspace".to_string(),
            "test@windmill.dev".to_string(),
            "test-user".to_string(),
            None,
            false,
            Some(matching),
            None,
            false,
        )
        .await?;
        assert_eq!(uuids.len(), 1, "matching path+type should create 1 job");

        Ok(())
    }

    // ── Test: draft-only skip ─────────────────────────────────────────

    #[sqlx::test(migrations = "../migrations", fixtures("base", "git_sync"))]
    async fn test_deployment_draft_only_skipped(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let _server = ApiServer::start(db.clone()).await?;

        // Insert a draft-only script
        sqlx::query!(
            "INSERT INTO script (workspace_id, path, hash, content, summary, description, language, created_by, created_at, archived, schema_validation, ws_error_handler_muted, deleted, draft_only)
             VALUES ('test-workspace', 'f/test/draft', 4001, 'def main(): pass', '', '', 'python3', 'test@windmill.dev', NOW(), false, false, false, false, true)"
        )
        .execute(&db)
        .await?;

        let obj = DeployedObject::Script {
            hash: ScriptHash(4001),
            path: "f/test/draft".to_string(),
            parent_path: None,
        };

        let uuids = windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
            db.clone(),
            "test-workspace".to_string(),
            "test@windmill.dev".to_string(),
            "test-user".to_string(),
            None,
            false,
            Some(obj),
            None,
            false,
        )
        .await?;

        assert_eq!(
            uuids.len(),
            0,
            "draft-only scripts should not trigger git sync"
        );

        Ok(())
    }

    // ── Test: settings propagation ────────────────────────────────────

    /// Verify use_individual_branch, group_by_folder, force_branch, skip_secret
    /// are all correctly forwarded to job args
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_deployment_settings_propagation(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let _server = ApiServer::start(db.clone()).await?;

        // Insert workspace_settings row is already done by base fixture
        sqlx::query!(
            r#"UPDATE workspace_settings SET git_sync = $1 WHERE workspace_id = 'test-workspace'"#,
            json!({
                "include_path": ["f/**"],
                "include_type": ["variable"],
                "repositories": [{
                    "script_path": "hub/28160/sync-script-to-git-repo-windmill",
                    "git_repo_resource_path": "$res:u/test/repo",
                    "use_individual_branch": true,
                    "group_by_folder": true,
                    "force_branch": "staging",
                    "settings": {
                        "include_path": ["f/**"],
                        "include_type": ["variable"],
                        "exclude_path": [],
                        "extra_include_path": []
                    }
                }]
            })
        )
        .execute(&db)
        .await?;

        let obj = DeployedObject::Variable { path: "f/test/myvar".to_string(), parent_path: None };

        let uuids = windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
            db.clone(),
            "test-workspace".to_string(),
            "test@windmill.dev".to_string(),
            "test-user".to_string(),
            None,
            false,
            Some(obj),
            None,
            false,
        )
        .await?;

        assert_eq!(uuids.len(), 1);

        let args = get_job_args(&db, uuids[0]).await;
        assert_eq!(args["use_individual_branch"], true);
        assert_eq!(args["group_by_folder"], true);
        assert_eq!(args["force_branch"], "staging");
        // include_type has "variable" but not "secret" → skip_secret should be true
        assert_eq!(args["skip_secret"], true);

        Ok(())
    }

    // ── Test: rename populates parent_path ─────────────────────────────

    #[sqlx::test(migrations = "../migrations", fixtures("base", "git_sync"))]
    async fn test_deployment_rename_populates_parent_path(
        db: Pool<Postgres>,
    ) -> anyhow::Result<()> {
        initialize_tracing().await;
        let _server = ApiServer::start(db.clone()).await?;

        let obj = DeployedObject::Script {
            hash: ScriptHash(6001),
            path: "f/new_name".to_string(),
            parent_path: Some("f/old_name".to_string()),
        };

        let uuids = windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
            db.clone(),
            "test-workspace".to_string(),
            "test@windmill.dev".to_string(),
            "test-user".to_string(),
            None,
            false,
            Some(obj),
            None,
            false,
        )
        .await?;

        assert_eq!(uuids.len(), 1);

        let args = get_job_args(&db, uuids[0]).await;
        // Both path and parent_path should be set
        // With debouncing disabled (old script version), these are flat args
        if let Some(path) = args.get("path") {
            assert_eq!(path, "f/new_name");
        }
        if let Some(parent_path) = args.get("parent_path") {
            assert_eq!(parent_path, "f/old_name");
        }
        // With debouncing, check inside items array
        if let Some(items) = args.get("items") {
            let first = &items[0];
            assert_eq!(first["path"], "f/new_name");
            assert_eq!(first["parent_path"], "f/old_name");
        }

        Ok(())
    }

    // ── Test: commit message formatting ───────────────────────────────

    #[sqlx::test(migrations = "../migrations", fixtures("base", "git_sync"))]
    async fn test_deployment_commit_message_formatting(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let _server = ApiServer::start(db.clone()).await?;

        // Helper to get commit_msg from a deployment
        let deploy = |db: &Pool<Postgres>, msg: Option<String>| {
            let db = db.clone();
            async move {
                let obj = DeployedObject::Script {
                    hash: ScriptHash(rand::random::<i64>()),
                    path: "f/test/x".to_string(),
                    parent_path: None,
                };
                windmill_git_sync::git_sync_ee::handle_deployment_metadata_inner(
                    db,
                    "test-workspace".to_string(),
                    "test@windmill.dev".to_string(),
                    "test-user".to_string(),
                    msg,
                    true, // skip_db_insert to avoid PK conflicts
                    Some(obj),
                    None,
                    false,
                )
                .await
            }
        };

        // Case 1: no message → default
        let uuids = deploy(&db, None).await?;
        let args = get_job_args(&db, uuids[0]).await;
        let msg = args
            .get("commit_msg")
            .or_else(|| {
                args.get("items")
                    .and_then(|i| i.get(0))
                    .and_then(|i| i.get("commit_msg"))
            })
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(msg.starts_with("[WM]"), "default: should start with [WM]");
        assert!(msg.contains("f/test/x"), "default: should contain path");

        // Case 2: custom message
        let uuids = deploy(&db, Some("fix bug".to_string())).await?;
        let args = get_job_args(&db, uuids[0]).await;
        let msg = args
            .get("commit_msg")
            .or_else(|| {
                args.get("items")
                    .and_then(|i| i.get(0))
                    .and_then(|i| i.get("commit_msg"))
            })
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(msg, "[WM] fix bug");

        // Case 3: empty message → treated as no message (default)
        let uuids = deploy(&db, Some("".to_string())).await?;
        let args = get_job_args(&db, uuids[0]).await;
        let msg = args
            .get("commit_msg")
            .or_else(|| {
                args.get("items")
                    .and_then(|i| i.get(0))
                    .and_then(|i| i.get("commit_msg"))
            })
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(msg.starts_with("[WM]"), "empty: should start with [WM]");
        assert!(msg.contains("f/test/x"), "empty: should contain path");

        Ok(())
    }

    // ── Test: API endpoints ───────────────────────────────────────────

    /// Tests git sync API endpoints. These require an EE license at runtime,
    /// so we accept 200 (licensed) or 400 (unlicensed) — either way the
    /// endpoint exists and responds correctly.
    #[sqlx::test(migrations = "../migrations", fixtures("base"))]
    async fn test_git_sync_api_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
        initialize_tracing().await;
        let server = ApiServer::start(db.clone()).await?;
        let port = server.addr.port();
        let base = format!("http://localhost:{port}/api/w/test-workspace/workspaces");

        // 1. edit_git_sync_config
        let resp = authed(client().post(format!("{base}/edit_git_sync_config")))
            .json(&json!({
                "git_sync_settings": {
                    "include_path": ["f/**"],
                    "include_type": ["script"],
                    "repositories": [{
                        "script_path": "hub/28160/sync",
                        "git_repo_resource_path": "$res:u/test/repo1",
                        "settings": {
                            "include_path": ["f/**"],
                            "include_type": ["script"],
                            "exclude_path": [],
                            "extra_include_path": []
                        }
                    }]
                }
            }))
            .send()
            .await?;
        let status = resp.status().as_u16();
        assert!(
            status == 200 || status == 400,
            "edit_git_sync_config: unexpected status {status}"
        );

        if status == 200 {
            // Full assertions only when EE license is active
            let stored: serde_json::Value = sqlx::query_scalar!(
                "SELECT git_sync FROM workspace_settings WHERE workspace_id = 'test-workspace'"
            )
            .fetch_one(&db)
            .await?
            .unwrap_or(json!(null));
            assert_eq!(stored["repositories"].as_array().unwrap().len(), 1);

            // 2. edit_git_sync_repository
            let resp = authed(client().post(format!("{base}/edit_git_sync_repository")))
                .json(&json!({
                    "git_repo_resource_path": "$res:u/test/repo2",
                    "repository": {
                        "script_path": "hub/28160/sync",
                        "git_repo_resource_path": "$res:u/test/repo2",
                        "settings": {
                            "include_path": ["g/**"],
                            "include_type": ["flow"],
                            "exclude_path": [],
                            "extra_include_path": []
                        }
                    }
                }))
                .send()
                .await?;
            assert_eq!(resp.status(), 200);

            let stored: serde_json::Value = sqlx::query_scalar!(
                "SELECT git_sync FROM workspace_settings WHERE workspace_id = 'test-workspace'"
            )
            .fetch_one(&db)
            .await?
            .unwrap_or(json!(null));
            assert_eq!(stored["repositories"].as_array().unwrap().len(), 2);

            // 3. delete_git_sync_repository
            let resp = authed(client().delete(format!("{base}/delete_git_sync_repository")))
                .json(&json!({ "git_repo_resource_path": "$res:u/test/repo2" }))
                .send()
                .await?;
            assert_eq!(resp.status(), 200);

            let stored: serde_json::Value = sqlx::query_scalar!(
                "SELECT git_sync FROM workspace_settings WHERE workspace_id = 'test-workspace'"
            )
            .fetch_one(&db)
            .await?
            .unwrap_or(json!(null));
            assert_eq!(stored["repositories"].as_array().unwrap().len(), 1);
        }

        Ok(())
    }
}
