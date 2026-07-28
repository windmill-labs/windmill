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
//!   `table://<resource_path>/<schema>/<name>`, never `dbt://…`. Keying on the
//!   producing tool would mean a native script reading the same warehouse table
//!   forms no edge, and the cross-boundary cascade — the reason to build the
//!   graph at all — never fires (decision 11).
//! * **The warehouse is identified by the Windmill resource path**, matching
//!   `datatable://` and `ducklake://`. Connection details (host, account) are
//!   never part of the key: the same warehouse is reachable under several
//!   hostnames, and credential material has no business in an asset key. The
//!   resource names the default database too, so relations in it need no
//!   qualification; one that OVERRODE its database qualifies its schema segment
//!   (`table_asset_path`), since two same-named relations in different databases
//!   are not the same table. The accepted consequence is documented in
//!   docs/dbt-runtime.md — two resources pointing at one physical warehouse do
//!   not unify.
//!
//! # Mutator contract
//!
//! `replace_dbt_manifest`, `clear_dbt_manifest` and
//! `clear_dbt_manifest_by_script_hash` take the workspace and the script to act
//! on as plain arguments and enforce nothing: **the caller must already have
//! verified write access to that script**, exactly like the sibling
//! `assets::replace_static_asset_usage` each is called next to. A user-scoped
//! transaction is not enforcement here — `dbt_node` and `dbt_edge` carry no RLS
//! policy and grant `windmill_user` full access, deliberately, because the
//! writer is the dependency job rather than a request.
//!
//! Every caller today satisfies it through the route that owns the script:
//! `windmill-api-scripts::scripts` clears from the create, archive and delete
//! handlers, each behind a `scripts:write:<path>` scope check plus that route's
//! own owner or admin requirement; the worker replaces from the dependency job
//! of the very script being deployed. A new call site that cannot name where it
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
#[derive(Serialize, Debug, Clone, PartialEq)]
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

