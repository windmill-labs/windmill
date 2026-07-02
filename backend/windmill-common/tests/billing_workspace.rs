#![cfg(feature = "cloud")]
//! Tests for the fork/dev "billing workspace" resolution and the fork-cap seat/count helpers.

use sqlx::{Pool, Postgres};
use windmill_common::workspaces::{
    count_paid_seats, count_workspace_forks, fork_chain_depth, fork_subtree_height,
    get_billing_workspace_id, invalidate_billing_workspace_cache,
};

async fn insert_ws(db: &Pool<Postgres>, id: &str, parent: Option<&str>, deleted: bool) {
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id, deleted)
         VALUES ($1, $1, 'test-user', $2, $3)",
    )
    .bind(id)
    .bind(parent)
    .bind(deleted)
    .execute(db)
    .await
    .expect("insert workspace");
    // The resolver caches per id (60s TTL) in a process-global cache shared across tests, so drop
    // any stale mapping for this id before the test reads it.
    invalidate_billing_workspace_cache(id);
}

async fn insert_member(
    db: &Pool<Postgres>,
    w_id: &str,
    email: &str,
    operator: bool,
    disabled: bool,
    is_service_account: bool,
) {
    // `usr.username` has a `proper_username` check (no `@`), so derive one from the email prefix.
    let username = email.split('@').next().unwrap();
    sqlx::query(
        "INSERT INTO usr (workspace_id, email, username, is_admin, operator, disabled, is_service_account, role)
         VALUES ($1, $2, $3, false, $4, $5, $6, 'Developer')",
    )
    .bind(w_id)
    .bind(email)
    .bind(username)
    .bind(operator)
    .bind(disabled)
    .bind(is_service_account)
    .execute(db)
    .await
    .expect("insert usr");
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn billing_workspace_resolves_to_root(db: Pool<Postgres>) {
    insert_ws(&db, "bwt-root", None, false).await;
    insert_ws(&db, "bwt-fork", Some("bwt-root"), false).await;
    insert_ws(&db, "bwt-grandchild", Some("bwt-fork"), false).await;

    assert_eq!(
        get_billing_workspace_id(&db, "bwt-root").await.unwrap(),
        "bwt-root"
    );
    assert_eq!(
        get_billing_workspace_id(&db, "bwt-fork").await.unwrap(),
        "bwt-root"
    );
    assert_eq!(
        get_billing_workspace_id(&db, "bwt-grandchild")
            .await
            .unwrap(),
        "bwt-root"
    );
    // Unknown / orphaned ids resolve to themselves.
    assert_eq!(
        get_billing_workspace_id(&db, "bwt-missing").await.unwrap(),
        "bwt-missing"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn billing_workspace_survives_cycles(db: Pool<Postgres>) {
    insert_ws(&db, "bwc-a", None, false).await;
    insert_ws(&db, "bwc-b", Some("bwc-a"), false).await;
    // Introduce a cycle a -> b -> a; no row has a NULL parent, so resolution falls back to the input
    // and the depth guard keeps it from looping forever.
    sqlx::query("UPDATE workspace SET parent_workspace_id = 'bwc-b' WHERE id = 'bwc-a'")
        .execute(&db)
        .await
        .unwrap();
    invalidate_billing_workspace_cache("bwc-a");
    invalidate_billing_workspace_cache("bwc-b");

    assert_eq!(
        get_billing_workspace_id(&db, "bwc-a").await.unwrap(),
        "bwc-a"
    );
    assert_eq!(
        get_billing_workspace_id(&db, "bwc-b").await.unwrap(),
        "bwc-b"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn paid_seats_and_fork_count(db: Pool<Postgres>) {
    insert_ws(&db, "seat-root", None, false).await;
    // 2 developers + 2 operators counted -> ceil(2 + 0.5*2) = 3.
    insert_member(&db, "seat-root", "dev1@w.dev", false, false, false).await;
    insert_member(&db, "seat-root", "dev2@w.dev", false, false, false).await;
    insert_member(&db, "seat-root", "op1@w.dev", true, false, false).await;
    insert_member(&db, "seat-root", "op2@w.dev", true, false, false).await;
    // These must NOT count towards seats.
    insert_member(&db, "seat-root", "disabled@w.dev", false, true, false).await;
    insert_member(&db, "seat-root", "svc@w.dev", false, false, true).await;

    assert_eq!(count_paid_seats(&db, "seat-root").await.unwrap(), 3);

    insert_ws(&db, "seat-fork1", Some("seat-root"), false).await;
    insert_ws(&db, "seat-fork2", Some("seat-root"), false).await;
    // A deleted fork itself is not counted...
    insert_ws(&db, "seat-fork-deleted", Some("seat-root"), true).await;
    // ...but a live sub-fork under it still is (deleted filter is on the outer SELECT, not the walk).
    insert_ws(&db, "seat-deleted-child", Some("seat-fork-deleted"), false).await;
    // A grandchild fork still counts.
    insert_ws(&db, "seat-fork1-child", Some("seat-fork1"), false).await;

    // Live: fork1, fork2, fork1-child, deleted-child -> 4 (seat-fork-deleted excluded).
    assert_eq!(count_workspace_forks(&db, "seat-root").await.unwrap(), 4);
    // A standalone workspace has no forks.
    assert_eq!(count_workspace_forks(&db, "seat-fork2").await.unwrap(), 0);
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn billing_cache_invalidation_reflects_reparent(db: Pool<Postgres>) {
    insert_ws(&db, "inv-root-a", None, false).await;
    insert_ws(&db, "inv-root-b", None, false).await;
    insert_ws(&db, "inv-fork", Some("inv-root-a"), false).await;

    // Resolve + cache: fork -> root-a.
    assert_eq!(
        get_billing_workspace_id(&db, "inv-fork").await.unwrap(),
        "inv-root-a"
    );

    // Reparent in the DB, as delete+recreate-under-another-root (or attach) would.
    sqlx::query("UPDATE workspace SET parent_workspace_id = 'inv-root-b' WHERE id = 'inv-fork'")
        .execute(&db)
        .await
        .unwrap();

    // The cached mapping survives until invalidated (this is the staleness the delete/attach/rename
    // paths must clear).
    assert_eq!(
        get_billing_workspace_id(&db, "inv-fork").await.unwrap(),
        "inv-root-a"
    );

    // After invalidation (what delete_workspace / attach_dev_workspace / change_workspace_id now call),
    // it re-resolves to the new root.
    invalidate_billing_workspace_cache("inv-fork");
    assert_eq!(
        get_billing_workspace_id(&db, "inv-fork").await.unwrap(),
        "inv-root-b"
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn fork_depth_and_subtree_height(db: Pool<Postgres>) {
    // Chain: root -> f1 -> f2 -> f3
    insert_ws(&db, "fd-root", None, false).await;
    insert_ws(&db, "fd-f1", Some("fd-root"), false).await;
    insert_ws(&db, "fd-f2", Some("fd-f1"), false).await;
    insert_ws(&db, "fd-f3", Some("fd-f2"), false).await;

    // Depth walks up to the root: root = 0, each fork adds one.
    assert_eq!(fork_chain_depth(&db, "fd-root").await.unwrap(), 0);
    assert_eq!(fork_chain_depth(&db, "fd-f1").await.unwrap(), 1);
    assert_eq!(fork_chain_depth(&db, "fd-f3").await.unwrap(), 3);
    // An unknown id has no chain, so depth 0 (treated as a root by the guard).
    assert_eq!(fork_chain_depth(&db, "fd-missing").await.unwrap(), 0);

    // Height walks down: the deepest live descendant below the node.
    assert_eq!(fork_subtree_height(&db, "fd-root").await.unwrap(), 3);
    assert_eq!(fork_subtree_height(&db, "fd-f2").await.unwrap(), 1);
    assert_eq!(fork_subtree_height(&db, "fd-f3").await.unwrap(), 0);

    // A deleted leaf doesn't add to the height.
    insert_ws(&db, "fd-f3-del", Some("fd-f3"), true).await;
    assert_eq!(fork_subtree_height(&db, "fd-f3").await.unwrap(), 0);

    // ...but a LIVE descendant below a soft-deleted intermediate still counts at its true depth
    // (the walk traverses through the deleted node; only the aggregation filters deleted).
    insert_ws(&db, "fd-f3-live-gc", Some("fd-f3-del"), false).await;
    assert_eq!(fork_subtree_height(&db, "fd-f3").await.unwrap(), 2);
}
