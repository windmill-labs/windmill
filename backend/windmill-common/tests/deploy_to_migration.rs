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
        ("cycroot", false),
        ("wm-fork-cyc", false), // existing fork of cycroot; cycroot -> it would close a loop
        ("gone", true),         // archived source: its link must still be recorded
        ("seeded", false),      // existing fork whose deploy_to is just the seeded parent
        ("otherprod", false),
        ("ownersrc", false), // root that already owns a dev workspace
        ("ownerdev", false),
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
        ("cycroot", Some("wm-fork-cyc")),
        ("wm-fork-cyc", None),
        ("gone", Some("prod")),
        ("seeded", Some("prod")),
        ("otherprod", None),
        ("ownersrc", Some("otherprod")),
        ("ownerdev", None),
    ] {
        set_deploy_to(&db, id, target).await;
    }

    // Pre-existing lineage the migration must respect rather than loop through.
    for (child, parent) in [("wm-fork-cyc", "cycroot"), ("seeded", "prod")] {
        sqlx::query("UPDATE workspace SET parent_workspace_id = $2 WHERE id = $1")
            .bind(child)
            .bind(parent)
            .execute(&db)
            .await
            .expect("seed lineage");
    }
    sqlx::query(
        "UPDATE workspace SET parent_workspace_id = 'ownersrc', is_dev_workspace = true \
         WHERE id = 'ownerdev'",
    )
    .execute(&db)
    .await
    .expect("seed existing dev pairing");

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
            // Linking cycroot to its own fork would close a loop no deploy_to edge reveals.
            ("cycroot".to_string(), "wm-fork-cyc".to_string()),
            // An archived source still had a real link; it must not vanish with the column.
            ("gone".to_string(), "prod".to_string()),
            // Converting this would leave its dev workspace nested under a fork, which
            // attach_dev_workspace refuses to create.
            ("ownersrc".to_string(), "otherprod".to_string()),
            ("selfref".to_string(), "selfref".to_string()),
            ("stale".to_string(), "prod".to_string()),
        ]
    );
    assert_eq!(lineage(&db, "selfref").await, (None, false));
    // cycroot keeps its own fork and gains no parent of its own.
    assert_eq!(lineage(&db, "cycroot").await, (None, false));
    // The source keeps its own dev workspace and gains no parent of its own.
    assert_eq!(lineage(&db, "ownersrc").await, (None, false));
    assert_eq!(
        lineage(&db, "ownerdev").await,
        (Some("ownersrc".to_string()), true)
    );
    // A fork whose deploy_to merely repeated its parent loses nothing, so it is not reported.
    assert_eq!(
        lineage(&db, "seeded").await,
        (Some("prod".to_string()), false)
    );

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

/// The outcome almost every instance sees: every link converts, so the migration leaves no
/// leftovers table behind and rollback still works without one.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn clean_conversion_leaves_no_leftovers_table(db: Pool<Postgres>) {
    db.execute(DOWN).await.expect("restore pre-migration shape");

    insert_ws(&db, "cprod", false).await;
    insert_ws(&db, "cstaging", false).await;
    set_deploy_to(&db, "cprod", None).await;
    set_deploy_to(&db, "cstaging", Some("cprod")).await;

    db.execute(UP).await.expect("run unification migration");

    assert_eq!(
        lineage(&db, "cstaging").await,
        (Some("cprod".to_string()), true)
    );
    let table_exists: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('workspace_deploy_to_unmigrated')::text")
            .fetch_one(&db)
            .await
            .expect("probe leftovers table");
    assert_eq!(table_exists, None, "empty leftovers table must not survive");

    db.execute(DOWN).await.expect("roll back without the table");
}

/// Executes the rewritten `list_ws_specific_versions`. plpgsql defers everything past a raw parse
/// to the first call, so replaying the migration only proves the body parses — this calls it.
///
/// Asserts the traversal a prod/dev pair needs: each finds the other, while a plain fork under the
/// same prod stays out of the prod's result even though it holds the same path.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn ws_specific_versions_pairs_prod_and_dev_without_plain_forks(db: Pool<Postgres>) {
    for id in ["lwp", "lwd", "lwf"] {
        insert_ws(&db, id, false).await;
        // The caller is a superadmin in the base fixture, so no per-workspace usr row is needed:
        // the function synthesises an admin identity and the RLS probe runs unrestricted.
        sqlx::query(
            "INSERT INTO resource (workspace_id, path, value, resource_type) \
             VALUES ($1, 'u/admin/shared', '{}'::jsonb, 'postgresql')",
        )
        .bind(id)
        .execute(&db)
        .await
        .expect("seed resource");
    }
    sqlx::query(
        "UPDATE workspace SET parent_workspace_id = 'lwp', is_dev_workspace = true WHERE id = 'lwd'",
    )
    .execute(&db)
    .await
    .expect("attach dev");
    sqlx::query("UPDATE workspace SET parent_workspace_id = 'lwp' WHERE id = 'lwf'")
        .execute(&db)
        .await
        .expect("attach fork");

    let versions = |seed: &'static str| {
        let db = db.clone();
        async move {
            let mut rows: Vec<String> = sqlx::query_scalar(
                "SELECT ws FROM list_ws_specific_versions($1, 'test@windmill.dev', 'resource', 'u/admin/shared')",
            )
            .bind(seed)
            .fetch_all(&db)
            .await
            .expect("call list_ws_specific_versions");
            rows.sort();
            rows
        }
    };

    // A prod reaches its dev workspace but not its throwaway forks.
    assert_eq!(versions("lwp").await, vec!["lwd", "lwp"]);
    // The dev workspace reaches back up to prod.
    assert_eq!(versions("lwd").await, vec!["lwd", "lwp"]);
    // A plain fork still sees its ancestors, and the dev workspace hanging off them.
    assert_eq!(versions("lwf").await, vec!["lwd", "lwf", "lwp"]);
}
