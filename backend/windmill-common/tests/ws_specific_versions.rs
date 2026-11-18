//! Executes `list_ws_specific_versions`, the workspace-specific resource/variable resolver.
//!
//! Nothing in the repo calls this function, and plpgsql defers everything past a raw parse to the
//! first call, so without a test that invokes it a broken body ships looking healthy.
//!
//! Run with:
//!   cargo test -p windmill-common --test ws_specific_versions

use sqlx::{Pool, Postgres};

async fn insert_ws(db: &Pool<Postgres>, id: &str) {
    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ($1, $1, 'test-user')")
        .bind(id)
        .execute(db)
        .await
        .expect("insert workspace");
    sqlx::query(
        "INSERT INTO resource (workspace_id, path, value, resource_type) \
         VALUES ($1, 'u/admin/shared', '{}'::jsonb, 'postgresql')",
    )
    .bind(id)
    .execute(db)
    .await
    .expect("seed resource");
}

/// The traversal a prod/dev pair needs: each finds the other, while a plain fork under the same
/// prod stays out of the prod's result even though it holds the same path. Descending into plain
/// forks would fan a root out over its whole live fork subtree.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn pairs_prod_and_dev_without_plain_forks(db: Pool<Postgres>) {
    for id in ["lwp", "lwd", "lwf"] {
        insert_ws(&db, id).await;
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

    // The base fixture's caller is a superadmin, so the function synthesises an admin identity and
    // the RLS probe runs unrestricted without per-workspace `usr` rows.
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

    assert_eq!(versions("lwp").await, vec!["lwd", "lwp"]);
    assert_eq!(versions("lwd").await, vec!["lwd", "lwp"]);
    assert_eq!(versions("lwf").await, vec!["lwd", "lwf", "lwp"]);
}
