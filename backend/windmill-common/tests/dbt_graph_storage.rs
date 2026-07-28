//! The dbt graph's storage invariants, against a real database.
//!
//! Every defect found in this area across review was DB-shaped — which row set
//! a read resolves to, which rows a clear takes, whether a snapshot exists at
//! all — and none of it is reachable from a unit test on a pure function. These
//! pin the answers that took several rounds to settle.

use sqlx::{Pool, Postgres};
use windmill_common::dbt_manifest::{
    clear_dbt_manifest, clear_dbt_manifest_version, prune_dbt_run_graphs, replace_dbt_manifest,
    IngestedManifest, IngestedNode, DEPLOYED_GRAPH, DEPLOYED_GRAPH_VERSIONS_KEPT,
};

const WS: &str = "test-workspace";
const PATH: &str = "f/test/proj";

async fn deploy_script(db: &Pool<Postgres>, hash: i64) {
    sqlx::query!(
        "INSERT INTO script (workspace_id, hash, path, summary, description, content, created_by,
                             language, lock)
         VALUES ($1, $2, $3, '', '', 'profile: {}', 'test-user', 'dbt', '')",
        WS,
        hash,
        PATH,
    )
    .execute(db)
    .await
    .unwrap();
}

fn manifest(names: &[&str]) -> IngestedManifest {
    IngestedManifest {
        nodes: names
            .iter()
            .map(|n| IngestedNode {
                unique_id: format!("model.p.{n}"),
                resource_type: "model".to_string(),
                name: n.to_string(),
                asset_path: Some(format!("u/a/wh/s/{n}")),
                raw_code: Some(format!("select 1 as {n}")),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

async fn nodes_for(db: &Pool<Postgres>, hash: i64, job: uuid::Uuid) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) FROM dbt_node WHERE workspace_id = $1 AND script_hash = $2 AND job_id = $3",
        WS,
        hash,
        job
    )
    .fetch_one(db)
    .await
    .unwrap()
    .unwrap_or(0)
}

async fn markers_for_path(db: &Pool<Postgres>) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) FROM dbt_graph_snapshot WHERE workspace_id = $1 AND script_path = $2",
        WS,
        PATH
    )
    .fetch_one(db)
    .await
    .unwrap()
    .unwrap_or(0)
}

async fn markers(db: &Pool<Postgres>, hash: i64) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) FROM dbt_graph_snapshot WHERE workspace_id = $1 AND script_hash = $2",
        WS,
        hash
    )
    .fetch_one(db)
    .await
    .unwrap()
    .unwrap_or(0)
}

/// A run whose graph matches the version's stores nothing. Marking a descriptor
/// dynamic is conservative — a `{{ }}` in `vars` says the arguments reach dbt,
/// not that they change which models exist — so the usual dynamic run resolves
/// to exactly the deployed graph, and storing a copy per run is what filled the
/// table with an unchanging picture.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn an_identical_run_stores_no_snapshot(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    let m = manifest(&["a", "b"]);
    let job = uuid::Uuid::from_u128(7);

    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &m, "root")
        .await
        .unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, Some(job), &m, "root")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(nodes_for(&db, 1, DEPLOYED_GRAPH).await, 2);
    assert_eq!(
        nodes_for(&db, 1, job).await,
        0,
        "identical run must not snapshot"
    );
    assert_eq!(markers(&db, 1).await, 1, "and leaves no marker of its own");
}

/// A run whose model set differs keeps its own, and the version's is untouched:
/// this is what lets an older run page render the project that run built.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_differing_run_keeps_its_own_snapshot(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    let job = uuid::Uuid::from_u128(7);

    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &manifest(&["a"]), "root")
        .await
        .unwrap();
    replace_dbt_manifest(
        &mut tx,
        WS,
        PATH,
        1,
        Some(job),
        &manifest(&["a", "b"]),
        "root",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(nodes_for(&db, 1, DEPLOYED_GRAPH).await, 1);
    assert_eq!(nodes_for(&db, 1, job).await, 2);
}

/// A dynamic run can disable every model. That empty graph is an answer, and it
/// has to be distinguishable from a run that stored nothing — otherwise the run
/// page falls back to the deployed models and shows what the run never built.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn an_empty_run_graph_is_still_a_snapshot(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    let job = uuid::Uuid::from_u128(7);

    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &manifest(&["a"]), "root")
        .await
        .unwrap();
    replace_dbt_manifest(
        &mut tx,
        WS,
        PATH,
        1,
        Some(job),
        &IngestedManifest::default(),
        "root",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(nodes_for(&db, 1, job).await, 0);
    let marker = sqlx::query_scalar!(
        "SELECT count(*) FROM dbt_graph_snapshot WHERE workspace_id = $1 AND job_id = $2",
        WS,
        job
    )
    .fetch_one(&db)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(marker, 1, "an empty graph still has a marker");
}

