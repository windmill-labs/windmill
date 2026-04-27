mod workspace_dependencies {

    use sqlx::{Pool, Postgres};
    use tokio_stream::StreamExt;
    use windmill_common::scripts::ScriptLang;
    use windmill_common::workspace_dependencies::WorkspaceDependencies;
    use windmill_dep_map::workspace_dependencies::NewWorkspaceDependencies;
    use windmill_test_utils::in_test_worker;
    use windmill_test_utils::init_client;
    use windmill_test_utils::listen_for_completed_jobs;

    mod deps {
        pub const REQUIREMENTS_IN: &'static str = "tiny==0.1.3";
        pub const REQUIREMENTS_IN_V2: &'static str = "tiny==0.2.0";

        pub const PACKAGE_JSON: &'static str = r##"
    {
      "name": "example-project",
      "version": "1.0.0",
      "dependencies": {
        "express": "^4.17.1"
      }
    }
    "##;

        #[allow(dead_code)]
        pub const PACKAGE_JSON_V2: &'static str = r##"
    {
      "name": "example-project",
      "version": "2.0.0",
      "dependencies": {
        "express": "^4.18.0",
        "axios": "^1.0.0"
      }
    }
    "##;

        pub const COMPOSER_JSON: &'static str = r##"
    {
      "name": "example/project",
      "require": {
        "monolog/monolog": "^2.3"
      }
    }
    "##;
    }

    // =========================================================================
    // CRUD Tests
    // =========================================================================

    /// Test: Create workspace dependencies and verify they are stored correctly.
    #[sqlx::test(fixtures("base"))]
    async fn test_create_workspace_dependencies(db: Pool<Postgres>) -> anyhow::Result<()> {
        let id = NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN.into(),
            name: Some("test-deps".to_owned()),
            description: Some("Test dependencies".to_owned()),
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        assert!(id > 0, "Should return a valid ID");

        // Verify it was stored correctly
        let stored = WorkspaceDependencies::get(id, "test-workspace".to_owned(), &db).await?;
        assert_eq!(stored.name, Some("test-deps".to_owned()));
        assert_eq!(stored.content, deps::REQUIREMENTS_IN);
        assert_eq!(stored.language, ScriptLang::Python3);

        Ok(())
    }

    /// Test: Create unnamed (default) workspace dependencies.
    #[sqlx::test(fixtures("base"))]
    async fn test_create_unnamed_workspace_dependencies(db: Pool<Postgres>) -> anyhow::Result<()> {
        let id = NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Bun,
            content: deps::PACKAGE_JSON.into(),
            name: None, // Unnamed = default
            description: None,
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        assert!(id > 0, "Should return a valid ID");

        // Verify it was stored correctly
        let stored = WorkspaceDependencies::get(id, "test-workspace".to_owned(), &db).await?;
        assert_eq!(stored.name, None);
        assert_eq!(stored.language, ScriptLang::Bun);

        Ok(())
    }

    /// Test: List workspace dependencies returns all active entries.
    #[sqlx::test(fixtures("base"))]
    async fn test_list_workspace_dependencies(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Create multiple workspace dependencies
        for (lang, content, name) in [
            (ScriptLang::Python3, deps::REQUIREMENTS_IN, Some("python-deps")),
            (ScriptLang::Bun, deps::PACKAGE_JSON, Some("bun-deps")),
            (ScriptLang::Bun, deps::PACKAGE_JSON, None), // Default bun deps
        ] {
            NewWorkspaceDependencies {
                workspace_id: "test-workspace".into(),
                language: lang,
                content: content.into(),
                name: name.map(|s| s.to_owned()),
                description: None,
            }
            .create(
                (
                    "test@test.com".to_owned(),
                    "u/test".to_owned(),
                    "test".to_owned(),
                ),
                db.clone(),
            )
            .await?;
        }

        let list = WorkspaceDependencies::list("test-workspace", &db).await?;
        assert_eq!(list.len(), 3, "Should have 3 workspace dependencies");

        Ok(())
    }

    /// Test: Archive workspace dependencies marks them as archived.
    #[sqlx::test(fixtures("base"))]
    async fn test_archive_workspace_dependencies(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Create workspace dependencies
        let _id = NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN.into(),
            name: Some("to-archive".to_owned()),
            description: None,
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        // Verify it exists
        let list_before = WorkspaceDependencies::list("test-workspace", &db).await?;
        assert_eq!(list_before.len(), 1);

        // Archive it
        WorkspaceDependencies::archive(
            Some("to-archive".to_owned()),
            ScriptLang::Python3,
            "test-workspace",
            &db,
        )
        .await?;

        // Verify it's no longer in the active list
        let list_after = WorkspaceDependencies::list("test-workspace", &db).await?;
        assert_eq!(list_after.len(), 0, "Archived deps should not appear in list");

        Ok(())
    }

    /// Test: Delete workspace dependencies permanently removes them.
    #[sqlx::test(fixtures("base"))]
    async fn test_delete_workspace_dependencies(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Create workspace dependencies
        let id = NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN.into(),
            name: Some("to-delete".to_owned()),
            description: None,
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        // Verify it exists
        assert!(
            WorkspaceDependencies::get(id, "test-workspace".to_owned(), &db)
                .await
                .is_ok()
        );

        // Delete it
        WorkspaceDependencies::delete(
            Some("to-delete".to_owned()),
            ScriptLang::Python3,
            "test-workspace",
            &db,
        )
        .await?;

        // Verify it's gone (should error)
        let result = WorkspaceDependencies::get(id, "test-workspace".to_owned(), &db).await;
        assert!(result.is_err(), "Deleted deps should not be retrievable");

        Ok(())
    }

    // =========================================================================
    // Version History Tests
    // =========================================================================

    /// Test: Creating new version archives the old one.
    #[sqlx::test(fixtures("base"))]
    async fn test_versioning_archives_previous(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Create first version
        let id1 = NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN.into(),
            name: Some("versioned".to_owned()),
            description: Some("Version 1".to_owned()),
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        // Create second version with same name
        let id2 = NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN_V2.into(),
            name: Some("versioned".to_owned()),
            description: Some("Version 2".to_owned()),
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        assert_ne!(id1, id2, "Should create a new entry");

        // List should only show the active (latest) version
        let list = WorkspaceDependencies::list("test-workspace", &db).await?;
        assert_eq!(list.len(), 1, "Should only have 1 active entry");
        assert_eq!(list[0].content, deps::REQUIREMENTS_IN_V2);

        // History should show both versions
        let history = WorkspaceDependencies::get_history(
            Some("versioned".to_owned()),
            ScriptLang::Python3,
            "test-workspace",
            &db,
        )
        .await?;
        assert_eq!(history.len(), 2, "Should have 2 versions in history");

        Ok(())
    }

    /// Test: Description is inherited from previous version if not provided.
    #[sqlx::test(fixtures("base"))]
    async fn test_description_inheritance(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Create first version with description
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN.into(),
            name: Some("inherit-desc".to_owned()),
            description: Some("Original description".to_owned()),
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        // Create second version without description
        let id2 = NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN_V2.into(),
            name: Some("inherit-desc".to_owned()),
            description: None, // Should inherit
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        let stored = WorkspaceDependencies::get(id2, "test-workspace".to_owned(), &db).await?;
        assert_eq!(
            stored.description,
            Some("Original description".to_owned()),
            "Description should be inherited from previous version"
        );

        Ok(())
    }

    // =========================================================================
    // Workspace Isolation Tests
    // =========================================================================

    /// Test: Workspace dependencies are isolated between workspaces.
    #[sqlx::test(fixtures("base"))]
    async fn test_workspace_isolation(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Create another workspace
        sqlx::query!(
            "INSERT INTO workspace (id, name, owner) VALUES ('other-workspace', 'other', 'test-user')"
        )
        .execute(&db)
        .await?;
        sqlx::query!("INSERT INTO workspace_settings (workspace_id) VALUES ('other-workspace')")
            .execute(&db)
            .await?;

        // Create deps in test-workspace
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN.into(),
            name: Some("shared-name".to_owned()),
            description: None,
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        // Create deps in other-workspace with same name
        NewWorkspaceDependencies {
            workspace_id: "other-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN_V2.into(),
            name: Some("shared-name".to_owned()),
            description: None,
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        // Each workspace should have exactly 1 entry
        let list1 = WorkspaceDependencies::list("test-workspace", &db).await?;
        let list2 = WorkspaceDependencies::list("other-workspace", &db).await?;

        assert_eq!(list1.len(), 1);
        assert_eq!(list2.len(), 1);

        // Content should be different
        assert_eq!(list1[0].content, deps::REQUIREMENTS_IN);
        assert_eq!(list2[0].content, deps::REQUIREMENTS_IN_V2);

        Ok(())
    }

    // =========================================================================
    // Language-specific Tests
    // =========================================================================

    /// Test: Different languages can have same-named workspace dependencies.
    #[sqlx::test(fixtures("base"))]
    async fn test_same_name_different_languages(db: Pool<Postgres>) -> anyhow::Result<()> {
        // Create Python deps
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Python3,
            content: deps::REQUIREMENTS_IN.into(),
            name: Some("common".to_owned()),
            description: None,
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        // Create Bun deps with same name
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Bun,
            content: deps::PACKAGE_JSON.into(),
            name: Some("common".to_owned()),
            description: None,
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        let list = WorkspaceDependencies::list("test-workspace", &db).await?;
        assert_eq!(list.len(), 2, "Should have 2 entries (different languages)");

        let python_deps: Vec<_> = list
            .iter()
            .filter(|d| d.language == ScriptLang::Python3)
            .collect();
        let bun_deps: Vec<_> = list
            .iter()
            .filter(|d| d.language == ScriptLang::Bun)
            .collect();

        assert_eq!(python_deps.len(), 1);
        assert_eq!(bun_deps.len(), 1);

        Ok(())
    }

    /// Test: Nativets and Bunnative use Bun workspace dependencies.
    #[sqlx::test(fixtures("base"))]
    async fn test_nativets_uses_bun_deps(db: Pool<Postgres>) -> anyhow::Result<()> {
        use windmill_common::worker::Connection;

        // Create Bun deps (which Nativets should use)
        NewWorkspaceDependencies {
            workspace_id: "test-workspace".into(),
            language: ScriptLang::Bun,
            content: deps::PACKAGE_JSON.into(),
            name: None,
            description: None,
        }
        .create(
            (
                "test@test.com".to_owned(),
                "u/test".to_owned(),
                "test".to_owned(),
            ),
            db.clone(),
        )
        .await?;

        // Query for Nativets should return Bun deps
        let result = WorkspaceDependencies::get_latest(
            None,
            ScriptLang::Nativets,
            "test-workspace",
            Connection::Sql(db.clone()),
        )
        .await?;

        assert!(result.is_some(), "Nativets should find Bun deps");
        assert_eq!(result.unwrap().language, ScriptLang::Bun);

        Ok(())
    }

    // =========================================================================
    // Path Generation Tests
    // =========================================================================

    /// Test: to_path generates correct paths for named and unnamed deps.
    #[test]
    fn test_to_path_generation() {
        // Unnamed (default) deps
        let path = WorkspaceDependencies::to_path(&None, ScriptLang::Python3).unwrap();
        assert_eq!(path, "dependencies/requirements.in");

        let path = WorkspaceDependencies::to_path(&None, ScriptLang::Bun).unwrap();
        assert_eq!(path, "dependencies/package.json");

        let path = WorkspaceDependencies::to_path(&None, ScriptLang::Php).unwrap();
        assert_eq!(path, "dependencies/composer.json");

        // Named deps
        let path =
            WorkspaceDependencies::to_path(&Some("custom".to_owned()), ScriptLang::Python3).unwrap();
        assert_eq!(path, "dependencies/custom.requirements.in");

        let path =
            WorkspaceDependencies::to_path(&Some("custom".to_owned()), ScriptLang::Bun).unwrap();
        assert_eq!(path, "dependencies/custom.package.json");
    }

    /// Test: to_path returns error for unsupported languages.
    #[test]
    fn test_to_path_unsupported_language() {
        // Deno doesn't support workspace dependencies
        let result = WorkspaceDependencies::to_path(&None, ScriptLang::Deno);
        assert!(result.is_err(), "Deno should not support workspace deps");
    }

    /// Test E2E: Creating named workspace dependencies triggers re-lock jobs for dependent scripts.
    ///
    /// This test:
    /// 1. Uses fixture with Python, Bun, PHP scripts linked to named workspace deps via dependency_map
    /// 2. Creates named workspace dependencies for each language
    /// 3. Verifies dependency jobs are triggered for all linked scripts
    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "workspace_dependencies_leafs"))]
    async fn basic_manual_named(db: Pool<Postgres>) -> anyhow::Result<()> {
        let ((_client, port, _s), db, mut completed) = (
            init_client(db.clone()).await,
            &db,
            listen_for_completed_jobs(&db).await,
        );

        // Create named workspace dependencies for Python, Bun, and PHP
        // These will trigger dependency jobs for scripts linked via dependency_map
        for (lang, content) in [
            (ScriptLang::Python3, deps::REQUIREMENTS_IN),
            (ScriptLang::Bun, deps::PACKAGE_JSON),
            (ScriptLang::Php, deps::COMPOSER_JSON),
        ] {
            NewWorkspaceDependencies {
                workspace_id: "test-workspace".into(),
                language: lang,
                content: content.into(),
                name: Some("test".to_owned()),
                description: None,
            }
            .create(
                (
                    "test@test.com".to_owned(),
                    "u/test-user".to_owned(),
                    "test-user".to_owned(),
                ),
                db.clone(),
            )
            .await?;
        }

        // Wait for 3 dependency jobs (one per script in fixture)
        let mut completed_paths = vec![];
        for _ in 0..3 {
            let job_id = in_test_worker(db, async { completed.next().await }, port)
                .await
                .expect("Expected a dependency job to complete");

            let job_path = sqlx::query_scalar!(
                "SELECT runnable_path FROM v2_job WHERE id = $1",
                job_id
            )
            .fetch_one(db)
            .await?;

            if let Some(path) = job_path {
                completed_paths.push(path);
            }
        }

        // Verify all 3 scripts received dependency jobs
        completed_paths.sort();
        let expected = vec![
            "f/leafs/php".to_string(),
            "f/leafs/python".to_string(),
            "f/leafs/ts".to_string(),
        ];

        assert_eq!(
            completed_paths, expected,
            "All scripts should have received dependency jobs"
        );

        // Verify no extra jobs were created
        let total_jobs = sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
            .fetch_one(db)
            .await?;

        assert_eq!(total_jobs, Some(3), "Should have exactly 3 jobs");

        Ok(())
    }

    #[sqlx::test(fixtures("base", "hub_sync_blacklist"))]
    async fn hub_sync_blacklist_from_workspace_deps(db: Pool<Postgres>) -> anyhow::Result<()> {
        let ((_client, port, _s), db, mut completed) = (
            init_client(db.clone()).await,
            &db,
            listen_for_completed_jobs(&db).await,
        );

        // Verify built-in fixtures exist
        // Check that hub_sync script exists and is a Bun script
        let hub_sync_lang = sqlx::query_scalar!(
            r#"SELECT language AS "language: ScriptLang" FROM script WHERE path = 'u/admin/hub_sync'"#
        )
        .fetch_one(db)
        .await
        .unwrap();
        assert_eq!(
            hub_sync_lang,
            ScriptLang::Bun,
            "Expected hub_sync to be a Bun script"
        );

        // Create unnamed (default) workspace dependencies for Bun
        let _id = NewWorkspaceDependencies {
            workspace_id: "admins".into(),
            language: ScriptLang::Bun,
            content: deps::PACKAGE_JSON.into(),
            name: None, // No name = default workspace dependencies
            description: None,
        }
        .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
        .await
        .unwrap();

        // Wait for exactly 1 job (only u/admin/simple_bun, not hub_sync)
        let job_id = in_test_worker(db, async { completed.next().await }, port)
            .await
            .expect("Expected one job to complete");

        // Query the job's runnable_path
        let runnable_path =
            sqlx::query_scalar!("SELECT runnable_path FROM v2_job WHERE id = $1", job_id)
                .fetch_one(db)
                .await
                .unwrap();

        assert_eq!(
            runnable_path,
            Some("u/admin/simple_bun".to_string()),
            "Expected job runnable_path to be 'u/admin/simple_bun' (hub_sync should be blacklisted)"
        );

        // Assert total job count is 1
        let job_count = sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job")
            .fetch_one(db)
            .await
            .unwrap();

        assert_eq!(job_count, Some(1), "Expected exactly one job total");

        // Assert v2_job_queue is empty
        let queue_count = sqlx::query_scalar!("SELECT COUNT(*) FROM v2_job_queue")
            .fetch_one(db)
            .await
            .unwrap();

        assert_eq!(queue_count, Some(0), "Expected job queue to be empty");

        Ok(())
    }
}
