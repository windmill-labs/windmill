//! Sets the process-wide `DEPENDENCY_JOB_TAG`, so it lives in its own test binary: any other
//! test queuing a dependency job in the same process would land on the overridden tag.

use sqlx::{Pool, Postgres};
use windmill_common::worker::DEPENDENCY_JOB_TAG;
use windmill_dep_map::scoped_dependency_map::DependencyDependent;

#[sqlx::test(fixtures("base"))]
async fn relocks_go_to_the_configured_dependency_job_tag(db: Pool<Postgres>) -> anyhow::Result<()> {
    DEPENDENCY_JOB_TAG.store(std::sync::Arc::new(Some("deps-$workspace".to_string())));

    for (path, hash, tag) in [
        ("f/tags/tagged", 7_100_001_i64, Some("default-$workspace")),
        ("f/tags/plain", 7_100_002_i64, None),
    ] {
        sqlx::query(
            "INSERT INTO script(workspace_id, created_by, content, schema, summary, description, path, hash, language, tag)
             VALUES ('test-workspace', 'test-user', 'def main(): pass', '{}', '', '', $1, $2, 'python3', $3)",
        )
        .bind(path)
        .bind(hash)
        .bind(tag)
        .execute(&db)
        .await?;
    }
    let importers = ["f/tags/tagged", "f/tags/plain"]
        .map(|p| DependencyDependent {
            importer_path: p.to_string(),
            importer_kind: "script".to_string(),
            importer_node_ids: None,
        })
        .to_vec();
    windmill_dep_map::trigger_dependents::trigger_dependents_to_recompute_dependencies(
        "test-workspace",
        importers,
        None,
        None,
        "test@windmill.dev",
        "test-user",
        "u/test-user",
        &db,
        vec![],
    )
    .await?;

    let tags: Vec<(String, String)> = sqlx::query_as(
        "SELECT runnable_path, tag FROM v2_job WHERE kind = 'dependencies' ORDER BY runnable_path",
    )
    .fetch_all(&db)
    .await?;
    assert_eq!(
        tags,
        vec![
            (
                "f/tags/plain".to_string(),
                "deps-test-workspace".to_string()
            ),
            (
                "f/tags/tagged".to_string(),
                "deps-test-workspace".to_string()
            ),
        ]
    );
    Ok(())
}
