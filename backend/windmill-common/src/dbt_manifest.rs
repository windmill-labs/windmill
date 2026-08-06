//! Reading dbt's `target/manifest.json` into the asset graph.
//!
//! The manifest is dbt's own parsed project: every model, snapshot, seed,
//! source and test, with the physical relation each resolves to and the
//! `ref()`/`source()` edges between them. Ingesting it is what makes dbt models
//! first-class assets rather than an opaque job (docs/dbt-runtime.md).
//!
//! Two things this module is deliberate about:
//!
//! * **Asset identity is the physical relation.** A model becomes
//!   `dbt://<warehouse>/<schema>/<name>`: the scheme names dbt, which is the
//!   only thing that creates one, but the PATH is the relation and never dbt's
//!   own `unique_id`. Two projects meet at a handoff — one materializes a mart,
//!   the next declares it a `source` — where `model.a.orders` and
//!   `source.b.analytics.orders` differ but the relation does not, so keying on
//!   the node id would leave each project an island; a native script reading the
//!   same table would form no edge either. A dbt run does not trigger those
//!   readers (decision 11, "No cascade from dbt"); sharing the node is what makes
//!   the lineage one graph.
//! * **The warehouse is identified by its workspace NAME**, matching the way
//!   `ducklake://main.orders` names a lake. Connection details (host, account)
//!   are never part of the key: the same warehouse is reachable under several
//!   hostnames, and credential material has no business in an asset key. The
//!   warehouse names the default database too, so relations in it need no
//!   qualification; one that OVERRODE its database qualifies its schema segment
//!   (`table_asset_path`), since two same-named relations in different databases
//!   are not the same table. The accepted consequence is documented in
//!   docs/dbt-runtime.md — two resources pointing at one physical warehouse do
//!   not unify.
//!
//! # Mutator contract
//!
//! Every `pub` mutator in this module — the manifest ones
//! (`replace_dbt_manifest`, `clear_dbt_manifest_version`,
//! `clear_dbt_editor_graphs`),
//! the snapshot sweep, and the retry-state ones (`move_dbt_run_state`,
//! `clear_dbt_run_state`, `clear_dbt_run_state_if_path_retired`) — takes the
//! workspace and the script to act on as plain arguments and enforces nothing:
//! **the caller must already have verified write access to that script**,
//! exactly like the sibling `assets::replace_static_asset_usage` each is called
//! next to. The retry-state ones destroy a resumable failure and move saved
//! invocation arguments between paths, so an unauthorized caller there hands one
//! principal's arguments to another. A user-scoped
//! transaction is not enforcement here — `dbt_node` and `dbt_edge` carry no RLS
//! policy and grant `windmill_user` full access, deliberately, because the
//! writer is the dependency job rather than a request.
//!
//! Every caller today satisfies it through the route that owns the script:
//! `windmill-api-scripts::scripts` clears and moves from the create, archive,
//! rename and delete handlers, each behind a `scripts:write:<path>` scope check
//! plus that route's own owner or admin requirement; the worker replaces from
//! the dependency job of the very script being deployed, and saves retry state
//! for the script it just ran. A new call site that cannot name where it
//! authorized is a bug.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use windmill_parser::asset_parser::canonicalize_table_asset_path;

use crate::assets::{AssetKind, AssetUsageAccessType, AssetWithAltAccessType};
use crate::error::Result;

#[derive(Deserialize, Debug, Default)]
pub struct Manifest {
    #[serde(default)]
    pub metadata: ManifestMetadata,
    #[serde(default)]
    pub nodes: BTreeMap<String, ManifestNode>,
    #[serde(default)]
    pub sources: BTreeMap<String, ManifestNode>,
    #[serde(default)]
    pub parent_map: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize, Debug, Default)]
