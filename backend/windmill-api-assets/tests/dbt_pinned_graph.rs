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
        username_override_is_token_label: false,
        is_session_token: false,
        token_prefix: None,
        read_only: false,
        job_id: None,
        credential_expiry: None,
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
        r#"INSERT INTO dbt_node (workspace_id, script_path, script_hash, job_id, unique_id,
                                 resource_type, name, asset_path, raw_code, tags, description,
                                 columns, freshness)
           VALUES ($1, $2, $3, $4, 'model.p.orders', 'model', 'orders',
                   'u/a/wh/analytics/orders', 'select 1', '{finance}', 'daily order facts',
                   '{"order_id": {"description": "natural key"}}'::jsonb,
                   '{"warn_after": {"count": 12, "period": "hour"}}'::jsonb)"#,
        WS,
        PATH,
        HASH,
        job
    )
    .execute(db)
    .await
    .unwrap();
    // A test node, for the arguments it carries: `accepted_values` spells out a
    // column's domain.
    sqlx::query!(
        r#"INSERT INTO dbt_node (workspace_id, script_path, script_hash, job_id, unique_id,
                                 resource_type, name, tags, test_kind, test_column, test_args,
                                 attached_node)
           VALUES ($1, $2, $3, $4, 'test.p.accepted_values_orders_status', 'test',
                   'accepted_values_orders_status', '{}', 'accepted_values', 'status',
                   '{"values": ["gold", "silver"]}'::jsonb, 'model.p.orders')"#,
        WS,
        PATH,
        HASH,
        job
    )
    .execute(db)
    .await
    .unwrap();
}

/// Everything on a node that the project's author wrote, rather than the shape
/// of the relation it produces.
const AUTHORED: [&str; 5] = [
    "select 1",
    "daily order facts",
    "finance",
    "natural key",
    "gold",
];

fn query() -> GraphQuery {
    GraphQuery { asset_kinds: Some("dbt".to_string()), folder: None, dbt_script_hash: None }
}

/// Pinned to a run they are entitled to, the outsider gets the model — and
/// nothing its author wrote. Both halves matter: dropping the first is the blank
/// Models panel under working progress rows, dropping the second hands the
/// project's source and documentation to anyone holding a link.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_pinned_run_survives_no_access_to_its_script(db: Pool<Postgres>) {
    let job = uuid::Uuid::from_u128(7);
    seed(&db, job).await;

    let pinned = PinnedRun { job_id: job, script_path: PATH.to_string(), script_hash: Some(HASH) };
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
    for authored in AUTHORED {
        assert!(
            !body.to_string().contains(authored),
            "`{authored}` is the project's, and stays behind access to it: {body}"
        );
    }
    assert!(
        body.to_string().contains("u/a/wh/analytics/orders"),
        "while the relation the run wrote is what the page is for: {body}"
    );

    // The same read by someone who may open the project: the gate has to be the
    // caller's access, not a field this endpoint stopped serving.
    let seen = asset_graph_for(
        &ApiAuthed { is_admin: true, ..outsider() },
        WS,
        UserDB::new(db.clone()),
        db.clone(),
        query(),
        Some(PinnedRun { job_id: job, script_path: PATH.to_string(), script_hash: Some(HASH) }),
    )
    .await
    .unwrap();
    let seen = serde_json::to_value(&seen.0).unwrap().to_string();
    for authored in AUTHORED {
        assert!(
            seen.contains(authored),
            "`{authored}` renders for a reader of the project: {seen}"
        );
    }
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
    let pinned = PinnedRun { job_id: job, script_path: PATH.to_string(), script_hash: Some(HASH) };
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
        serde_json::to_value(&res.0)
            .unwrap()
            .to_string()
            .contains("orders"),
        "a completed run of an archived version still renders its models"
    );

    let now = asset_graph_for(
        &admin,
        WS,
        UserDB::new(db.clone()),
        db.clone(),
        query(),
        None,
    )
    .await
    .unwrap();
    assert!(
        !serde_json::to_value(&now.0)
            .unwrap()
            .to_string()
            .contains("model.p.orders"),
        "while the workspace graph, which describes what is live, drops it"
    );
}

