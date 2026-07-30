//! Covers the one-shot migration that folded `workspace_settings.deploy_to` into the fork lineage.
//!
//! `sqlx::test` hands us an already-migrated database, so the test replays the real migration files
//! rather than a copy of their SQL: run the down migration to restore the pre-unification shape,
//! seed the legacy rows, then run the up migration and assert what it produced. That also exercises
//! the rollback path, which is the only way a skipped link is recoverable.
//!
//! Run with:
//!   cargo test -p windmill-common --test deploy_to_migration

use sqlx::{Executor, Pool, Postgres};

const UP: &str = include_str!("../../migrations/20260730080304_unify_deploy_to_into_parent.up.sql");
const DOWN: &str =
    include_str!("../../migrations/20260730080304_unify_deploy_to_into_parent.down.sql");

async fn insert_ws(db: &Pool<Postgres>, id: &str, deleted: bool) {
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, deleted) VALUES ($1, $1, 'test-user', $2)",
    )
    .bind(id)
    .bind(deleted)
    .execute(db)
    .await
    .expect("insert workspace");
}

async fn set_deploy_to(db: &Pool<Postgres>, id: &str, deploy_to: Option<&str>) {
    sqlx::query("INSERT INTO workspace_settings (workspace_id, deploy_to) VALUES ($1, $2)")
        .bind(id)
        .bind(deploy_to)
        .execute(db)
        .await
        .expect("insert workspace_settings");
}

async fn lineage(db: &Pool<Postgres>, id: &str) -> (Option<String>, bool) {
    sqlx::query_as("SELECT parent_workspace_id, is_dev_workspace FROM workspace WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await
        .expect("read lineage")
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn deploy_to_folds_into_lineage(db: Pool<Postgres>) {
    db.execute(DOWN).await.expect("restore pre-migration shape");

    for (id, deleted) in [
        ("prod", false),
        ("staging", false),
        ("stale", true), // archived claimant on prod: must not demote the live pair
        ("shared", false),
        ("a1", false),
        ("a2", false), // two live claimants on `shared`
        ("cprod", false),
        ("cstg", false),
        ("cdev", false), // cdev -> cstg -> cprod
        ("selfref", false),
        ("cyc1", false),
        ("cyc2", false),
    ] {
        insert_ws(&db, id, deleted).await;
    }
    for (id, target) in [
        ("prod", None),
        ("staging", Some("prod")),
        ("stale", Some("prod")),
        ("shared", None),
        ("a1", Some("shared")),
        ("a2", Some("shared")),
        ("cprod", None),
        ("cstg", Some("cprod")),
        ("cdev", Some("cstg")),
        ("selfref", Some("selfref")),
        ("cyc1", Some("cyc2")),
        ("cyc2", Some("cyc1")),
    ] {
        set_deploy_to(&db, id, target).await;
    }

    db.execute(UP).await.expect("run unification migration");

    // Sole live claimant on a free root keeps its own identity as that root's dev workspace. An
    // archived claimant must not count, or the live pair silently degrades to a plain fork.
    assert_eq!(
        lineage(&db, "staging").await,
        (Some("prod".to_string()), true)
    );
    // Several live claimants cannot all be the one dev workspace, so all of them become forks.
    assert_eq!(
        lineage(&db, "a1").await,
        (Some("shared".to_string()), false)
    );
    assert_eq!(
        lineage(&db, "a2").await,
        (Some("shared".to_string()), false)
    );
    // A chain is representable: each link converts, and only the one rooted at a free root is dev.
    assert_eq!(
        lineage(&db, "cstg").await,
        (Some("cprod".to_string()), true)
    );
    assert_eq!(
        lineage(&db, "cdev").await,
        (Some("cstg".to_string()), false)
    );

    // What the lineage cannot express is preserved rather than dropped with the column.
    let preserved: Vec<(String, String)> = sqlx::query_as(
        "SELECT workspace_id, deploy_to FROM workspace_deploy_to_unmigrated ORDER BY workspace_id",
    )
    .fetch_all(&db)
    .await
    .expect("read preserved links");
    assert_eq!(
        preserved,
        vec![
            ("cyc1".to_string(), "cyc2".to_string()),
            ("cyc2".to_string(), "cyc1".to_string()),
            ("selfref".to_string(), "selfref".to_string()),
        ]
    );
    assert_eq!(lineage(&db, "selfref").await, (None, false));

    // Rollback restores every link, including the ones that could not convert.
    db.execute(DOWN).await.expect("roll back");
    let restored: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT workspace_id, deploy_to FROM workspace_settings
         WHERE workspace_id IN ('staging', 'selfref', 'cyc1') ORDER BY workspace_id",
    )
    .fetch_all(&db)
    .await
    .expect("read restored");
    assert_eq!(
        restored,
        vec![
            ("cyc1".to_string(), Some("cyc2".to_string())),
            ("selfref".to_string(), Some("selfref".to_string())),
            ("staging".to_string(), Some("prod".to_string())),
        ]
    );
}
