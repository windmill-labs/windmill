//! The dbt graph's storage invariants, against a real database.
//!
//! Every defect found in this area across review was DB-shaped — which row set
//! a read resolves to, which rows a clear takes, whether a snapshot exists at
//! all — and none of it is reachable from a unit test on a pure function. These
//! pin the answers that took several rounds to settle.

use sqlx::{Pool, Postgres};
use windmill_common::dbt_manifest::{
    clear_dbt_editor_graphs, clear_dbt_manifest_version, prune_dbt_run_graphs,
    replace_dbt_editor_graph, replace_dbt_manifest, IngestedManifest, IngestedNode,
    DBT_EDITOR_GRAPHS_KEPT, DEPLOYED_GRAPH, DEPLOYED_GRAPH_VERSIONS_KEPT,
};

const WS: &str = "test-workspace";
const PATH: &str = "f/test/proj";
/// The principal an editor parse runs as. Retention is bounded per identity,
/// so the tests have to name one.
const ME: &str = "u/me";

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
        // Chained, so the edge insert actually runs: `ref()` lineage is in every
        // real project, and a fixture with no edges leaves that statement
        // unexecuted by every test in this file.
        edges: names
            .windows(2)
            .map(|w| (format!("model.p.{}", w[0]), format!("model.p.{}", w[1])))
            .collect(),
        ..Default::default()
    }
}

/// Edges for one version, so a test can assert the batched insert ran at all.
async fn edges_for(db: &Pool<Postgres>, hash: i64) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) FROM dbt_edge WHERE workspace_id = $1 AND script_hash = $2",
        WS,
        hash
    )
    .fetch_one(db)
    .await
    .unwrap()
    .unwrap_or(0)
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
    // The lineage is stored and cleared with its nodes. Asserted here because
    // this is where two versions coexist: it pins the batched edge insert
    // against a real database as well as the version scoping.
    assert_eq!(edges_for(&db, 1).await, 0, "the cleared version's edges go");
    assert_eq!(edges_for(&db, 2).await, 1, "the other version keeps its own");
}

/// The routes that hard-delete a path clear no graph rows: they delete the
/// `script` rows and let `ON DELETE CASCADE` take the sidecars, since taking
/// those first would lock them ahead of the script row and deadlock a concurrent
/// publication. So the cascade has to reach every sidecar, snapshots and markers
/// included.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn deleting_the_script_cascades_to_every_sidecar(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    deploy_script(&db, 2).await;
    let job = uuid::Uuid::from_u128(7);
    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &manifest(&["a", "b"]), "root")
        .await
        .unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, Some(job), &manifest(&["a"]), "root")
        .await
        .unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 2, None, &manifest(&["b", "c"]), "root")
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(markers_for_path(&db).await, 3, "two versions and one run");

    sqlx::query!(
        "DELETE FROM script WHERE workspace_id = $1 AND path = $2",
        WS,
        PATH
    )
    .execute(&db)
    .await
    .unwrap();

    assert_eq!(nodes_for(&db, 1, DEPLOYED_GRAPH).await, 0);
    assert_eq!(nodes_for(&db, 1, job).await, 0, "a run's snapshot goes too");
    assert_eq!(nodes_for(&db, 2, DEPLOYED_GRAPH).await, 0);
    assert_eq!(edges_for(&db, 1).await, 0);
    assert_eq!(edges_for(&db, 2).await, 0);
    assert_eq!(markers_for_path(&db).await, 0);
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

    prune_dbt_run_graphs(&db, PATH, WS).await.unwrap();

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

    prune_dbt_run_graphs(&db, PATH, WS).await.unwrap();

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

/// The third provenance: a `parse` of the EDITOR's buffer, which names no
/// deployed version. It cannot borrow the deployed hash — the buffer differs
/// from it, and a project being written may have no deployed version at all — so
/// it is keyed to the parse job alone and leaves the version's graph untouched.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn an_editor_parse_is_keyed_to_its_job_and_no_version(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    let job = uuid::Uuid::from_u128(7);

    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &manifest(&["a"]), "root")
        .await
        .unwrap();
    replace_dbt_editor_graph(&mut tx, WS, PATH, job, ME, &manifest(&["a", "b"]), "root")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(
        editor_nodes(&db, job).await,
        2,
        "the buffer's own models are stored"
    );
    assert_eq!(
        nodes_for(&db, 1, DEPLOYED_GRAPH).await,
        1,
        "and the deployed version's graph is untouched"
    );
    assert_eq!(
        nodes_for(&db, 1, job).await,
        0,
        "nothing is attributed to the version"
    );
}

