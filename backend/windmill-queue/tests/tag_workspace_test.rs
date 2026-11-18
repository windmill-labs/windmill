//! Regression guard for the workspace id that job tags resolve to.
//!
//! An ephemeral fork borrows its parent's id (no worker is ever provisioned for a `wm-fork-*` id),
//! but a dev workspace keeps its own — it is long-lived and may already have dedicated workers, so
//! attaching one must not move its jobs onto the parent's pool.
//!
//! Run with:
//!   cargo test -p windmill-queue --test tag_workspace_test

use sqlx::{Pool, Postgres};
use windmill_common::worker::Connection;
use windmill_common::workspaces::root_workspace_id;
use windmill_queue::tags::{
    apply_fork_lineage_change, invalidate_fork_parent_cache, tag_workspace_id,
};

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
    invalidate_fork_parent_cache(id);
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn tag_workspace_id_resolves_to_the_nearest_servable_ancestor(db: Pool<Postgres>) {
    insert_ws(&db, "twt-prod", None, false).await;
    insert_ws(&db, "wm-fork-twt", Some("twt-prod"), false).await;
    insert_ws(&db, "twt-dev", Some("twt-prod"), true).await;
    // A fork of a fork: the intermediate id is generated too, so it cannot end the walk.
    insert_ws(&db, "wm-fork-nested", Some("wm-fork-twt"), false).await;
    // A generated-id workspace re-designated as a dev workspace: still nothing serves its own id.
    // Under its own root, since only one dev workspace is allowed per parent.
    insert_ws(&db, "twt-prod2", None, false).await;
    insert_ws(&db, "wm-fork-asdev", Some("twt-prod2"), true).await;
    // A fork under a genuine dev workspace stops there — that workspace has its own workers.
    insert_ws(&db, "wm-fork-underdev", Some("twt-dev"), false).await;

    assert_eq!(tag_workspace_id("twt-prod", &db).await, "twt-prod");
    assert_eq!(tag_workspace_id("wm-fork-twt", &db).await, "twt-prod");
    assert_eq!(tag_workspace_id("twt-dev", &db).await, "twt-dev");
    assert_eq!(tag_workspace_id("wm-fork-nested", &db).await, "twt-prod");
    assert_eq!(tag_workspace_id("wm-fork-asdev", &db).await, "twt-prod2");
    assert_eq!(tag_workspace_id("wm-fork-underdev", &db).await, "twt-dev");
}

/// Fork ids are reclaimable, so a deleted fork's cached mapping must not outlive it. The deleting
/// process invalidates locally, but every other replica only learns through the broadcast; without
/// it, a job pushed in the recreated fork routes to the previous parent's tag for the whole TTL.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn reclaimed_fork_id_follows_its_new_parent(db: Pool<Postgres>) {
    insert_ws(&db, "rc-first", None, false).await;
    insert_ws(&db, "rc-second", None, false).await;
    insert_ws(&db, "wm-fork-rc", Some("rc-first"), false).await;

    // Warm the mapping the way a job push would.
    assert_eq!(tag_workspace_id("wm-fork-rc", &db).await, "rc-first");

    // The id is freed and claimed again under a different parent. No local invalidation here: this
    // stands in for a replica that did not handle the delete.
    sqlx::query("DELETE FROM workspace WHERE id = 'wm-fork-rc'")
        .execute(&db)
        .await
        .expect("delete fork");
    sqlx::query(
        "INSERT INTO workspace (id, name, owner, parent_workspace_id) \
         VALUES ('wm-fork-rc', 'wm-fork-rc', 'test-user', 'rc-second')",
    )
    .execute(&db)
    .await
    .expect("reclaim fork id");

    // Until the broadcast lands the stale ancestor survives — that is the bug the broadcast exists
    // to close, so pin it rather than leave the mechanism untested.
    assert_eq!(tag_workspace_id("wm-fork-rc", &db).await, "rc-first");

    apply_fork_lineage_change("wm-fork-rc");
    assert_eq!(tag_workspace_id("wm-fork-rc", &db).await, "rc-second");
}

/// `WM_ROOT_WORKSPACE` answers the same walk up the ancestor chain as the tag workspace, so its
/// cache rides these two sweeps instead of carrying call sites of its own. That is only true while
/// they actually drop it: dropping either line below leaves a job reporting an environment its
/// workspace left minutes earlier, and no test in `windmill-common` can catch it — that crate
/// cannot depend on this one.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn the_sweeps_here_also_drop_the_root_workspace(db: Pool<Postgres>) {
    let conn = Connection::Sql(db.clone());
    insert_ws(&db, "rwx-prod", None, false).await;
    insert_ws(&db, "rwx-mid", Some("rwx-prod"), false).await;
    insert_ws(&db, "wm-fork-rwx", Some("rwx-mid"), false).await;

    // Warm both caches the way pushing and then starting a job would.
    assert_eq!(root_workspace_id(&conn, "wm-fork-rwx").await, "rwx-prod");

    // Promoting a mid-chain workspace to a dev workspace is the mutation that moves the answer for
    // its whole subtree. Done in SQL because the handler that does it lives in the API crate.
    sqlx::query("UPDATE workspace SET is_dev_workspace = true WHERE id = 'rwx-mid'")
        .execute(&db)
        .await
        .expect("promote to dev workspace");

    invalidate_fork_parent_cache("wm-fork-rwx");
    assert_eq!(root_workspace_id(&conn, "wm-fork-rwx").await, "rwx-mid");

    // And again through the broadcast leg, which is the only one a replica ever sees. Only the
    // single-id payload is exercised: `"*"` clears the process-global caches, which would yank the
    // warmed entries out from under the other tests running in this same process.
    sqlx::query("UPDATE workspace SET is_dev_workspace = false WHERE id = 'rwx-mid'")
        .execute(&db)
        .await
        .expect("demote dev workspace");

    apply_fork_lineage_change("wm-fork-rwx");
    assert_eq!(root_workspace_id(&conn, "wm-fork-rwx").await, "rwx-prod");
}