/// `extra_perms` is a grant on a ROW, so a path recreated with narrower ones
/// leaves the old version readable to whoever the old row named. The source
/// probe therefore has to name the version it is about to return: matched on
/// the path alone, that stale grant answered for the pinned version's SQL and
/// handed a viewer the body of a project they were never given.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn an_old_grant_at_the_same_path_does_not_expose_a_newer_version(db: Pool<Postgres>) {
    const V2: i64 = 43;
    let v1_job = uuid::Uuid::from_u128(7);
    let v2_job = uuid::Uuid::from_u128(8);
    seed(&db, v1_job).await;

    // The version the outsider WAS granted, archived — the shape the grant
    // outlives the deploy in.
    sqlx::query!(
        r#"UPDATE script SET archived = true, extra_perms = '{"u/outsider": true}'::jsonb
            WHERE workspace_id = $1 AND hash = $2"#,
        WS,
        HASH
    )
    .execute(&db)
    .await
    .unwrap();

    // The version they were not.
    sqlx::query!(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content,
                             created_by, language, lock)
         VALUES ($1, $2, $3, '', '', 'profile: {}', 'test-user', 'dbt', '')",
        WS,
        V2,
        PATH,
    )
    .execute(&db)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO dbt_graph_snapshot (workspace_id, script_path, script_hash, job_id, digest)
         VALUES ($1, $2, $3, $4, 'd2')",
        WS,
        PATH,
        V2,
        v2_job
    )
    .execute(&db)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO dbt_node (workspace_id, script_path, script_hash, job_id, unique_id,
                               resource_type, name, asset_path, raw_code, tags)
         VALUES ($1, $2, $3, $4, 'model.p.orders', 'model', 'orders',
                 'u/a/wh/analytics/orders', 'select 2', '{}')",
        WS,
        PATH,
        V2,
        v2_job
    )
    .execute(&db)
    .await
    .unwrap();

    let pinned = PinnedRun { job_id: v2_job, script_path: PATH.to_string(), script_hash: Some(V2) };
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
    let body = serde_json::to_value(&res.0).unwrap().to_string();

    assert!(
        body.contains("orders"),
        "the run they were given still renders its models: {body}"
    );
    assert!(
        !body.contains("select 2"),
        "but not the SQL of a version their grant never covered: {body}"
    );
}

/// The editor's own graph, keyed to the parse job and to no version.
async fn seed_editor_graph(db: &Pool<Postgres>, job: uuid::Uuid) {
    sqlx::query!(
        "INSERT INTO dbt_graph_snapshot (workspace_id, script_path, script_hash, job_id, digest)
         VALUES ($1, $2, NULL, $3, 'd')",
        WS,
        PATH,
        job
    )
    .execute(db)
    .await
    .unwrap();
    sqlx::query!(
        r#"INSERT INTO dbt_node (workspace_id, script_path, script_hash, job_id, unique_id,
                                 resource_type, name, asset_path, raw_code, tags)
           VALUES ($1, $2, NULL, $3, 'model.p.draft', 'model', 'draft',
                   'u/a/wh/analytics/draft', 'select 3', '{}')"#,
        WS,
        PATH,
        job
    )
    .execute(db)
    .await
    .unwrap();
}

/// A buffer parse is the third provenance: no deployed version behind it, and
/// its models are ones no `asset` row has heard of. It renders when pinned to
/// the job that produced it — the only way in — and its SQL comes with it, since
/// it exists because this caller's own parse job wrote the buffer.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn an_editor_graph_renders_only_through_its_own_job(db: Pool<Postgres>) {
    let job = uuid::Uuid::from_u128(7);
    seed(&db, job).await;
    let parse = uuid::Uuid::from_u128(9);
    seed_editor_graph(&db, parse).await;

    let admin = ApiAuthed { is_admin: true, ..outsider() };
    let pinned = asset_graph_for(
        &admin,
        WS,
        UserDB::new(db.clone()),
        db.clone(),
        query(),
        Some(PinnedRun { job_id: parse, script_path: PATH.to_string(), script_hash: None }),
    )
    .await
    .unwrap();
    let body = serde_json::to_value(&pinned.0).unwrap();
    assert!(
        body.to_string().contains("u/a/wh/analytics/draft"),
        "the buffer's own models render: {body}"
    );
    assert!(
        body.to_string().contains("select 3"),
        "and their SQL, which is what the caller just wrote: {body}"
    );
    assert_eq!(
        body["dbt_snapshot_job"],
        serde_json::json!(parse),
        "labelled as a graph of its own, so the editor can say where it came from"
    );
    assert!(
        !body.to_string().contains("select 1"),
        "and the deployed version's models are not mixed into it: {body}"
    );

    // Through the PATH — which is what the workspace graph and every run of the
    // deployed version ask for — a buffer parse must not appear at all. It
    // describes an editor's unsaved state, not what the script owns.
    let workspace = asset_graph_for(
        &admin,
        WS,
        UserDB::new(db.clone()),
        db.clone(),
        query(),
        None,
    )
    .await
    .unwrap();
    let workspace = serde_json::to_value(&workspace.0).unwrap().to_string();
    assert!(
        !workspace.contains("u/a/wh/analytics/draft"),
        "an editor's buffer is not part of the workspace graph: {workspace}"
    );

    let deployed_run = asset_graph_for(
        &admin,
        WS,
        UserDB::new(db.clone()),
        db.clone(),
        query(),
        Some(PinnedRun { job_id: job, script_path: PATH.to_string(), script_hash: Some(HASH) }),
    )
    .await
    .unwrap();
    let deployed_run = serde_json::to_value(&deployed_run.0).unwrap().to_string();
    assert!(
        !deployed_run.contains("u/a/wh/analytics/draft"),
        "nor of a run of the deployed version: {deployed_run}"
    );
}