/// A buffer identical to the deploy still stores a graph, unlike a run's
/// snapshot: the editor pins to its own job, so suppressing the write would
/// leave it nothing to pin to and its provenance label would claim a parse that
/// is not on screen.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn an_editor_parse_matching_the_deploy_still_stores(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    let job = uuid::Uuid::from_u128(7);
    let m = manifest(&["a"]);

    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &m, "root")
        .await
        .unwrap();
    replace_dbt_editor_graph(&mut tx, WS, PATH, job, ME, &m, "root")
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(editor_nodes(&db, job).await, 1);
}

/// Bounded per (path, principal) as each refresh lands: the ones before it are
/// dead the moment a newer parse arrives, and a click that stored a full model
/// set forever would be the editor's own leak.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn only_the_newest_editor_parses_are_kept(db: Pool<Postgres>) {
    let over = DBT_EDITOR_GRAPHS_KEPT + 2;
    for i in 1..=over {
        let mut tx = db.begin().await.unwrap();
        replace_dbt_editor_graph(
            &mut tx,
            WS,
            PATH,
            uuid::Uuid::from_u128(i as u128),
            ME,
            &manifest(&["a"]),
            "root",
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }
    assert_eq!(
        editor_markers(&db).await,
        DBT_EDITOR_GRAPHS_KEPT,
        "the bound holds"
    );
    assert_eq!(
        editor_nodes(&db, uuid::Uuid::from_u128(over as u128)).await,
        1,
        "the newest is always among them"
    );
    assert_eq!(
        editor_nodes(&db, uuid::Uuid::from_u128(1)).await,
        0,
        "and its rows go with its marker"
    );
}

/// Archiving or deleting ONE version must not take an editor's graph with it —
/// that graph describes a buffer, not a version.
///
/// Retiring the whole PATH must, and cannot rely on the cascade the versioned
/// rows ride: a version-less row leaves `script_hash` NULL, which satisfies the
/// composite foreign key without referencing anything, so deleting the script
/// takes the version's graph and leaves the editor's behind.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn a_version_clear_spares_editor_graphs_and_a_path_clear_does_not(db: Pool<Postgres>) {
    deploy_script(&db, 1).await;
    let job = uuid::Uuid::from_u128(7);

    let mut tx = db.begin().await.unwrap();
    replace_dbt_manifest(&mut tx, WS, PATH, 1, None, &manifest(&["a"]), "root")
        .await
        .unwrap();
    replace_dbt_editor_graph(&mut tx, WS, PATH, job, ME, &manifest(&["a"]), "root")
        .await
        .unwrap();
    clear_dbt_manifest_version(&mut tx, WS, PATH, 1).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(nodes_for(&db, 1, DEPLOYED_GRAPH).await, 0);
    assert_eq!(editor_nodes(&db, job).await, 1, "the buffer's graph survives");

    let mut tx = db.begin().await.unwrap();
    clear_dbt_editor_graphs(&mut tx, WS, PATH).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(editor_nodes(&db, job).await, 0, "retiring the path takes it");
}

/// A preview names its own PATH and needs only `jobs:run`, so a bound over the
/// path alone is a way to retire the graphs of whoever is actually editing that
/// script. Each identity reclaims only its own.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn one_principal_cannot_evict_another_s_editor_graphs(db: Pool<Postgres>) {
    let mine = uuid::Uuid::from_u128(1);
    let mut tx = db.begin().await.unwrap();
    replace_dbt_editor_graph(&mut tx, WS, PATH, mine, ME, &manifest(&["a"]), "root")
        .await
        .unwrap();
    // Someone else, parsing the same path far more often than the bound allows.
    for i in 0..(DBT_EDITOR_GRAPHS_KEPT + 3) {
        replace_dbt_editor_graph(
            &mut tx,
            WS,
            PATH,
            uuid::Uuid::from_u128(100 + i as u128),
            "u/squatter",
            &manifest(&["a"]),
            "root",
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();

    assert_eq!(
        editor_nodes(&db, mine).await,
        1,
        "another principal's parses must not retire this one's graph"
    );
}

/// Nodes of one editor parse.
async fn editor_nodes(db: &Pool<Postgres>, job: uuid::Uuid) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) FROM dbt_node
          WHERE workspace_id = $1 AND job_id = $2 AND script_hash IS NULL",
        WS,
        job
    )
    .fetch_one(db)
    .await
    .unwrap()
    .unwrap_or(0)
}

async fn editor_markers(db: &Pool<Postgres>) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) FROM dbt_graph_snapshot
          WHERE workspace_id = $1 AND script_path = $2 AND script_hash IS NULL",
        WS,
        PATH
    )
    .fetch_one(db)
    .await
    .unwrap()
    .unwrap_or(0)
}
