mod workspace_dependencies {

    use windmill_test_utils::in_test_worker;
    use windmill_test_utils::init_client;
    use windmill_test_utils::listen_for_completed_jobs;
    use sqlx::{Pool, Postgres};
    use tokio_stream::StreamExt;
    use windmill_common::scripts::ScriptLang;
    use windmill_dep_map::workspace_dependencies::NewWorkspaceDependencies;
    mod deps {
        pub const REQUIREMENTS_IN: &'static str = "tiny==0.1.3";
        //     pub const GO_MOD: &'static str = r##"
        // module example.com/project

        // go 1.20

        // require github.com/gin-gonic/gin v1.8.1
        // "##;

        pub const PACKAGE_JSON: &'static str = r##"
    {
      "name": "example-project",
      "version": "1.0.0",
      "dependencies": {
        "express": "^4.17.1"
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

    #[cfg(feature = "python")]
    #[sqlx::test(fixtures("base", "workspace_dependencies_leafs"))]
    #[ignore]
    async fn basic_manual_named(db: Pool<Postgres>) -> anyhow::Result<()> {
        let ((_client, port, _s), db, mut completed) = (
            init_client(db.clone()).await,
            &db,
            listen_for_completed_jobs(&db).await,
        );

        for (idx, (l, c)) in [
            (ScriptLang::Python3, deps::REQUIREMENTS_IN),
            (ScriptLang::Bun, deps::PACKAGE_JSON),
            (ScriptLang::Php, deps::COMPOSER_JSON),
            // (ScriptLang::Go, deps::GO_MOD),
        ]
        .iter()
        .enumerate()
        {
            let id = NewWorkspaceDependencies {
                workspace_id: "test-workspace".into(),
                language: *l,
                content: (*c).into(),
                name: Some("test".to_owned()),
                description: None,
            }
            .create(("".to_owned(), "".to_owned(), "".to_owned()), db.clone())
            .await
            .unwrap();

            assert_eq!(idx + 1, id as usize);
        }

        // Wait for 4 jobs.
        // Creating those dependencies will trigger redeployment of all scripts in workspace_dependencies_leafs.sql
        in_test_worker(
            db,
            async {
                completed.next().await;
                completed.next().await;
                completed.next().await;
                // completed.next().await;
            },
            port,
        )
        .await;

        // Verify all scripts have correct locks
        // let mut langs = vec![];
        // for r in sqlx::query!(
        //     r#"SELECT language AS "language: ScriptLang",lock FROM script WHERE archived = false"#
        // )
        // .fetch_all(db)
        // .await
        // .unwrap()
        // {
        //     match r.language {
        //         ScriptLang::Python3 => assert_eq!("", &r.lock.unwrap()),
        //         ScriptLang::Go => todo!(),
        //         ScriptLang::Bun => todo!(),
        //         ScriptLang::Bunnative => todo!(),
        //         ScriptLang::Php => todo!(),
        //         _ => panic!("Unsupported language"),
        //     }

        //     langs.push(r.language);
        // }

        // langs.sort();
        // // Just tiny additional verification for peace of mind.
        // assert_eq!(langs.as_slice(), &[]);

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
