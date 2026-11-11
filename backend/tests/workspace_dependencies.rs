mod workspace_dependencies {

    use sqlx::{Pool, Postgres};
    use windmill_common::scripts::ScriptLang;
    use windmill_dep_map::workspace_dependencies::NewWorkspaceDependencies;

    const PACKAGE_JSON: &'static str = r##"
    {
      "name": "example-project",
      "version": "1.0.0",
      "dependencies": {
        "express": "^4.17.1"
      }
    }
    "##;

    #[sqlx::test(fixtures("base", "hub_sync_blacklist"))]
    async fn hub_sync_blacklist_from_workspace_deps(db: Pool<Postgres>) -> anyhow::Result<()> {
        // The first unnamed workspace dependencies wait out the existence cache before
        // redeploying their dependents. Read once per process, and this binary has no other test.
        std::env::set_var("EXISTS_CACHE_TIMEOUT_MS", "0");
        let db = &db;

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

        // Unnamed (default) workspace dependencies for Bun
        NewWorkspaceDependencies {
            workspace_id: "admins".into(),
            language: ScriptLang::Bun,
            content: PACKAGE_JSON.into(),
            name: None,
            description: None,
        }
        .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
        .await
        .unwrap();

        // The dependency map decides which runnables are redeployed, and is rebuilt before
        // `create` returns.
        let importers: Vec<String> = sqlx::query_scalar(
            "SELECT importer_path FROM dependency_map
             WHERE workspace_id = 'admins' AND imported_path = 'dependencies/package.json'
             ORDER BY importer_path",
        )
        .fetch_all(db)
        .await?;
        assert_eq!(
            importers,
            vec!["u/admin/simple_bun".to_string()],
            "hub_sync must not be recorded as importing the default workspace dependencies"
        );

        // The redeploy is pushed in the background as a debounced dependency job; it never needs
        // to run for what is asserted here.
        let mut jobs: Vec<(Option<String>, String)> = vec![];
        for _ in 0..200 {
            jobs = sqlx::query_as(
                "SELECT runnable_path, kind::text FROM v2_job WHERE workspace_id = 'admins'",
            )
            .fetch_all(db)
            .await?;
            if !jobs.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert_eq!(
            jobs,
            vec![(
                Some("u/admin/simple_bun".to_string()),
                "dependencies".to_string()
            )],
            "Expected exactly one dependency job, for u/admin/simple_bun (hub_sync should be blacklisted)"
        );

        Ok(())
    }
}