pub struct ManifestMetadata {
    #[serde(default)]
    pub dbt_version: String,
    #[serde(default)]
    pub adapter_type: String,
    #[serde(default)]
    pub project_name: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct ManifestNode {
    #[serde(default)]
    pub resource_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub alias: Option<String>,
    /// Sources only: the PHYSICAL table name. `name` is the logical one
    /// `source()` refers to, and the two differ whenever a source is declared
    /// with an `identifier`. Sources have no `alias`.
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    /// dbt's resolved database (BigQuery's project, Snowflake's database). A
    /// model can override it per node, so it is part of identity.
    #[serde(default)]
    pub database: Option<String>,
    /// dbt leaves this null exactly for nodes with no physical relation:
    /// ephemeral models (inlined as CTEs) and tests.
    #[serde(default)]
    pub relation_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub config: NodeConfig,
    #[serde(default)]
    pub test_metadata: Option<TestMetadata>,
    #[serde(default)]
    pub column_name: Option<String>,
    #[serde(default)]
    pub attached_node: Option<String>,
    #[serde(default)]
    pub columns: BTreeMap<String, ManifestColumn>,
    #[serde(default)]
    pub freshness: Option<serde_json::Value>,
    /// The model's SQL as written, `{{ ref() }}` and all. `dbt parse` fills
    /// this; `compiled_code` needs a `dbt compile`, which no phase here runs.
    #[serde(default)]
    pub raw_code: Option<String>,
    /// Its path inside the dbt project, e.g. `models/staging/stg_orders.sql`.
    #[serde(default)]
    pub original_file_path: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
pub struct NodeConfig {
    #[serde(default)]
    pub materialized: Option<String>,
    /// dbt allows a list for composite keys; only a single-column key maps onto
    /// a Windmill `merge`/`scd2` strategy, so a composite one is left unmapped.
    #[serde(default)]
    pub unique_key: Option<serde_json::Value>,
    #[serde(default)]
    pub severity: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
pub struct ManifestColumn {
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
pub struct TestMetadata {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub kwargs: serde_json::Value,
    #[serde(default)]
    pub namespace: Option<String>,
}

/// A content digest of the graph a row set describes.
///
/// Covers every stored field of the nodes, their `ref()` edges and the root the
/// relations resolved under — more than the graph draws, which errs toward
/// storing a snapshot rather than suppressing one that differs.
fn graph_digest(ingested: &IngestedManifest, relation_root: &str) -> String {
    use sha2::{Digest, Sha256};
    // SHA-256, not `DefaultHasher`: this is written to the database and compared on
    // later runs, possibly by a worker on another toolchain, and `DefaultHasher` is
    // explicitly not stable across Rust releases. A drifting hash would store a full
    // snapshot every run, looking exactly like the suppression never working.
    let mut h = Sha256::new();
    // Delimited and serialized, both parts the same way: `{:?}` renames a field
    // with the struct, changing every digest once for nothing, and unseparated
    // parts let one part's tail read as the next part's head.
    h.update(relation_root.as_bytes());
    h.update(b"\0");
    h.update(
        serde_json::to_string(&ingested.nodes)
            .unwrap_or_default()
            .as_bytes(),
    );
    h.update(b"\0");
    h.update(
        serde_json::to_string(&ingested.edges)
            .unwrap_or_default()
            .as_bytes(),
    );
    format!("{:x}", h.finalize())
}

/// The `job_id` of a graph that belongs to a deployed VERSION rather than to one
/// run: what a static descriptor writes at deploy and all of its runs read.
///
/// A sentinel rather than NULL because `job_id` is part of the primary key, and
/// Postgres does not treat two NULLs as the same key — every re-ingest would
/// insert a new row set instead of replacing one.
pub const DEPLOYED_GRAPH: uuid::Uuid = uuid::Uuid::nil();

/// How many deploys of one script keep their graph.
///
/// A version's graph is what its own finished runs render from, so it cannot
/// expire on a clock — a run page is as old as its job. It is bounded by COUNT
/// instead: the newest deploys keep theirs and older ones are dropped, which
/// makes growth `versions x models` per path rather than unbounded in time. A CI
/// deploying on every commit would otherwise add a full model set per commit and
/// nothing would ever reclaim it.
///
/// Generous on purpose. Losing a graph empties that version's run pages, so the
/// bound exists to stop unbounded growth, not to be reached in normal use.
pub const DEPLOYED_GRAPH_VERSIONS_KEPT: i64 = 50;

/// How long a run's graph snapshot outlives it. Only dynamic descriptors write
/// one, and the run page is the only reader.
pub const RUN_GRAPH_RETENTION_DAYS: i32 = 30;

/// Drop the run snapshots older than the retention window, across the instance.
///
/// How long a run's progress rows outlive it.
///
/// They exist for the run page, which reads them live and — for a run that left
/// no `run_results.json`, cancelled or killed — afterwards. Bounded by age and
/// swept by the writes themselves, so no background job has to know this table.
const RUN_PROGRESS_RETENTION_DAYS: i32 = 30;

/// Drop progress rows older than the retention above.
///
/// Called from EVERY writer, like `prune_dbt_run_graphs`: these rows carry no
/// job foreign key on purpose, so nothing else reclaims them, and a writer that
/// skips this is a configuration with no sweep at all — an agent-only workspace
/// writes them exclusively through the API.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn prune_run_progress(db: &crate::DB, w_id: &str) {
    let res = sqlx::query!(
        "DELETE FROM dbt_run_progress
          WHERE workspace_id = $1 AND updated_at < now() - make_interval(days => $2)",
        w_id,
        RUN_PROGRESS_RETENTION_DAYS,
    )
    .execute(db)
    .await;
    if let Err(e) = res {
        tracing::warn!("pruning dbt run progress: {e:#}");
    }
}

/// Called by everything that writes them — a run, a deploy, and the endpoint an
/// agent worker publishes through — so this table needs no background sweep, the
/// same shape `dbt_run_progress` uses. A writer that does not call it is a
/// configuration with no sweep at all: an agent-only project is deployed once and
/// never runs where the pool is reachable.
///
/// Also drops the deploy graphs of the running script beyond
/// `DEPLOYED_GRAPH_VERSIONS_KEPT`, which is the only reclamation those have.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn prune_dbt_run_graphs(
    db: &sqlx::Pool<Postgres>,
    // The script this run belongs to. The version sweep is scoped to it rather
    // than run instance-wide: unscoped it seq-scans every version of every
    // script on the instance, on every dbt run, and past the keep-count is a
    // rare state — so nearly every execution paid for it to reclaim nothing.
    script_path: &str,
    workspace_id: &str,
) -> Result<()> {
    // Markers first, then the rows they stand for: a marker outliving its nodes
    // would report a snapshot the reader then finds empty, which is the very
    // confusion the marker exists to remove. All three share one age predicate,
    // so a row and its marker expire together.
    sqlx::query!(
        "DELETE FROM dbt_graph_snapshot
          WHERE job_id <> '00000000-0000-0000-0000-000000000000'
            AND ingested_at < now() - make_interval(days => $1)",
        RUN_GRAPH_RETENTION_DAYS,
    )
    .execute(db)
    .await?;
    // Bounded by age on BOTH sides, and the sentinel spelled as a literal so the
    // partial indexes apply: a bound parameter cannot be proven to match the
    // index predicate, which turned this into a whole-table scan on every run.
    sqlx::query!(
        "DELETE FROM dbt_node
          WHERE job_id <> '00000000-0000-0000-0000-000000000000'
            AND ingested_at < now() - make_interval(days => $1)",
        RUN_GRAPH_RETENTION_DAYS,
    )
    .execute(db)
    .await?;
    sqlx::query!(
        "DELETE FROM dbt_edge
          WHERE job_id <> '00000000-0000-0000-0000-000000000000'
            AND ingested_at < now() - make_interval(days => $1)",
        RUN_GRAPH_RETENTION_DAYS,
    )
    .execute(db)
    .await?;
    // In ONE transaction with the orphan sweep: a restart in the gap leaves graph
    // rows whose marker is gone, and since the sweep runs only when a marker went,
    // every later call computes `retired == 0` and skips them for good.
    let mut tx = db.begin().await?;
    // Ordered by the script's own `created_at`, so "newest" is the newest deploy
    // and not the newest ingest: a late-finishing job must not promote an old
    // version. Scoped to one (workspace, path), which the path index serves.
    let retired = sqlx::query!(
        "WITH keep AS (
           SELECT hash FROM script
            WHERE workspace_id = $2 AND path = $3 AND language = 'dbt'
            ORDER BY created_at DESC LIMIT $1
         )
         DELETE FROM dbt_graph_snapshot g
          WHERE g.workspace_id = $2 AND g.script_path = $3
            AND g.job_id = '00000000-0000-0000-0000-000000000000'
            AND NOT EXISTS (SELECT 1 FROM keep k WHERE k.hash = g.script_hash)",
        DEPLOYED_GRAPH_VERSIONS_KEPT,
        workspace_id,
        script_path,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    // Only when a marker actually went: these two are the complement of every
    // partial index here — all of them `WHERE job_id <> DEPLOYED` — and past the
    // keep-count is rare, so the ordinary run should pay for neither.
    if retired > 0 {
        for table in ["dbt_node", "dbt_edge"] {
            sqlx::query(&format!(
                "DELETE FROM {table} t
                  WHERE t.workspace_id = $1 AND t.script_path = $2
                    AND t.job_id = '00000000-0000-0000-0000-000000000000'
                    AND NOT EXISTS (SELECT 1 FROM dbt_graph_snapshot g
                                     WHERE g.workspace_id = t.workspace_id
                                       AND g.script_path = t.script_path
                                       AND g.script_hash = t.script_hash
                                       AND g.job_id = t.job_id)"
            ))
            .bind(workspace_id)
            .bind(script_path)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

/// Longest model body kept for display. Generous for a hand-written model and
/// small enough that a generated one cannot bloat the sidecar.
const MAX_SQL_BYTES: usize = 32 * 1024;

fn truncate_sql(code: &str) -> String {
    if code.len() <= MAX_SQL_BYTES {
        return code.to_string();
    }
    // On a char boundary, so the result stays valid UTF-8.
    let mut end = MAX_SQL_BYTES;
    while end > 0 && !code.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n-- … truncated by Windmill …", &code[..end])
}

/// One ingested dbt node, ready to be written to `dbt_node`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(default)]
pub struct IngestedNode {
    pub unique_id: String,
    pub resource_type: String,
    pub name: String,
    pub asset_path: Option<String>,
    pub materialized: Option<String>,
    pub materialize_strategy: Option<String>,
    pub unique_key: Option<String>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub test_kind: Option<String>,
    pub test_column: Option<String>,
    pub test_args: Option<serde_json::Value>,
    pub severity: Option<String>,
    pub attached_node: Option<String>,
    pub columns: Option<serde_json::Value>,
    pub freshness: Option<serde_json::Value>,
    /// The transform itself, for the graph to render. The copy taken at
    /// deploy: the file itself is in the script's module bundle.
    pub raw_code: Option<String>,
    pub original_file_path: Option<String>,
}

// Serde: an agent worker cannot write these tables directly, so it posts the
// whole manifest to the server, which stores it with the same function the SQL
// path uses.
#[derive(Debug, Default, Serialize, Deserialize)]
// `default`: these cross the wire for an agent worker, and a field the serializer
// skips must not make the whole manifest unparseable on the other side.
#[serde(default)]
pub struct IngestedManifest {
    pub nodes: Vec<IngestedNode>,
    pub edges: Vec<(String, String)>,
    /// The `asset` rows the owning script produces (models) and consumes
    /// (sources) — what the lineage graph is drawn from.
    pub assets: Vec<AssetWithAltAccessType>,
    pub dbt_version: String,
    pub adapter_type: String,
}

/// dbt's `materialized` mapped onto Windmill's write strategy.
///
/// The mapping is exact for the four strategies Windmill has, and deliberately
/// `None` where dbt has no analogue: `view` produces a relation but performs no
/// write, and `ephemeral` produces no relation at all.
pub fn materialize_strategy_for(materialized: &str, has_unique_key: bool) -> Option<String> {
    match materialized {
        "table" | "seed" => Some("replace".to_string()),
        // Any `unique_key` makes it a merge — including a composite one, whose
        // columns `single_unique_key` cannot name. Reporting `append` for those
        // would describe an upsert as an insert-only write.
        "incremental" | "microbatch" => Some(match has_unique_key {
            true => "merge".to_string(),
            false => "append".to_string(),
        }),
        "snapshot" => Some("scd2".to_string()),
        _ => None,
    }
}

/// Whether dbt declared any `unique_key`, composite or not.
fn has_unique_key(v: Option<&serde_json::Value>) -> bool {
    match v {
        Some(serde_json::Value::String(s)) => !s.is_empty(),
        Some(serde_json::Value::Array(a)) => !a.is_empty(),
        _ => false,
    }
}

fn single_unique_key(v: Option<&serde_json::Value>) -> Option<String> {
    match v {
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        // A composite key has no single-column analogue; leave it unmapped
        // rather than silently picking the first column.
        _ => None,
    }
}

/// `dbt://` path of the relation a node resolves to, or `None` when it has
/// none. Assembles the parts; `table_asset_path` owns the spelling.
fn asset_path_for(
    node: &ManifestNode,
    warehouse: &str,
    default_database: Option<&str>,
) -> Option<String> {
    node.relation_name.as_ref()?;
    // `schema` is dbt's RESOLVED schema; `config.schema` is only the suffix
    // `generate_schema_name` combines with the target's (`schema: snapshots` under
    // `analytics` lands in `analytics_snapshots`). Keying on the config value names
    // a relation that does not exist, invisibly, until nothing links.
    let schema = node.schema.as_deref()?;
    // `identifier` for a source, `alias` for a model; `name` only when neither
    // is set. Using `name` for a source declared `name: orders, identifier:
    // raw_orders` would key the asset on a relation that does not exist, so no
    // native script reading `raw_orders` could ever join it.
    let name = node
        .identifier
        .as_deref()
        .or(node.alias.as_deref())
        .unwrap_or(&node.name);
    table_asset_path(
        warehouse,
        node.database.as_deref(),
        schema,
        name,
        default_database,
    )
}

/// `asset.path`'s column width. A `dbt://` key is the only asset path assembled
/// from warehouse identifiers rather than from a Windmill path, so it is the
/// only one that can outgrow the column.
pub const MAX_ASSET_PATH_LEN: usize = 255;

/// The one derivation of a `dbt://` path from a relation's parts.
///
/// Three call sites need it and they MUST agree: the manifest ingest (which
/// creates the graph node), the live progress events, and the end-of-run
/// settlement (which both record status against it). A site that derives it
/// differently records progress against a path no node has, and nothing looks
/// broken — the run succeeds, the graph just never moves.
///
/// The resource names the warehouse and its default database, so a relation in
/// that database uses the plain three-segment spelling. One that overrode its
/// database is genuinely elsewhere and qualifies its schema segment as
/// `<database>.<schema>`, so two same-named relations cannot collapse. A project
/// that owns its `profiles.yml` reports its target's database the same way, so
/// both kinds of project spell one relation identically; only where a target
/// leaves its database implicit does every relation qualify, because assuming
/// they all share a database is what would collapse them.
pub fn table_asset_path(
    warehouse: &str,
    database: Option<&str>,
    schema: &str,
    name: &str,
    default_database: Option<&str>,
) -> Option<String> {
    let qualified = match database.map(str::trim).filter(|d| !d.is_empty()) {
        Some(db) if !default_database.is_some_and(|d| d.eq_ignore_ascii_case(db)) => {
            format!("{db}.{schema}")
        }
        _ => schema.to_string(),
    };
    // `/` is the segment boundary, and `canonicalize_table_asset_path` recovers
    // the components from the LAST two — so an identifier carrying one (legal
    // when quoted) moves that boundary: `("a/b", "c")` and `("a", "b/c")` both
    // spell `<warehouse>/a/b/c` and two relations share one node, one lineage
    // and one progress row. Refused rather than encoded: the `dbt://` spelling
    // is a contract (docs/dbt-runtime.md, decision 11) that annotations and the
    // frontend parse too. The relation keeps its manifest row and gets no asset,
    // exactly as an over-long one does.
    if qualified.contains('/') || name.contains('/') {
        return None;
    }
    let path = canonicalize_table_asset_path(&format!("{warehouse}/{qualified}/{name}"));
    // The bound lives HERE because all three derivations must agree, and it is
    // the column's: `asset.path` is VARCHAR(255), counted in CHARACTERS, and a
    // `dbt://` key is the only asset path assembled from warehouse identifiers
    // rather than from a Windmill path. Over it, the relation keeps its manifest
    // row and gets no asset — the alternative is Postgres rejecting the insert
    // and taking the graph down, or one swallowed `value too long` per node.
    (path.chars().count() <= MAX_ASSET_PATH_LEN).then_some(path)
}

/// Parse a `manifest.json` into rows, edges and asset usages.
///
/// `warehouse` is the workspace warehouse's NAME, which the profile
/// target points at, e.g. `main`.
/// `selected` is the node set the descriptor's `select`/`exclude` resolves to,
/// as reported by dbt itself (`dbt ls`). `None` means the whole project. It
/// scopes what this script is recorded as owning: a script that builds only
/// `tag:nightly` must not register as the producer of every other model, or the
/// graph shows it owning models it never touches. Running several scripts
/// with different selections is the intended shape (docs/dbt-runtime.md,
/// decision 6), and this is what makes them compose.
pub fn ingest_manifest(
    manifest: &Manifest,
    warehouse: &str,
    // The profile target's database. Nodes in it use the plain three-segment
    // spelling; a node that overrode it qualifies its schema segment.
    default_database: Option<&str>,
    selected: Option<&std::collections::HashSet<String>>,
) -> IngestedManifest {
    let mut out = IngestedManifest {
        dbt_version: manifest.metadata.dbt_version.clone(),
        adapter_type: manifest.metadata.adapter_type.clone(),
        ..Default::default()
    };
    let mut assets: HashMap<(AssetKind, String), AssetUsageAccessType> = HashMap::new();

    // A source is this script's input only if something it builds reads it, or a
    // narrowly-selected script claims reads on tables it never touches. Without a
    // selection it builds everything, and a source nothing reads is still not an
    // input. The same set answers the cross-config question below.
    let direct_parents: std::collections::HashSet<&str> = {
        let mut out = std::collections::HashSet::new();
        let mut queue: Vec<&str> = manifest
            .parent_map
            .iter()
            .filter(|(child, _)| selected.is_none_or(|sel| sel.contains(child.as_str())))
            // Only what the script BUILDS establishes a dependency. Selecting a
            // model also selects its tests, and a `relationships` test depends
            // on the model it points at — so counting test parents would make a
            // staging-only script subscribe to a mart it merely asserts against.
            .filter(|(child, _)| {
                manifest
                    .nodes
                    .get(*child)
                    .is_some_and(|n| n.resource_type != "test")
            })
            .flat_map(|(_, parents)| parents.iter().map(|p| p.as_str()))
            .collect();
        let mut seen: std::collections::HashSet<&str> = queue.iter().copied().collect();
        while let Some(parent) = queue.pop() {
            // An ephemeral model is inlined as a CTE, so it produces nothing to
            // depend on — but whatever IT reads is still this script's real
            // input. Stopping at it loses the subscription entirely, and the
            // model then shows no upstream edge to its actual source.
            let is_ephemeral = manifest
                .nodes
                .get(parent)
                .is_some_and(|n| n.relation_name.is_none());
            if !is_ephemeral {
                out.insert(parent);
                continue;
            }
            for grandparent in manifest.parent_map.get(parent).into_iter().flatten() {
                if seen.insert(grandparent.as_str()) {
                    queue.push(grandparent.as_str());
                }
            }
        }
        out
    };
    for (unique_id, node) in manifest.nodes.iter().chain(manifest.sources.iter()) {
        // Whether this script BUILDS the node, as opposed to depending on it. Direct
        // parents are kept either way, because an edge needs both endpoints: drop
        // one and the `ref()` reaching it has nothing to point at, leaving two
        // relations with no line between them.
        let is_selected = selected.is_none_or(|sel| sel.contains(unique_id.as_str()));
        let keep = match node.resource_type.as_str() {
            "source" => direct_parents.contains(unique_id.as_str()),
            _ => is_selected || direct_parents.contains(unique_id.as_str()),
        };
        if !keep {
            continue;
        }
        let asset_path = asset_path_for(node, warehouse, default_database);
        let materialized = node
            .config
            .materialized
            .clone()
            .or(match node.resource_type.as_str() {
                // Sources and seeds carry no `materialized` of their own but their
                // physical nature is fixed.
                "source" => Some("source".to_string()),
                _ => None,
            });
        let unique_key = single_unique_key(node.config.unique_key.as_ref());
        let test = node.test_metadata.as_ref();
        out.nodes.push(IngestedNode {
            unique_id: unique_id.clone(),
            resource_type: node.resource_type.clone(),
            name: node.name.clone(),
            asset_path: asset_path.clone(),
            materialize_strategy: materialized.as_deref().and_then(|m| {
                materialize_strategy_for(m, has_unique_key(node.config.unique_key.as_ref()))
            }),
            materialized,
            unique_key,
            tags: node.tags.clone(),
            description: node.description.clone().filter(|d| !d.is_empty()),
            // A package's test keeps its own name (`dbt_utils.accepted_range`)
            // so it is legible rather than silently dropped for not being one
            // of the four generic kinds.
            test_kind: test.map(|t| match t.namespace.as_deref() {
                Some(ns) if !ns.is_empty() => format!("{ns}.{}", t.name),
                _ => t.name.clone(),
            }),
            test_column: node.column_name.clone(),
            test_args: test.map(|t| t.kwargs.clone()),
            severity: node.config.severity.clone(),
            attached_node: node.attached_node.clone(),
            columns: (!node.columns.is_empty()).then(|| {
                serde_json::json!(node
                    .columns
                    .iter()
                    .map(|(k, v)| (k.clone(), v.description.clone().unwrap_or_default()))
                    .collect::<BTreeMap<_, _>>())
            }),
            freshness: node.freshness.clone(),
            // The transform the graph renders. Capped: a project can hold
            // thousands of models and this is duplicated per deploy, so a
            // pathological file is truncated rather than allowed to bloat the
            // sidecar. Tests carry generated SQL nobody reads — skipped.
            raw_code: (node.resource_type != "test")
                .then(|| node.raw_code.clone())
                .flatten()
                .filter(|c| !c.trim().is_empty())
                .map(|c| truncate_sql(&c)),
            original_file_path: node.original_file_path.clone().filter(|p| !p.is_empty()),
        });

        // The dbt script writes what it materializes and reads its sources.
        // Internal `ref()`s between its own models are lineage inside the
        // project, not usages of the script — modelling them as reads would
        // make the script depend on its own output.
        let Some(path) = asset_path else { continue };
        let access = match node.resource_type.as_str() {
            // A parent kept only to anchor an edge is read, not written: this
            // script's selection does not build it, and claiming the write would
            // make two scripts splitting one project both own the same relation.
            "model" | "snapshot" | "seed" if is_selected => AssetUsageAccessType::W,
            "model" | "snapshot" | "seed" => AssetUsageAccessType::R,
            "source" => AssetUsageAccessType::R,
            _ => continue,
        };
        let key = (AssetKind::Dbt, path);
        let merged = match (assets.get(&key), access) {
            (Some(AssetUsageAccessType::R), AssetUsageAccessType::W)
            | (Some(AssetUsageAccessType::W), AssetUsageAccessType::R) => AssetUsageAccessType::RW,
            (Some(existing), _) => *existing,
            (None, a) => a,
        };
        assets.insert(key, merged);
    }

    out.nodes.sort_by(|a, b| a.unique_id.cmp(&b.unique_id));

    let kept: std::collections::HashSet<&str> =
        out.nodes.iter().map(|n| n.unique_id.as_str()).collect();
    // An ephemeral model is inlined as a CTE, so it is a node with no relation
    // and nothing to draw. Its edges are still real lineage though: dropping
    // `A -> E` and `E -> B` loses the `A -> B` the reader needs. Walk through
    // ephemeral parents to the nearest ones that do have a relation.
    let is_ephemeral = |id: &str| {
        manifest
            .nodes
            .get(id)
            .is_some_and(|n| n.relation_name.is_none() && n.resource_type != "test")
    };
    let physical_parents = |child: &str| -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut queue: Vec<&str> = manifest
            .parent_map
            .get(child)
            .into_iter()
            .flatten()
            .map(|p| p.as_str())
            .collect();
        while let Some(p) = queue.pop() {
            if !seen.insert(p.to_string()) {
                continue;
            }
            if is_ephemeral(p) {
                queue.extend(
                    manifest
                        .parent_map
                        .get(p)
                        .into_iter()
                        .flatten()
                        .map(|g| g.as_str()),
                );
            } else if kept.contains(p) {
                out.push(p.to_string());
            }
        }
        out
    };
    for child in manifest.parent_map.keys() {
        if !kept.contains(child.as_str()) || is_ephemeral(child) {
            continue;
        }
        for parent in physical_parents(child) {
            out.edges.push((parent, child.clone()));
        }
    }
    out.edges.sort();

    let mut assets: Vec<_> = assets.into_iter().collect();
    assets.sort_by(|(a, _), (b, _)| a.cmp(b));
    out.assets = assets
        .into_iter()
        .map(|((kind, path), access)| AssetWithAltAccessType {
            path,
            kind,
            access_type: Some(access),
            alt_access_type: None,
            columns: None,
        })
        .collect();
    out
}

/// Replace the stored manifest of one dbt script. Wipe-and-reinsert per ingest,
/// so a model deleted upstream disappears from the graph on the next deploy.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn replace_dbt_manifest(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
    // The deployed version this graph describes. Rows are keyed by it, so two
    // versions of one path coexist and an old run can still be shown the project
    // as it was when it ran.
    script_hash: i64,
    // The run this graph is a snapshot of, for a dynamic descriptor whose model
    // set depends on its arguments. `None` is the version's own graph as
    // deployed — what a static descriptor writes once and all of its runs read.
    job_id: Option<uuid::Uuid>,
    ingested: &IngestedManifest,
    // Where the profile put these relations, so a later run can tell whether the
    // resource has moved since — see the migration.
    relation_root: &str,
) -> Result<()> {
    let job_id = job_id.unwrap_or(DEPLOYED_GRAPH);
    let digest = graph_digest(ingested, relation_root);
    if job_id != DEPLOYED_GRAPH {
        // A snapshot earns its storage only by DIFFERING from the version's graph.
        // Marking a descriptor dynamic is conservative — a `{{ }}` in `vars` says
        // the arguments feed dbt, not that they change which models exist — so the
        // usual dynamic run duplicates a picture the read falls back to anyway.
        let deployed = sqlx::query_scalar!(
            "SELECT digest FROM dbt_graph_snapshot
              WHERE workspace_id = $1 AND script_path = $2 AND script_hash = $3
                AND job_id = $4",
            workspace_id,
            script_path,
            script_hash,
            DEPLOYED_GRAPH,
        )
        .fetch_optional(&mut **tx)
        .await?;
        if deployed.as_deref() == Some(digest.as_str()) {
            return Ok(());
        }
    }
    // Scoped to THIS version AND this run: wiping the path would take every
    // other version's graph with it, and wiping the version would take every
    // other run's snapshot — both are what keying exists to prevent.
    sqlx::query!(
        "DELETE FROM dbt_node WHERE workspace_id = $1 AND script_path = $2
           AND script_hash = $3 AND job_id = $4",
        workspace_id,
        script_path,
        script_hash,
        job_id
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "DELETE FROM dbt_edge WHERE workspace_id = $1 AND script_path = $2
           AND script_hash = $3 AND job_id = $4",
        workspace_id,
        script_path,
        script_hash,
        job_id
    )
    .execute(&mut **tx)
    .await?;
    // The marker, before the rows: a graph with no nodes at all is a legitimate
    // answer for a dynamic run that disabled every model, and the reader must be
    // able to tell it from a run that stored nothing.
    sqlx::query!(
        // The `WHERE` is not optional: a versioned graph's key is a PARTIAL unique
        // index (its version-less sibling is keyed by job alone), and Postgres
        // infers no arbiter from a partial index unless the statement repeats its
        // predicate.
        "INSERT INTO dbt_graph_snapshot
           (workspace_id, script_path, script_hash, job_id, digest, ingested_at)
         VALUES ($1, $2, $3, $4, $5, now())
         ON CONFLICT (workspace_id, script_path, script_hash, job_id)
           WHERE script_hash IS NOT NULL
         DO UPDATE SET digest = EXCLUDED.digest, ingested_at = now()",
        workspace_id,
        script_path,
        script_hash,
        job_id,
        digest
    )
    .execute(&mut **tx)
    .await?;

    insert_graph_rows(
        tx,
        workspace_id,
        script_path,
        Some(script_hash),
        job_id,
        ingested,
    )
    .await
}

/// The node and edge rows of one stored graph, whichever provenance keys it.
///
/// `script_hash` is `None` for a graph parsed from the editor's buffer, which
/// names no deployed version and is keyed to its own preview job.
async fn insert_graph_rows(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
    script_hash: Option<i64>,
    job_id: uuid::Uuid,
    ingested: &IngestedManifest,
) -> Result<()> {
    // Batched: a manifest carries a row per model, test, seed and snapshot, and
    // one awaited statement each made publication scale with database latency
    // while holding this transaction. Chunked because Postgres binds at most
    // 65535 parameters per statement.
    for chunk in ingested.nodes.chunks(NODE_INSERT_CHUNK) {
        let mut q = sqlx::QueryBuilder::new(
            "INSERT INTO dbt_node (workspace_id, script_path, script_hash, job_id, unique_id, \
             resource_type, name, asset_path, materialized, materialize_strategy, unique_key, \
             tags, description, test_kind, test_column, test_args, severity, attached_node, \
             columns, freshness, raw_code, original_file_path) ",
        );
        q.push_values(chunk, |mut b, n| {
            b.push_bind(workspace_id)
                .push_bind(script_path)
                .push_bind(script_hash)
                .push_bind(job_id)
                .push_bind(&n.unique_id)
                .push_bind(&n.resource_type)
                .push_bind(&n.name)
                .push_bind(&n.asset_path)
                .push_bind(&n.materialized)
                .push_bind(&n.materialize_strategy)
                .push_bind(&n.unique_key)
                .push_bind(&n.tags)
                .push_bind(&n.description)
                .push_bind(&n.test_kind)
                .push_bind(&n.test_column)
                .push_bind(&n.test_args)
                .push_bind(&n.severity)
                .push_bind(&n.attached_node)
                .push_bind(&n.columns)
                .push_bind(&n.freshness)
                .push_bind(&n.raw_code)
                .push_bind(&n.original_file_path);
        });
        q.build().execute(&mut **tx).await?;
    }

    for chunk in ingested.edges.chunks(EDGE_INSERT_CHUNK) {
        let mut q = sqlx::QueryBuilder::new(
            "INSERT INTO dbt_edge (workspace_id, script_path, script_hash, job_id, \
             parent_unique_id, child_unique_id) ",
        );
        q.push_values(chunk, |mut b, (parent, child)| {
            b.push_bind(workspace_id)
                .push_bind(script_path)
                .push_bind(script_hash)
                .push_bind(job_id)
                .push_bind(parent)
                .push_bind(child);
        });
        q.push(" ON CONFLICT DO NOTHING");
        q.build().execute(&mut **tx).await?;
    }
    Ok(())
}

/// How many buffer parses of one script, by one principal, keep their graph.
///
/// The editor reads back only the refresh it just launched, so one would do; the
/// slack covers a session where several tabs or a retried job are in flight.
pub const DBT_EDITOR_GRAPHS_KEPT: i64 = 5;

/// Store the graph a `parse` of the EDITOR's buffer produced.
///
/// The third provenance: not the version's graph, and not a run's snapshot of a
/// version. A buffer differs from what is deployed — that is the point of
/// refreshing it — and a project being written may have no deployed version at
/// all, so these rows carry no `script_hash` and are keyed to the preview job
/// that parsed them. `GET /jobs/dbt_graph/{id}` is the only way back to them;
/// nothing reaches them through the path, which is what keeps a `jobs:run`
/// principal from rewriting a deployed project's graph.
///
/// Unlike a run's snapshot this is never suppressed by matching the deployed
/// digest: the editor pins to its own job, so a buffer that happens to agree
/// with the deploy must still leave something to pin to.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn replace_dbt_editor_graph(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
    // The preview job that parsed the buffer, which is this graph's whole key.
    job_id: uuid::Uuid,
    // The identity that ran it, which is what the retention below is bounded
    // per. A preview's PATH is chosen by a caller who needs only `jobs:run`, so
    // a count per path alone would let one caller's parses evict the graphs of
    // whoever is actually editing that script.
    permissioned_as: &str,
    ingested: &IngestedManifest,
    relation_root: &str,
) -> Result<()> {
    // By job alone, so re-executing one — a zombie recovered onto another
    // worker — replaces its rows rather than colliding with them.
    for table in ["dbt_node", "dbt_edge", "dbt_graph_snapshot"] {
        sqlx::query(&format!(
            "DELETE FROM {table} WHERE workspace_id = $1 AND job_id = $2 AND script_hash IS NULL"
        ))
        .bind(workspace_id)
        .bind(job_id)
        .execute(&mut **tx)
        .await?;
    }
    // The marker, before the rows: a project whose models are all disabled
    // parses to an empty graph, and the reader has to tell that from a refresh
    // that stored nothing.
    sqlx::query!(
        "INSERT INTO dbt_graph_snapshot
           (workspace_id, script_path, script_hash, job_id, permissioned_as, digest, ingested_at)
         VALUES ($1, $2, NULL, $3, $4, $5, now())",
        workspace_id,
        script_path,
        job_id,
        permissioned_as,
        graph_digest(ingested, relation_root),
    )
    .execute(&mut **tx)
    .await?;
    insert_graph_rows(tx, workspace_id, script_path, None, job_id, ingested).await?;
    // Bounded here rather than by a sweep: a refresh is a click, and the ones
    // before it are dead the moment this one lands. The rows go with their
    // marker, in this transaction, so no restart can strand either half.
    //
    // Per (path, PRINCIPAL): the path is the caller's to name, so a bound over
    // it alone is a way to retire someone else's refreshes with nothing but
    // `jobs:run`. Each identity reclaims only its own.
    let retired: Vec<uuid::Uuid> = sqlx::query_scalar!(
        "DELETE FROM dbt_graph_snapshot g
          WHERE g.workspace_id = $1 AND g.script_path = $2 AND g.script_hash IS NULL
            AND g.permissioned_as IS NOT DISTINCT FROM $4
            AND g.job_id NOT IN (
              SELECT job_id FROM dbt_graph_snapshot
               WHERE workspace_id = $1 AND script_path = $2 AND script_hash IS NULL
                 AND permissioned_as IS NOT DISTINCT FROM $4
               ORDER BY ingested_at DESC LIMIT $3)
        RETURNING g.job_id",
        workspace_id,
        script_path,
        DBT_EDITOR_GRAPHS_KEPT,
        permissioned_as,
    )
    .fetch_all(&mut **tx)
    .await?;
    if !retired.is_empty() {
        for table in ["dbt_node", "dbt_edge"] {
            sqlx::query(&format!(
                "DELETE FROM {table} WHERE workspace_id = $1 AND job_id = ANY($2) \
                   AND script_hash IS NULL"
            ))
            .bind(workspace_id)
            .bind(&retired)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

/// Rows per statement, chosen so 22 columns stay under Postgres's 65535-bind
/// ceiling with room to spare.
const NODE_INSERT_CHUNK: usize = 2000;
/// Six columns, so the same ceiling allows far more.
const EDGE_INSERT_CHUNK: usize = 8000;

/// Clear one VERSION's graph: the delete-by-hash route, which only soft-deletes
/// its `script` row and so fires no cascade, and the ingest that finds no
/// warehouse identity left to key assets on. Both target a single `hash`, and the
/// other versions of the path stay live and keep needing their own models, SQL
/// and lineage.
///
/// There is no path-wide sibling. The routes that remove the `script` rows
/// outright let the tables' `ON DELETE CASCADE` take the graph, which is also
/// what keeps them from locking it ahead of the script row and deadlocking with
/// a concurrent publication.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn clear_dbt_manifest_version(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
    script_hash: i64,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM dbt_node WHERE workspace_id = $1 AND script_path = $2 AND script_hash = $3",
        workspace_id,
        script_path,
        script_hash
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "DELETE FROM dbt_edge WHERE workspace_id = $1 AND script_path = $2 AND script_hash = $3",
        workspace_id,
        script_path,
        script_hash
    )
    .execute(&mut **tx)
    .await?;
    // The marker too, and every job's: a marker left standing for rows that are
    // gone is read as a snapshot, and its digest still answers the suppression
    // check — so an identical run would write nothing and then render an empty
    // graph.
    sqlx::query!(
        "DELETE FROM dbt_graph_snapshot
          WHERE workspace_id = $1 AND script_path = $2 AND script_hash = $3",
        workspace_id,
        script_path,
        script_hash
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Drop the EDITOR graphs of one path, for the routes that delete the script
/// outright.
///
/// The versioned rows need no such call: they cascade off `script`, which is
/// what keeps a delete from locking the graph ahead of the script row. A
/// version-less row cannot ride that cascade — its `script_hash` is NULL, which
/// satisfies the composite foreign key without referencing anything — so
/// retiring the path is the one thing that has to reach them by hand.
///
/// Call it AFTER the `script` delete, like the retry state beside it: every dbt
/// writer takes the script row first, so a sidecar taken ahead of it deadlocks
/// one of the pair.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn clear_dbt_editor_graphs(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
) -> Result<()> {
    for table in ["dbt_node", "dbt_edge", "dbt_graph_snapshot"] {
        sqlx::query(&format!(
            "DELETE FROM {table}
              WHERE workspace_id = $1 AND script_path = $2 AND script_hash IS NULL"
        ))
        .bind(workspace_id)
        .bind(script_path)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Move a dbt script's saved retry state to its new path.
///
/// Keyed by path like the sidecar, but unlike the sidecar it is not
/// regenerated by anything: the deploy re-ingests a manifest, while these are
/// the results of a run that already happened. Clearing on rename would throw
/// away a resumable failure for a cosmetic change, so it travels instead.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn move_dbt_run_state(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<()> {
    // The destination may already hold state from a script that lived there
    // before; the incoming row is the newer truth for this project.
    sqlx::query!(
        "DELETE FROM dbt_run_state WHERE workspace_id = $1 AND script_path = $2",
        workspace_id,
        new_path
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "UPDATE dbt_run_state SET script_path = $3 WHERE workspace_id = $1 AND script_path = $2",
        workspace_id,
        old_path,
        new_path
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Drop the saved retry state, but only once NO live version of the path is
/// left.
///
/// `dbt_run_state`'s key is the path and the principal — one saved run per script
/// per identity it executes as, not
/// one per version — so archiving or deleting a single version must not take it
/// with them: the live version's `dbt retry` would be refused and the
/// partial-failure resume lost. It does not need to be version-scoped either,
/// because `identity` already refuses a resume whose project, warehouse or
/// engine moved.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn clear_dbt_run_state_if_path_retired(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM dbt_run_state WHERE workspace_id = $1 AND script_path = $2
           AND NOT EXISTS (SELECT 1 FROM script
                            WHERE workspace_id = $1 AND path = $2
                              AND deleted = false AND archived = false)",
        workspace_id,
        script_path
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Drop a dbt script's saved retry state.
///
/// Archive and delete: `run_results` is not small, the invocation arguments it
/// carries are the user's, and a script later created at the same path would
/// otherwise inherit a stranger's resumable failure.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn clear_dbt_run_state(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM dbt_run_state WHERE workspace_id = $1 AND script_path = $2",
        workspace_id,
        script_path
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured from a real `dbt parse` (dbt-core 1.12 / postgres); the same
    // shape is emitted by dbt-core 2.0 (manifest schema v12 in both).
    const MANIFEST: &str = r#"{
      "metadata": {"dbt_version": "1.12.0", "adapter_type": "postgres", "project_name": "jaffle_shop"},
      "nodes": {
        "model.jaffle_shop.customers": {
          "resource_type": "model", "name": "customers", "alias": "customers",
          "schema": "jaffle_dbt", "relation_name": "\"wh\".\"jaffle_dbt\".\"customers\"",
          "tags": ["nightly"], "config": {"materialized": "table"},
          "columns": {"customer_id": {"description": "pk"}},
          "raw_code": "select * from {{ ref('stg_customers') }}",
          "original_file_path": "models/customers.sql"
        },
        "model.jaffle_shop.orders_daily": {
          "resource_type": "model", "name": "orders_daily", "alias": "orders_daily",
          "schema": "jaffle_dbt", "relation_name": "\"wh\".\"jaffle_dbt\".\"orders_daily\"",
          "config": {"materialized": "incremental", "unique_key": "order_id"}
        },
        "model.jaffle_shop.order_events": {
          "resource_type": "model", "name": "order_events", "alias": "order_events",
          "schema": "jaffle_dbt", "database": "archive",
          "relation_name": "\"archive\".\"jaffle_dbt\".\"order_events\"",
          "config": {"materialized": "incremental"}
        },
        "model.jaffle_shop.composite_key": {
          "resource_type": "model", "name": "composite_key", "alias": "composite_key",
          "schema": "jaffle_dbt", "database": "wh",
          "relation_name": "\"wh\".\"jaffle_dbt\".\"composite_key\"",
          "config": {"materialized": "incremental", "unique_key": ["a", "b"]}
        },
        "model.jaffle_shop.stg_customers": {
          "resource_type": "model", "name": "stg_customers", "alias": "stg_customers",
          "schema": "jaffle_dbt", "relation_name": "\"wh\".\"jaffle_dbt\".\"stg_customers\"",
          "config": {"materialized": "view"}
        },
        "test.jaffle_shop.relationships_orders_daily_customer_id.ab": {
          "resource_type": "test", "name": "relationships_orders_daily_customer_id",
          "column_name": "customer_id", "attached_node": "model.jaffle_shop.orders_daily",
          "config": {"materialized": "test", "severity": "ERROR"},
          "test_metadata": {"name": "relationships",
                            "kwargs": {"to": "ref('stg_customers')", "field": "customer_id"},
                            "namespace": null}
        },
        "model.jaffle_shop.ephemeral_helper": {
          "resource_type": "model", "name": "ephemeral_helper", "schema": "jaffle_dbt",
          "relation_name": null, "config": {"materialized": "ephemeral"}
        },
        "snapshot.jaffle_shop.customers_snapshot": {
          "resource_type": "snapshot", "name": "customers_snapshot", "alias": "customers_snapshot",
          "schema": "jaffle_dbt_snapshots",
          "relation_name": "\"wh\".\"jaffle_dbt_snapshots\".\"customers_snapshot\"",
          "config": {"materialized": "snapshot", "unique_key": "customer_id",
                     "schema": "snapshots"}
        },
        "test.jaffle_shop.unique_customers_customer_id.c5": {
          "resource_type": "test", "name": "unique_customers_customer_id",
          "column_name": "customer_id", "attached_node": "model.jaffle_shop.customers",
          "config": {"materialized": "test", "severity": "ERROR"},
          "test_metadata": {"name": "unique", "kwargs": {"column_name": "customer_id"}, "namespace": null}
        },
        "test.jaffle_shop.accepted_values_orders_daily_status.14": {
          "resource_type": "test", "name": "accepted_values_orders_daily_status",
          "column_name": "status", "attached_node": "model.jaffle_shop.orders_daily",
          "config": {"materialized": "test", "severity": "ERROR"},
          "test_metadata": {"name": "accepted_values",
                            "kwargs": {"values": ["completed"], "column_name": "status"},
                            "namespace": null}
        },
        "test.jaffle_shop.dbt_utils_accepted_range_customers_n_orders__0.cb": {
          "resource_type": "test", "name": "dbt_utils_accepted_range_customers_n_orders__0",
          "column_name": "n_orders", "attached_node": "model.jaffle_shop.customers",
          "config": {"materialized": "test", "severity": "warn"},
          "test_metadata": {"name": "accepted_range", "kwargs": {"min_value": 0},
                            "namespace": "dbt_utils"}
        }
      },
      "sources": {
        "source.jaffle_shop.jaffle_raw.raw_orders": {
          "resource_type": "source", "name": "orders", "identifier": "raw_orders",
          "schema": "jaffle_raw",
          "relation_name": "\"wh\".\"jaffle_raw\".\"raw_orders\"",
          "freshness": {"warn_after": {"count": 1000, "period": "day"}}
        }
      },
      "parent_map": {
        "model.jaffle_shop.customers": ["model.jaffle_shop.orders_daily"],
        "model.jaffle_shop.orders_daily": ["source.jaffle_shop.jaffle_raw.raw_orders"],
        "model.jaffle_shop.stg_customers": ["source.jaffle_shop.jaffle_raw.raw_customers"],
        "test.jaffle_shop.relationships_orders_daily_customer_id.ab":
          ["model.jaffle_shop.orders_daily", "model.jaffle_shop.stg_customers"]
      }
    }"#;

    fn ingested() -> IngestedManifest {
        let m: Manifest = serde_json::from_str(MANIFEST).unwrap();
        ingest_manifest(&m, "main", Some("wh"), None)
    }

    fn node<'a>(i: &'a IngestedManifest, id: &str) -> &'a IngestedNode {
        i.nodes.iter().find(|n| n.unique_id == id).unwrap()
    }

    // dbt's four materializations must land on Windmill's four write
    // strategies, or the graph shows a model's write semantics wrong.
    #[test]
    fn materializations_map_onto_write_strategies() {
        let i = ingested();
        for (id, expected) in [
            ("model.jaffle_shop.customers", Some("replace")),
            ("model.jaffle_shop.orders_daily", Some("merge")),
            ("model.jaffle_shop.order_events", Some("append")),
            // A composite key is still an upsert; only its columns are
            // unmappable, so `unique_key` stays NULL while the strategy is merge.
            ("model.jaffle_shop.composite_key", Some("merge")),
            ("snapshot.jaffle_shop.customers_snapshot", Some("scd2")),
            // `ephemeral` is inlined as a CTE and writes nothing; `view`
            // produces a relation but performs no write. Neither has a
            // strategy, and inventing one would misreport the model.
            ("model.jaffle_shop.ephemeral_helper", None),
        ] {
            assert_eq!(
                node(&i, id).materialize_strategy.as_deref(),
                expected,
                "{id}"
            );
        }
    }

    // The four dbt generic tests are the four `// data_test` kinds. Package
    // tests keep their namespaced name instead of being dropped.
    #[test]
    fn generic_tests_carry_their_kind_column_and_severity() {
        let i = ingested();
        let unique = node(&i, "test.jaffle_shop.unique_customers_customer_id.c5");
        assert_eq!(unique.test_kind.as_deref(), Some("unique"));
        assert_eq!(unique.test_column.as_deref(), Some("customer_id"));
        assert_eq!(
            unique.attached_node.as_deref(),
            Some("model.jaffle_shop.customers")
        );
        assert_eq!(unique.severity.as_deref(), Some("ERROR"));

        let av = node(
            &i,
            "test.jaffle_shop.accepted_values_orders_daily_status.14",
        );
        assert_eq!(av.test_kind.as_deref(), Some("accepted_values"));
        assert_eq!(av.test_args.as_ref().unwrap()["values"][0], "completed");

        let pkg = node(
            &i,
            "test.jaffle_shop.dbt_utils_accepted_range_customers_n_orders__0.cb",
        );
        assert_eq!(pkg.test_kind.as_deref(), Some("dbt_utils.accepted_range"));
        // dbt-core 1.x echoes the author's casing, 2.x uppercases it — readers
        // must compare case-insensitively.
        assert_eq!(pkg.severity.as_deref(), Some("warn"));
    }

    // A source's `name` is what `source()` refers to; its `identifier` is the
    // table. Keying on the logical name would name a relation that does not
    // exist, so nothing reading the real one could ever join it.
    #[test]
    fn sources_key_on_their_identifier_not_their_logical_name() {
        let i = ingested();
        assert_eq!(
            node(&i, "source.jaffle_shop.jaffle_raw.raw_orders")
                .asset_path
                .as_deref(),
            Some("main/jaffle_raw/raw_orders")
        );
    }

    // The whole reason for `dbt://`: a model's key is the physical relation,
    // so a native script reading the same table lands on the same node.
    // Identifiers are canonicalized, and the database appears only when a model
    // overrode the target's.
    #[test]
    fn models_become_table_assets_keyed_on_the_warehouse() {
        let i = ingested();
        assert_eq!(
            node(&i, "model.jaffle_shop.customers")
                .asset_path
                .as_deref(),
            Some("main/jaffle_dbt/customers")
        );
        // `order_events` overrode its database. Dropping that would collapse it
        // onto a same-named relation in the target's own database, merging their
        // lineage and cascading into each other's consumers.
        assert_eq!(
            node(&i, "model.jaffle_shop.order_events")
                .asset_path
                .as_deref(),
            Some("main/archive.jaffle_dbt/order_events")
        );
        // `config.schema` on the snapshot is the custom SUFFIX (`snapshots`);
        // the relation actually lives in the resolved `jaffle_dbt_snapshots`.
        // Keying on the config value would name a table that does not exist.
        assert_eq!(
            node(&i, "snapshot.jaffle_shop.customers_snapshot")
                .asset_path
                .as_deref(),
            Some("main/jaffle_dbt_snapshots/customers_snapshot")
        );
        // No physical relation, no asset: nothing to key a `dbt://` path on.
        assert_eq!(
            node(&i, "model.jaffle_shop.ephemeral_helper").asset_path,
            None
        );
        assert_eq!(
            node(&i, "test.jaffle_shop.unique_customers_customer_id.c5").asset_path,
            None
        );
    }

    // `/` is what separates the segments a `dbt://` key is read back by, and a
    // quoted identifier may legally contain one — so two different relations
    // would spell the same key and share a node, its lineage and its progress.
    #[test]
    fn a_relation_whose_identifier_carries_a_separator_gets_no_key() {
        // The collision: `schema="a/b", name="c"` against `schema="a", name="b/c"`.
        assert_eq!(table_asset_path("main", None, "a/b", "c", None), None);
        assert_eq!(table_asset_path("main", None, "a", "b/c", None), None);
        // A qualifying database is part of the same segment, so it counts too.
        assert_eq!(
            table_asset_path("main", Some("d/b"), "s", "t", Some("other")),
            None
        );
        assert_eq!(
            table_asset_path("main", None, "a", "c", None).as_deref(),
            Some("main/a/c")
        );
    }

    // Models are what the script produces; sources are what it consumes. Get
    // this backwards and the cascade either never fires or fires on itself.
    #[test]
    fn script_writes_its_models_and_reads_its_sources() {
        let i = ingested();
        let got: Vec<(String, Option<AssetUsageAccessType>)> = i
            .assets
            .iter()
            .map(|a| {
                assert_eq!(a.kind, AssetKind::Dbt);
                (a.path.clone(), a.access_type)
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "main/archive.jaffle_dbt/order_events".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "main/jaffle_dbt/composite_key".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "main/jaffle_dbt/customers".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "main/jaffle_dbt/orders_daily".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "main/jaffle_dbt/stg_customers".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "main/jaffle_dbt_snapshots/customers_snapshot".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "main/jaffle_raw/raw_orders".into(),
                    Some(AssetUsageAccessType::R)
                ),
            ]
        );
    }

    // Splitting a project across scripts only composes if the seam still draws:
    // a script selecting a model whose parent another script builds must keep
    // that parent as an endpoint, or the two relations sit on the graph with no
    // line between them and the lineage silently stops at the selection border.
    #[test]
    fn a_parent_outside_the_selection_still_anchors_its_edge() {
        let m: Manifest = serde_json::from_str(MANIFEST).unwrap();
        // `customers` depends on `orders_daily`, which this script does not build.
        let sel: std::collections::HashSet<String> = ["model.jaffle_shop.customers".to_string()]
            .into_iter()
            .collect();
        let i = ingest_manifest(&m, "main", Some("wh"), Some(&sel));
        let owned: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::W))
            .map(|a| a.path.as_str())
            .collect();
        // The parent is NOT claimed as a write: the other script builds it.
        assert_eq!(owned, vec!["main/jaffle_dbt/customers"]);
        let reads: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::R))
            .map(|a| a.path.as_str())
            .collect();
        assert!(
            reads.contains(&"main/jaffle_dbt/orders_daily"),
            "the unselected parent is an input, got {reads:?}"
        );
        assert!(
            i.edges.contains(&(
                "model.jaffle_shop.orders_daily".to_string(),
                "model.jaffle_shop.customers".to_string()
            )),
            "the cross-selection edge must survive, got {:?}",
            i.edges
        );
    }

    // A script that builds a subset must not register as the producer of the
    // whole project: two scripts splitting one project would each claim all of it.
    #[test]
    fn selection_scopes_what_the_script_owns() {
        let m: Manifest = serde_json::from_str(MANIFEST).unwrap();
        let sel: std::collections::HashSet<String> = ["model.jaffle_shop.orders_daily".to_string()]
            .into_iter()
            .collect();
        let i = ingest_manifest(&m, "main", Some("wh"), Some(&sel));
        let owned: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::W))
            .map(|a| a.path.as_str())
            .collect();
        assert_eq!(owned, vec!["main/jaffle_dbt/orders_daily"]);
        // The source the selected model reads is kept — that is how the graph
        // knows the input. A source only unselected models read is NOT, or the
        // script would subscribe to tables it never touches.
        let reads: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::R))
            .map(|a| a.path.as_str())
            .collect();
        assert_eq!(reads, vec!["main/jaffle_raw/raw_orders"]);
        // Edges to nodes this script does not own are dropped with them.
        assert_eq!(
            i.edges,
            vec![(
                "source.jaffle_shop.jaffle_raw.raw_orders".to_string(),
                "model.jaffle_shop.orders_daily".to_string()
            )]
        );
    }

    // The graph renders a model by its own SQL, so the ingest has to carry it —
    // and a test's generated body is noise nobody reads.
    #[test]
    fn models_carry_their_sql_and_tests_do_not() {
        let i = ingested();
        let m = node(&i, "model.jaffle_shop.customers");
        assert!(
            m.raw_code.as_deref().is_some_and(|c| !c.is_empty()),
            "{:?}",
            m.raw_code
        );
        assert_eq!(
            m.original_file_path.as_deref(),
            Some("models/customers.sql")
        );
        let t = i.nodes.iter().find(|n| n.resource_type == "test").unwrap();
        assert_eq!(t.raw_code, None);
    }

    // A read is an upstream edge, so a source no model reads must not
    // become one — otherwise loading an unused table dispatches the whole dbt
    // script. This holds for the unselected, whole-project case too, which is
    // the common one.
    #[test]
    fn a_source_nothing_reads_is_not_an_input() {
        let mut v: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        v["sources"]["source.jaffle_shop.jaffle_raw.unused"] = serde_json::json!({
            "resource_type": "source", "name": "unused", "identifier": "unused",
            "schema": "jaffle_raw", "relation_name": "\"wh\".\"jaffle_raw\".\"unused\""
        });
        let m: Manifest = serde_json::from_value(v).unwrap();
        let i = ingest_manifest(&m, "main", Some("wh"), None);
        let reads: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::R))
            .map(|a| a.path.as_str())
            .collect();
        assert!(
            !reads.contains(&"main/jaffle_raw/unused"),
            "unused source registered as a read: {reads:?}"
        );
        assert!(reads.contains(&"main/jaffle_raw/raw_orders"), "{reads:?}");
    }

    // Splitting a project across scripts only composes if the downstream one
    // reads what the upstream one writes: without that read there is no edge
    // so the seam between two scripts splitting one project still draws.
    #[test]
    fn a_selected_script_reads_the_upstream_models_another_script_builds() {
        let m: Manifest = serde_json::from_str(MANIFEST).unwrap();
        // `customers` is selected; its parent `orders_daily` is built elsewhere.
        let sel: std::collections::HashSet<String> = ["model.jaffle_shop.customers".to_string()]
            .into_iter()
            .collect();
        let i = ingest_manifest(&m, "main", Some("wh"), Some(&sel));
        let by_access = |t: AssetUsageAccessType| -> Vec<&str> {
            i.assets
                .iter()
                .filter(|a| a.access_type == Some(t))
                .map(|a| a.path.as_str())
                .collect()
        };
        assert_eq!(
            by_access(AssetUsageAccessType::W),
            vec!["main/jaffle_dbt/customers"]
        );
        assert_eq!(
            by_access(AssetUsageAccessType::R),
            vec!["main/jaffle_dbt/orders_daily"]
        );

        // Selecting a model also selects its tests, and the `relationships`
        // test on orders_daily points at stg_customers. A script that only
        // builds staging models must not end up subscribed to the mart it is
        // merely asserted against.
        let staging: std::collections::HashSet<String> = [
            "model.jaffle_shop.stg_customers".to_string(),
            "test.jaffle_shop.relationships_orders_daily_customer_id.ab".to_string(),
        ]
        .into_iter()
        .collect();
        let i = ingest_manifest(&m, "main", Some("wh"), Some(&staging));
        assert!(
            !i.assets.iter().any(|a| a.path.ends_with("/orders_daily")),
            "a test's dependency must not become a read: {:?}",
            i.assets.iter().map(|a| &a.path).collect::<Vec<_>>()
        );
    }

    // An ephemeral parent is inlined as a CTE and produces nothing to depend on,
    // but what IT reads is still the selected model's real input — stopping
    // there loses the edge and the model shows no upstream.
    #[test]
    fn dependencies_traverse_through_ephemeral_parents() {
        let m: Manifest = serde_json::from_str(
            r#"{"nodes":{
              "model.p.mart":{"resource_type":"model","name":"mart","alias":"mart",
                "schema":"s","database":"wh","relation_name":"\"wh\".\"s\".\"mart\"",
                "config":{"materialized":"table"}},
              "model.p.helper":{"resource_type":"model","name":"helper","schema":"s",
                "relation_name":null,"config":{"materialized":"ephemeral"}},
              "model.p.base":{"resource_type":"model","name":"base","alias":"base",
                "schema":"s","database":"wh","relation_name":"\"wh\".\"s\".\"base\"",
                "config":{"materialized":"view"}}},
              "parent_map":{"model.p.mart":["model.p.helper"],
                            "model.p.helper":["model.p.base"]}}"#,
        )
        .unwrap();
        let sel: std::collections::HashSet<String> =
            ["model.p.mart".to_string()].into_iter().collect();
        let i = ingest_manifest(&m, "main", Some("wh"), Some(&sel));
        let reads: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::R))
            .map(|a| a.path.as_str())
            .collect();
        assert_eq!(reads, vec!["main/s/base"]);
    }

    // `A -> ephemeral -> B` must still read as `A -> B`: the ephemeral node has
    // no relation to draw, but the lineage it carries is real, and dropping both
    // of its edges leaves the reader with none.
    #[test]
    fn lineage_contracts_through_ephemeral_models() {
        let m: Manifest = serde_json::from_str(
            r#"{"nodes":{
              "model.p.a":{"resource_type":"model","name":"a","alias":"a","schema":"s",
                "database":"wh","relation_name":"\"wh\".\"s\".\"a\"",
                "config":{"materialized":"view"}},
              "model.p.e":{"resource_type":"model","name":"e","schema":"s",
                "relation_name":null,"config":{"materialized":"ephemeral"}},
              "model.p.b":{"resource_type":"model","name":"b","alias":"b","schema":"s",
                "database":"wh","relation_name":"\"wh\".\"s\".\"b\"",
                "config":{"materialized":"table"}}},
              "parent_map":{"model.p.e":["model.p.a"],"model.p.b":["model.p.e"]}}"#,
        )
        .unwrap();
        let i = ingest_manifest(&m, "main", Some("wh"), None);
        assert_eq!(
            i.edges,
            vec![("model.p.a".to_string(), "model.p.b".to_string())]
        );
    }

    #[test]
    fn ref_edges_come_from_the_parent_map() {
        let i = ingested();
        assert_eq!(
            i.edges,
            vec![
                (
                    "model.jaffle_shop.orders_daily".to_string(),
                    "model.jaffle_shop.customers".to_string()
                ),
                // A test's own edges are kept — the graph renders them — even
                // though they do not make the script depend on what they assert.
                (
                    "model.jaffle_shop.orders_daily".to_string(),
                    "test.jaffle_shop.relationships_orders_daily_customer_id.ab".to_string()
                ),
                (
                    "model.jaffle_shop.stg_customers".to_string(),
                    "test.jaffle_shop.relationships_orders_daily_customer_id.ab".to_string()
                ),
                (
                    "source.jaffle_shop.jaffle_raw.raw_orders".to_string(),
                    "model.jaffle_shop.orders_daily".to_string()
                ),
            ]
        );
    }

    /// The whole manifest crosses the wire when an agent worker publishes its
    /// graph, so what `ingest_manifest` produces has to survive its own
    /// serializer. A field skipped when empty and required on the way back makes
    /// every publish a 422 — invisible to a normal worker, which writes the rows
    /// directly.
    #[test]
    fn an_ingested_manifest_survives_the_agent_wire() {
        let manifest: Manifest = serde_json::from_str(
            r#"{"nodes":{"model.p.customers":{
                 "resource_type":"model","name":"customers","alias":"customers",
                 "schema":"analytics","database":"wh",
                 "relation_name":"\"wh\".\"analytics\".\"customers\""}},
               "parent_map":{"model.p.customers":[]}}"#,
        )
        .unwrap();
        let ingested = ingest_manifest(&manifest, "main", Some("wh"), None);
        assert!(!ingested.assets.is_empty(), "fixture must produce an asset");

        let wire = serde_json::to_value(&ingested).unwrap();
        let back: IngestedManifest = serde_json::from_value(wire.clone())
            .unwrap_or_else(|e| panic!("agent publish would 422: {e}\npayload: {wire}"));
        assert_eq!(back.nodes.len(), ingested.nodes.len());
        assert_eq!(back.assets.len(), ingested.assets.len());
        assert_eq!(back.assets[0].path, ingested.assets[0].path);
    }
}