/// Archiving or deleting ONE version must leave every other version's graph
/// alone: the routes act on a single hash, and the other versions stay live and
/// keep needing their own models, SQL and lineage.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn clearing_one_version_leaves_the_others(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    deploy_script(&db, 2).await;
    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &manifest(&["a"]), "root")
        .await
        .unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 2, None, &manifest(&["a", "b"]), "root")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = db.begin().await.unwrap();
    clear_dbt_manifest_version(&mut tx, WS, PATH, 1)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(nodes_for(&db, 1, DEPLOYED_GRAPH).await, 0);
    assert_eq!(markers(&db, 1).await, 0, "the marker goes with the rows");
    assert_eq!(
        nodes_for(&db, 2, DEPLOYED_GRAPH).await,
        2,
        "the other version survives"
    );
    assert_eq!(markers(&db, 2).await, 1);
}

/// The path-wide clear is for the routes that retire the whole path, and it has
/// to take the markers too — a marker standing for rows that are gone is read
/// as a snapshot, and its digest still answers the suppression check.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn clearing_the_path_takes_markers_with_it(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    deploy_script(&db, 2).await;
    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &manifest(&["a"]), "root")
        .await
        .unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 2, None, &manifest(&["b"]), "root")
        .await
        .unwrap();
    clear_dbt_manifest(&mut tx, WS, PATH).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(markers(&db, 1).await, 0);
    assert_eq!(markers(&db, 2).await, 0);
    assert_eq!(nodes_for(&db, 1, DEPLOYED_GRAPH).await, 0);
}

/// The sweep ages out run snapshots and never a version's own graph, which is
/// what its runs render from for as long as the version exists.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn the_sweep_takes_old_snapshots_and_spares_the_version(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    let old = uuid::Uuid::from_u128(7);
    let recent = uuid::Uuid::from_u128(8);

    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &manifest(&["a"]), "root")
        .await
        .unwrap();
    replace_dbt_manifest(
        &mut tx,
        WS,
        PATH,
        1,
        Some(old),
        &manifest(&["a", "b"]),
        "root",
    )
    .await
    .unwrap();
    replace_dbt_manifest(
        &mut tx,
        WS,
        PATH,
        1,
        Some(recent),
        &manifest(&["a", "c"]),
        "root",
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Age one snapshot past the window, rows and marker together.
    for t in ["dbt_node", "dbt_edge", "dbt_graph_snapshot"] {
        sqlx::query(&format!(
            "UPDATE {t} SET ingested_at = now() - interval '400 days' WHERE job_id = $1"
        ))
        .bind(old)
        .execute(&db)
        .await
        .unwrap();
    }

    prune_dbt_run_graphs(&db).await.unwrap();

    assert_eq!(nodes_for(&db, 1, old).await, 0, "the aged snapshot goes");
    assert_eq!(nodes_for(&db, 1, recent).await, 2, "the recent one stays");
    assert_eq!(
        nodes_for(&db, 1, DEPLOYED_GRAPH).await,
        1,
        "and a version's own graph is never swept"
    );
}

/// A version's graph cannot expire on a clock — a run page is as old as its job
/// — so growth is bounded by deploy COUNT instead. The newest deploys keep
/// theirs; past that the oldest are reclaimed, which is what stops a CI that
/// deploys on every commit from adding a model set per commit forever.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn only_the_newest_deploys_keep_their_graph(db: Pool<Postgres>) {
    let over = DEPLOYED_GRAPH_VERSIONS_KEPT + 3;
    for h in 1..=over {
        deploy_script(&db, h).await;
        let mut tx = db.begin().await.unwrap();
        replace_dbt_manifest(&mut tx, WS, PATH, h, None, &manifest(&["a"]), "root")
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    assert_eq!(markers_for_path(&db).await, over, "every deploy stored one");

    prune_dbt_run_graphs(&db).await.unwrap();

    assert_eq!(
        markers_for_path(&db).await,
        DEPLOYED_GRAPH_VERSIONS_KEPT,
        "the bound holds"
    );
    // The newest is always among them: losing the live version's graph would
    // empty the page of every run of it.
    assert_eq!(nodes_for(&db, over, DEPLOYED_GRAPH).await, 1);
    assert_eq!(nodes_for(&db, 1, DEPLOYED_GRAPH).await, 0, "the oldest is reclaimed");
}
