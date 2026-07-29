//! What a caller who cannot see a dbt script gets from a run of it.
//!
//! The share-link case: `require_job_read_access` has already let them through
//! to the job, so `asset_graph_for` is handed the run — but the project itself
//! is not theirs to read. The graph's SHAPE must survive that and its model SQL
//! must not, and the two are decided by different predicates. Resolving the
//! version from the caller's `script` access instead of from the job emptied the
//! whole dbt half, and every later fix in this area re-touched one of the two.

use sqlx::{Pool, Postgres};
use windmill_api_assets::{asset_graph_for, GraphQuery, PinnedRun};
use windmill_api_auth::ApiAuthed;
use windmill_common::db::UserDB;

const WS: &str = "test-workspace";
const PATH: &str = "f/private/proj";
const HASH: i64 = 42;

/// A member of the workspace with no grant on `f/private` — the reason someone
/// is sent a share link in the first place.
fn outsider() -> ApiAuthed {
    ApiAuthed {
        email: "outsider@windmill.dev".to_string(),
        username: "outsider".to_string(),
        is_admin: false,
        is_operator: false,
        groups: vec![],
        folders: vec![],
        scopes: None,
        username_override: None,
        token_prefix: None,
        read_only: false,
    }
}

async fn seed(db: &Pool<Postgres>, job: uuid::Uuid) {
    sqlx::query!(
        "INSERT INTO folder (workspace_id, name, display_name, owners, extra_perms)
         VALUES ($1, 'private', 'private', '{}', '{}')",
        WS
    )
    .execute(db)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content,
                             created_by, language, lock)
         VALUES ($1, $2, $3, '', '', 'profile: {}', 'test-user', 'dbt', '')",
        WS,
        HASH,
        PATH,
    )
    .execute(db)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO dbt_graph_snapshot (workspace_id, script_path, script_hash, job_id, digest)
         VALUES ($1, $2, $3, $4, 'd')",
        WS,
        PATH,
        HASH,
        job
    )
    .execute(db)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO dbt_node (workspace_id, script_path, script_hash, job_id, unique_id,
                               resource_type, name, asset_path, raw_code, tags)
         VALUES ($1, $2, $3, $4, 'model.p.orders', 'model', 'orders',
                 'u/a/wh/analytics/orders', 'select 1', '{}')",
        WS,
        PATH,
        HASH,
        job
    )
    .execute(db)
    .await
    .unwrap();
}

fn query() -> GraphQuery {
    GraphQuery { asset_kinds: Some("table".to_string()), folder: None, dbt_script_hash: None }
}

/// Pinned to a run they are entitled to, the outsider gets the model — and not
/// its SQL. Both halves matter: dropping the first is the blank Models panel
/// under working progress rows, dropping the second hands out the project's
/// source to anyone holding a link.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_pinned_run_survives_no_access_to_its_script(db: Pool<Postgres>) {
    let job = uuid::Uuid::from_u128(7);
    seed(&db, job).await;

    let pinned = PinnedRun { job_id: job, script_path: PATH.to_string(), script_hash: HASH };
    let res = asset_graph_for(
        &outsider(),
        WS,
        UserDB::new(db.clone()),
        db.clone(),
        query(),
        Some(pinned),
    )
    .await
    .unwrap();
    let body = serde_json::to_value(&res.0).unwrap();

    let nodes = body["dbt_nodes"]
        .as_array()
        .or(body["assets"].as_array())
        .unwrap();
    assert!(
        !nodes.is_empty(),
        "a run the caller may read must render, whatever their access to the project: {body}"
    );
    assert_eq!(
        body["dbt_snapshot_job"],
        serde_json::json!(job),
        "and the marker must agree with it, or the page polls the graph 40 times"
    );
    assert!(
        !body.to_string().contains("select 1"),
        "the model's SQL stays behind access to the script: {body}"
    );
}

/// Unpinned, the same caller sees nothing of the project: the workspace graph
/// answers for their own access, and this one is not theirs. This is the half
/// that must NOT be relaxed by making the pinned case work.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn without_a_run_the_same_caller_sees_no_dbt_nodes(db: Pool<Postgres>) {
    let job = uuid::Uuid::from_u128(7);
    seed(&db, job).await;

    let res = asset_graph_for(
        &outsider(),
        WS,
        UserDB::new(db.clone()),
        db.clone(),
        query(),
        None,
    )
    .await
    .unwrap();
    let body = serde_json::to_value(&res.0).unwrap();
    assert!(
        !body.to_string().contains("orders"),
        "an unpinned graph is the caller's own view of the workspace: {body}"
    );
}

/// Archiving retires the script; it must not retire the runs that already
/// happened. The pinned read resolves versions through a CTE that skips archived
/// rows, so nothing about the CURRENT workspace graph depends on the sidecar
/// surviving — which is exactly why deleting it looks safe and is not.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn archiving_the_script_leaves_its_finished_runs_renderable(db: Pool<Postgres>) {
    let job = uuid::Uuid::from_u128(7);
    seed(&db, job).await;
    sqlx::query!(
        "UPDATE script SET archived = true WHERE workspace_id = $1 AND hash = $2",
        WS,
        HASH
    )
    .execute(&db)
    .await
    .unwrap();

    let admin = ApiAuthed { is_admin: true, ..outsider() };
    let pinned = PinnedRun { job_id: job, script_path: PATH.to_string(), script_hash: HASH };
    let res = asset_graph_for(
        &admin,
        WS,
        UserDB::new(db.clone()),
        db.clone(),
        query(),
        Some(pinned),
    )
    .await
    .unwrap();
    assert!(
        serde_json::to_value(&res.0).unwrap().to_string().contains("orders"),
        "a completed run of an archived version still renders its models"
    );

    let now = asset_graph_for(&admin, WS, UserDB::new(db.clone()), db.clone(), query(), None)
        .await
        .unwrap();
    assert!(
        !serde_json::to_value(&now.0).unwrap().to_string().contains("model.p.orders"),
        "while the workspace graph, which describes what is live, drops it"
    );
}
