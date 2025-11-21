mod common;
use common::in_test_worker;
use common::init_client;
use common::listen_for_completed_jobs;
use sqlx::{Pool, Postgres};
use tokio_stream::StreamExt;
use windmill_common::scripts::ScriptLang;
use windmill_worker::workspace_dependencies::NewWorkspaceDependencies;

mod deps {
    pub const REQUIREMENTS_IN: &'static str = "tiny==0.1.3";
    pub const GO_MOD: &'static str = r##"
    module example.com/project

    go 1.20

    require github.com/gin-gonic/gin v1.8.1
    "##;

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
async fn basic_manual_named(db: Pool<Postgres>) -> anyhow::Result<()> {
    use common::RunJob;

    let ((_client, port, _s), db, mut completed) = (
        init_client(db.clone()).await,
        &db,
        listen_for_completed_jobs(&db).await,
    );

    for (idx, (l, c)) in [
        (ScriptLang::Python3, deps::REQUIREMENTS_IN),
        // TODO: Should test deno?
        (ScriptLang::Bun, deps::PACKAGE_JSON),
        (ScriptLang::Php, deps::COMPOSER_JSON),
        (ScriptLang::Go, deps::GO_MOD),
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
        .create(db)
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
            completed.next().await;
        },
        port,
    )
    .await;

    // Verify all scripts have correct locks
    let mut langs = vec![];
    for r in sqlx::query!(
        r#"SELECT language AS "language: ScriptLang",lock FROM script WHERE archived = false"#
    )
    .fetch_all(db)
    .await
    .unwrap()
    {
        match r.language {
            ScriptLang::Deno => todo!(),
            ScriptLang::Python3 => assert_eq!("", &r.lock.unwrap()),
            ScriptLang::Go => todo!(),
            ScriptLang::Bun => todo!(),
            ScriptLang::Bunnative => todo!(),
            ScriptLang::Php => todo!(),
            _ => panic!("Unsupported language"),
        }

        langs.push(r.language);
    }

    langs.sort();
    // Just tiny additional verification for peace of mind.
    assert_eq!(langs.as_slice(), &[]);

    Ok(())
}