/// Record one model's state for THIS RUN.
///
/// NO AUTHORIZATION: writes whatever `w_id`/`job_id` it is given. Callers MUST
/// already have established that the job is theirs — the API route takes the id
/// from the token rather than the body for exactly this reason — and MUST pass a
/// job that belongs to that workspace.
///
/// Alongside `record_materialization`, never instead of it: that table holds the
/// current state of a relation, one row, and its `job_id` is only the last
/// writer — two runs of a project building the same models overwrite each
/// other's. The run page needs what THIS job did, so it gets its own rows.
///
/// Shared with the API because an agent worker has no database and posts its
/// settled outcomes to a route that calls this same function — two spellings of
/// the row would drift.
pub async fn record_run_progress(
    db: &crate::DB,
    w_id: &str,
    job_id: &uuid::Uuid,
    asset_path: &str,
    status: crate::materialization::MaterializationStatus,
    row_count: Option<i64>,
    error: Option<&str>,
) {
    let res = sqlx::query!(
        "INSERT INTO dbt_run_progress
           (workspace_id, job_id, asset_kind, asset_path, status, row_count, error, updated_at)
         VALUES ($1, $2, 'dbt', $3, $4, $5, $6, now())
         ON CONFLICT (workspace_id, job_id, asset_kind, asset_path)
         DO UPDATE SET status = EXCLUDED.status, row_count = EXCLUDED.row_count,
                       error = EXCLUDED.error, updated_at = now()",
        w_id,
        job_id,
        asset_path,
        status as crate::materialization::MaterializationStatus,
        row_count,
        error,
    )
    .execute(db)
    .await;
    if let Err(e) = res {
        // Progress is a display, not the run: a failure here must not fail a
        // build that is otherwise fine.
        tracing::warn!("recording dbt run progress for {asset_path}: {e:#}");
    }
}

/// One settled node, posted by a worker that has no database.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DbtRunProgressRequest {
    pub asset_path: String,
    pub status: crate::materialization::MaterializationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
