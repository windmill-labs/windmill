//! Regression guard for the workspace a job reports as `WM_ROOT_WORKSPACE`.
//!
//! Run with:
//!   cargo test -p windmill-common --test root_workspace

use sqlx::{Pool, Postgres};
use windmill_common::worker::Connection;
use windmill_common::workspaces::{invalidate_root_workspace_cache, root_workspace_id};

async fn insert_ws(db: &Pool<Postgres>, id: &str, parent: Option<&str>, is_dev: bool) {
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id, is_dev_workspace)
         VALUES ($1, $1, 'test-user', $2, $3)",
    )
    .bind(id)
    .bind(parent)
    .bind(is_dev)
    .execute(db)
    .await
    .expect("insert workspace");
    // The resolver caches per id in a process-global cache shared across tests.
    invalidate_root_workspace_cache(id);
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn root_workspace_is_the_nearest_dev_or_prod_ancestor(db: Pool<Postgres>) {
    let conn = Connection::Sql(db.clone());
    insert_ws(&db, "rwt-prod", None, false).await;
    insert_ws(&db, "rwt-dev", Some("rwt-prod"), true).await;
    insert_ws(&db, "wm-fork-rwt", Some("rwt-prod"), false).await;
    insert_ws(&db, "wm-fork-underdev", Some("rwt-dev"), false).await;
    insert_ws(&db, "wm-fork-nested", Some("wm-fork-underdev"), false).await;
    // A generated-id workspace re-designated as a dev workspace, under its own root since only one
    // dev workspace is allowed per parent.
    insert_ws(&db, "rwt-prod2", None, false).await;
    insert_ws(&db, "wm-fork-asdev", Some("rwt-prod2"), true).await;
    insert_ws(&db, "wm-fork-asdev-fork", Some("wm-fork-asdev"), false).await;

    assert_eq!(root_workspace_id(&conn, "rwt-prod").await, "rwt-prod");
    assert_eq!(root_workspace_id(&conn, "rwt-dev").await, "rwt-dev");
    assert_eq!(root_workspace_id(&conn, "wm-fork-rwt").await, "rwt-prod");
    assert_eq!(
        root_workspace_id(&conn, "wm-fork-underdev").await,
        "rwt-dev"
    );
    assert_eq!(root_workspace_id(&conn, "wm-fork-nested").await, "rwt-dev");
    // A dev workspace ends the walk whatever its id. Tag resolution deliberately walks past a
    // generated-id dev workspace because nothing provisions workers for such an id, but the
    // environment a fork belongs to is still that workspace.
    assert_eq!(
        root_workspace_id(&conn, "wm-fork-asdev").await,
        "wm-fork-asdev"
    );
    assert_eq!(
        root_workspace_id(&conn, "wm-fork-asdev-fork").await,
        "wm-fork-asdev"
    );
    // An id with no lineage to resolve is its own environment.
    assert_eq!(root_workspace_id(&conn, "rwt-missing").await, "rwt-missing");
}

/// The resolver's TTL is long because correctness rests on the sweep instead: every lineage
/// mutation reaches this cache through `windmill_queue::tags::invalidate_fork_parent_cache`. Pin
/// that the sweep is what actually refreshes the answer, so the wiring cannot be dropped and leave
/// a stale environment reported for minutes.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn attaching_a_dev_workspace_takes_effect_on_invalidation(db: Pool<Postgres>) {
    let conn = Connection::Sql(db.clone());
    insert_ws(&db, "rwi-prod", None, false).await;
    insert_ws(&db, "rwi-mid", Some("rwi-prod"), false).await;
    insert_ws(&db, "wm-fork-rwi", Some("rwi-mid"), false).await;

    // Warm the entry the way a job start would.
    assert_eq!(root_workspace_id(&conn, "wm-fork-rwi").await, "rwi-prod");

    sqlx::query("UPDATE workspace SET is_dev_workspace = true WHERE id = 'rwi-mid'")
        .execute(&db)
        .await
        .expect("promote to dev workspace");

    // Until the sweep lands the fork still reports the old environment; that staleness is exactly
    // what the invalidation exists to close.
    assert_eq!(root_workspace_id(&conn, "wm-fork-rwi").await, "rwi-prod");

    invalidate_root_workspace_cache("wm-fork-rwi");
    assert_eq!(root_workspace_id(&conn, "wm-fork-rwi").await, "rwi-mid");
}