#[derive(Debug, Default)]
pub struct IngestedManifest {
    pub nodes: Vec<IngestedNode>,
    pub edges: Vec<(String, String)>,
    /// The `asset` rows the owning script produces (models) and consumes
    /// (sources) — what drives the cascade and the lineage graph.
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

/// `table://` path of the relation a node resolves to, or `None` when it has
/// none. Assembles the parts; `table_asset_path` owns the spelling.
fn asset_path_for(
    node: &ManifestNode,
    resource_path: &str,
    default_database: Option<&str>,
) -> Option<String> {
    node.relation_name.as_ref()?;
    // `schema` is dbt's RESOLVED schema; `config.schema` is only the custom
    // suffix `generate_schema_name` combines with the target's (a snapshot
    // configured `schema: snapshots` under target schema `analytics` lands in
    // `analytics_snapshots`). Keying on the config value would name a relation
    // that does not exist, and the mismatch is invisible until nothing links.
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
    Some(table_asset_path(
        resource_path,
        node.database.as_deref(),
        schema,
        name,
        default_database,
    ))
}

/// The one derivation of a `table://` path from a relation's parts.
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
/// `<database>.<schema>`, so two same-named relations cannot collapse. When the
/// default is unknown — the project brought its own `profiles.yml`, so Windmill
/// never saw a database — every relation that names one qualifies, because
/// assuming they all share a database is what would collapse them.
pub fn table_asset_path(
    resource_path: &str,
    database: Option<&str>,
    schema: &str,
    name: &str,
    default_database: Option<&str>,
) -> String {
    let qualified = match database.map(str::trim).filter(|d| !d.is_empty()) {
        Some(db) if !default_database.is_some_and(|d| d.eq_ignore_ascii_case(db)) => {
            format!("{db}.{schema}")
        }
        _ => schema.to_string(),
    };
    canonicalize_table_asset_path(&format!("{resource_path}/{qualified}/{name}"))
}

/// Parse a `manifest.json` into rows, edges and asset usages.
///
/// `resource_path` is the Windmill path of the warehouse resource the profile
/// target points at, e.g. `f/prod/snowflake`.
/// `selected` is the node set the descriptor's `select`/`exclude` resolves to,
/// as reported by dbt itself (`dbt ls`). `None` means the whole project. It
/// scopes what this script is recorded as owning: a script that builds only
/// `tag:nightly` must not register as the producer of every other model, or the
/// cascade fires downstream of models it never touched. Running several scripts
/// with different selections is the intended shape (docs/dbt-runtime.md,
/// decision 6), and this is what makes them compose.
pub fn ingest_manifest(
    manifest: &Manifest,
    resource_path: &str,
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

    // Direct parents of the selected set. A source is only this script's input
    // if something it actually builds reads it — keeping every source would
    // make a narrowly-selected script claim reads on tables it never touches,
    // and those reads are cascade subscriptions. The same set answers the
    // cross-config question below.
    // Without a selection the script builds everything, so every node is its
    // own builder — but a source nothing reads is still not an input.
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
            // model then never cascades when its actual source changes.
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
        // Whether this script SELECTED the node — i.e. builds it — as opposed to
        // merely depending on it. A selection's direct parents are kept either
        // way, because an edge needs both endpoints: drop the parent and the
        // `ref()` that reaches it has nothing to point at, leaving two relations
        // on the graph with no line between them.
        let is_selected = selected.is_none_or(|sel| sel.contains(unique_id.as_str()));
        let keep = match node.resource_type.as_str() {
            "source" => direct_parents.contains(unique_id.as_str()),
            _ => is_selected || direct_parents.contains(unique_id.as_str()),
        };
        if !keep {
            continue;
        }
        let asset_path = asset_path_for(node, resource_path, default_database);
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
        let key = (AssetKind::Table, path);
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
    ingested: &IngestedManifest,
    // Where the profile put these relations, so a later run can tell whether the
    // resource has moved since — see the migration.
    relation_root: &str,
) -> Result<()> {
    clear_dbt_manifest(tx, workspace_id, script_path).await?;

    for n in &ingested.nodes {
        sqlx::query!(
            "INSERT INTO dbt_node (workspace_id, script_path, unique_id, resource_type, name,
                 asset_path, materialized, materialize_strategy, unique_key, tags, description,
                 test_kind, test_column, test_args, severity, attached_node, columns, freshness,
                 relation_root, raw_code, original_file_path)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17,
                     $18, $19, $20, $21)",
            workspace_id,
            script_path,
            n.unique_id,
            n.resource_type,
            n.name,
            n.asset_path,
            n.materialized,
            n.materialize_strategy,
            n.unique_key,
            &n.tags,
            n.description,
            n.test_kind,
            n.test_column,
            n.test_args,
            n.severity,
            n.attached_node,
            n.columns,
            n.freshness,
            relation_root,
            n.raw_code,
            n.original_file_path,
        )
        .execute(&mut **tx)
        .await?;
    }

    for (parent, child) in &ingested.edges {
        sqlx::query!(
            "INSERT INTO dbt_edge (workspace_id, script_path, parent_unique_id, child_unique_id)
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            workspace_id,
            script_path,
            parent,
            child
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Drop everything one script contributed to the graph. See the mutator
/// contract above: this authorizes nothing.
pub async fn clear_dbt_manifest(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM dbt_node WHERE workspace_id = $1 AND script_path = $2",
        workspace_id,
        script_path
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "DELETE FROM dbt_edge WHERE workspace_id = $1 AND script_path = $2",
        workspace_id,
        script_path
    )
    .execute(&mut **tx)
    .await?;
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

/// The by-hash sibling of `clear_dbt_run_state`, for the archive and delete
/// routes that only have a hash.
///
/// See the mutator contract above: this authorizes nothing.
pub async fn clear_dbt_run_state_by_script_hash(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_hash: crate::scripts::ScriptHash,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM dbt_run_state WHERE workspace_id = $1
           AND script_path = (SELECT path FROM script WHERE hash = $2 AND workspace_id = $1)",
        workspace_id,
        script_hash.0
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

/// Drop everything the script at `script_hash`'s path contributed. The
/// by-hash sibling of `clear_dbt_manifest`, for the delete/archive routes that
/// only have a hash — `dbt_node` has no script foreign key, so every one of
/// them has to clear explicitly or stale provenance outlives its script and
/// attaches itself to whatever is created at that path next.
///
/// See the mutator contract above: this authorizes nothing. `script_hash`
/// selects the path to clear; it is not a capability.
pub async fn clear_dbt_manifest_by_script_hash(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_hash: crate::scripts::ScriptHash,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM dbt_node WHERE workspace_id = $1
           AND script_path = (SELECT path FROM script WHERE hash = $2 AND workspace_id = $1)",
        workspace_id,
        script_hash.0
    )
    .execute(&mut **tx)
    .await?;
    sqlx::query!(
        "DELETE FROM dbt_edge WHERE workspace_id = $1
           AND script_path = (SELECT path FROM script WHERE hash = $2 AND workspace_id = $1)",
        workspace_id,
        script_hash.0
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
        ingest_manifest(&m, "f/prod/wh", Some("wh"), None)
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
            Some("f/prod/wh/jaffle_raw/raw_orders")
        );
    }

    // The whole reason for `table://`: a model's key is the physical relation,
    // so a native script reading the same table lands on the same node.
    // Identifiers are canonicalized, and the database appears only when a model
    // overrode the target's.
    #[test]
    fn models_become_table_assets_keyed_on_the_resource_path() {
        let i = ingested();
        assert_eq!(
            node(&i, "model.jaffle_shop.customers")
                .asset_path
                .as_deref(),
            Some("f/prod/wh/jaffle_dbt/customers")
        );
        // `order_events` overrode its database. Dropping that would collapse it
        // onto a same-named relation in the target's own database, merging their
        // lineage and cascading into each other's consumers.
        assert_eq!(
            node(&i, "model.jaffle_shop.order_events")
                .asset_path
                .as_deref(),
            Some("f/prod/wh/archive.jaffle_dbt/order_events")
        );
        // `config.schema` on the snapshot is the custom SUFFIX (`snapshots`);
        // the relation actually lives in the resolved `jaffle_dbt_snapshots`.
        // Keying on the config value would name a table that does not exist.
        assert_eq!(
            node(&i, "snapshot.jaffle_shop.customers_snapshot")
                .asset_path
                .as_deref(),
            Some("f/prod/wh/jaffle_dbt_snapshots/customers_snapshot")
        );
        // No physical relation, no asset — `dbt://` stays reserved and unused.
        assert_eq!(
            node(&i, "model.jaffle_shop.ephemeral_helper").asset_path,
            None
        );
        assert_eq!(
            node(&i, "test.jaffle_shop.unique_customers_customer_id.c5").asset_path,
            None
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
                assert_eq!(a.kind, AssetKind::Table);
                (a.path.clone(), a.access_type)
            })
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "f/prod/wh/archive.jaffle_dbt/order_events".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "f/prod/wh/jaffle_dbt/composite_key".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "f/prod/wh/jaffle_dbt/customers".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "f/prod/wh/jaffle_dbt/orders_daily".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "f/prod/wh/jaffle_dbt/stg_customers".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "f/prod/wh/jaffle_dbt_snapshots/customers_snapshot".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "f/prod/wh/jaffle_raw/raw_orders".into(),
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
        let sel: std::collections::HashSet<String> =
            ["model.jaffle_shop.customers".to_string()].into_iter().collect();
        let i = ingest_manifest(&m, "f/prod/wh", Some("wh"), Some(&sel));
        let owned: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::W))
            .map(|a| a.path.as_str())
            .collect();
        // The parent is NOT claimed as a write: the other script builds it.
        assert_eq!(owned, vec!["f/prod/wh/jaffle_dbt/customers"]);
        let reads: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::R))
            .map(|a| a.path.as_str())
            .collect();
        assert!(
            reads.contains(&"f/prod/wh/jaffle_dbt/orders_daily"),
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
        let sel: std::collections::HashSet<String> =
            ["model.jaffle_shop.orders_daily".to_string()].into_iter().collect();
        let i = ingest_manifest(&m, "f/prod/wh", Some("wh"), Some(&sel));
        let owned: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::W))
            .map(|a| a.path.as_str())
            .collect();
        assert_eq!(owned, vec!["f/prod/wh/jaffle_dbt/orders_daily"]);
        // The source the selected model reads is kept — that is how the graph
        // knows the input. A source only unselected models read is NOT, or the
        // script would subscribe to tables it never touches.
        let reads: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::R))
            .map(|a| a.path.as_str())
            .collect();
        assert_eq!(reads, vec!["f/prod/wh/jaffle_raw/raw_orders"]);
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
        assert!(m.raw_code.as_deref().is_some_and(|c| !c.is_empty()), "{:?}", m.raw_code);
        assert_eq!(m.original_file_path.as_deref(), Some("models/customers.sql"));
        let t = i.nodes.iter().find(|n| n.resource_type == "test").unwrap();
        assert_eq!(t.raw_code, None);
    }

    // A read is a cascade subscription, so a source no model reads must not
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
        let i = ingest_manifest(&m, "f/prod/wh", Some("wh"), None);
        let reads: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::R))
            .map(|a| a.path.as_str())
            .collect();
        assert!(
            !reads.contains(&"f/prod/wh/jaffle_raw/unused"),
            "unused source registered as a read: {reads:?}"
        );
        assert!(reads.contains(&"f/prod/wh/jaffle_raw/raw_orders"), "{reads:?}");
    }

    // Splitting a project across scripts only composes if the downstream one
    // reads what the upstream one writes: without that read there is no edge
    // for the upstream's write to cascade along.
    #[test]
    fn a_selected_script_reads_the_upstream_models_another_script_builds() {
        let m: Manifest = serde_json::from_str(MANIFEST).unwrap();
        // `customers` is selected; its parent `orders_daily` is built elsewhere.
        let sel: std::collections::HashSet<String> =
            ["model.jaffle_shop.customers".to_string()].into_iter().collect();
        let i = ingest_manifest(&m, "f/prod/wh", Some("wh"), Some(&sel));
        let by_access = |t: AssetUsageAccessType| -> Vec<&str> {
            i.assets
                .iter()
                .filter(|a| a.access_type == Some(t))
                .map(|a| a.path.as_str())
                .collect()
        };
        assert_eq!(by_access(AssetUsageAccessType::W), vec!["f/prod/wh/jaffle_dbt/customers"]);
        assert_eq!(
            by_access(AssetUsageAccessType::R),
            vec!["f/prod/wh/jaffle_dbt/orders_daily"]
        );

        // Selecting a model also selects its tests, and the `relationships`
        // test on orders_daily points at stg_customers. A script that only
        // builds staging models must not end up subscribed to the mart it is
        // merely asserted against.
        let staging: std::collections::HashSet<String> =
            ["model.jaffle_shop.stg_customers".to_string(),
             "test.jaffle_shop.relationships_orders_daily_customer_id.ab".to_string()]
                .into_iter()
                .collect();
        let i = ingest_manifest(&m, "f/prod/wh", Some("wh"), Some(&staging));
        assert!(
            !i.assets
                .iter()
                .any(|a| a.path.ends_with("/orders_daily")),
            "a test's dependency must not become a read: {:?}",
            i.assets.iter().map(|a| &a.path).collect::<Vec<_>>()
        );
    }

    // An ephemeral parent is inlined as a CTE and produces nothing to depend on,
    // but what IT reads is still the selected model's real input — stopping
    // there loses the subscription and the model never cascades.
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
        let i = ingest_manifest(&m, "f/prod/wh", Some("wh"), Some(&sel));
        let reads: Vec<&str> = i
            .assets
            .iter()
            .filter(|a| a.access_type == Some(AssetUsageAccessType::R))
            .map(|a| a.path.as_str())
            .collect();
        assert_eq!(reads, vec!["f/prod/wh/s/base"]);
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
        let i = ingest_manifest(&m, "f/prod/wh", Some("wh"), None);
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
}
