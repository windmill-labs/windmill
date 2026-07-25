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
//!   `datatable://` and `ducklake://`. Connection details (host, account,
//!   database) are never part of the key: the same warehouse is reachable under
//!   several hostnames, and credential material has no business in an asset
//!   key. The accepted consequence is documented in docs/dbt-runtime.md — two
//!   resources pointing at one physical warehouse do not unify.

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
    #[serde(default)]
    pub schema: Option<String>,
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
    #[serde(default)]
    pub schema: Option<String>,
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
pub fn materialize_strategy_for(materialized: &str, unique_key: Option<&str>) -> Option<String> {
    match materialized {
        "table" | "seed" => Some("replace".to_string()),
        "incremental" | "microbatch" => Some(match unique_key {
            Some(_) => "merge".to_string(),
            None => "append".to_string(),
        }),
        "snapshot" => Some("scd2".to_string()),
        _ => None,
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
/// none. The database segment of dbt's relation is intentionally dropped: the
/// warehouse (and its database) is already identified by `resource_path`.
fn asset_path_for(node: &ManifestNode, resource_path: &str) -> Option<String> {
    node.relation_name.as_ref()?;
    let schema = node.config.schema.as_ref().or(node.schema.as_ref())?;
    let name = node.alias.as_ref().unwrap_or(&node.name);
    Some(canonicalize_table_asset_path(&format!(
        "{resource_path}/{schema}/{name}"
    )))
}

/// Parse a `manifest.json` into rows, edges and asset usages.
///
/// `resource_path` is the Windmill path of the warehouse resource the profile
/// target points at, e.g. `f/prod/snowflake`.
pub fn ingest_manifest(manifest: &Manifest, resource_path: &str) -> IngestedManifest {
    let mut out = IngestedManifest {
        dbt_version: manifest.metadata.dbt_version.clone(),
        adapter_type: manifest.metadata.adapter_type.clone(),
        ..Default::default()
    };
    let mut assets: HashMap<(AssetKind, String), AssetUsageAccessType> = HashMap::new();

    for (unique_id, node) in manifest.nodes.iter().chain(manifest.sources.iter()) {
        let asset_path = asset_path_for(node, resource_path);
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
            materialize_strategy: materialized
                .as_deref()
                .and_then(|m| materialize_strategy_for(m, unique_key.as_deref())),
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
        });

        // The dbt script writes what it materializes and reads its sources.
        // Internal `ref()`s between its own models are lineage inside the
        // project, not usages of the script — modelling them as reads would
        // make the script depend on its own output.
        let Some(path) = asset_path else { continue };
        let access = match node.resource_type.as_str() {
            "model" | "snapshot" | "seed" => AssetUsageAccessType::W,
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

    for (child, parents) in &manifest.parent_map {
        for parent in parents {
            out.edges.push((parent.clone(), child.clone()));
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
pub async fn replace_dbt_manifest(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    script_path: &str,
    ingested: &IngestedManifest,
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

    for n in &ingested.nodes {
        sqlx::query!(
            "INSERT INTO dbt_node (workspace_id, script_path, unique_id, resource_type, name,
                 asset_path, materialized, materialize_strategy, unique_key, tags, description,
                 test_kind, test_column, test_args, severity, attached_node, columns, freshness)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)",
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
          "columns": {"customer_id": {"description": "pk"}}
        },
        "model.jaffle_shop.orders_daily": {
          "resource_type": "model", "name": "orders_daily", "alias": "orders_daily",
          "schema": "jaffle_dbt", "relation_name": "\"wh\".\"jaffle_dbt\".\"orders_daily\"",
          "config": {"materialized": "incremental", "unique_key": "order_id"}
        },
        "model.jaffle_shop.order_events": {
          "resource_type": "model", "name": "order_events", "alias": "order_events",
          "schema": "jaffle_dbt", "relation_name": "\"wh\".\"jaffle_dbt\".\"order_events\"",
          "config": {"materialized": "incremental"}
        },
        "model.jaffle_shop.ephemeral_helper": {
          "resource_type": "model", "name": "ephemeral_helper", "schema": "jaffle_dbt",
          "relation_name": null, "config": {"materialized": "ephemeral"}
        },
        "snapshot.jaffle_shop.customers_snapshot": {
          "resource_type": "snapshot", "name": "customers_snapshot", "alias": "customers_snapshot",
          "schema": "jaffle_dbt_snapshots",
          "relation_name": "\"wh\".\"jaffle_dbt_snapshots\".\"customers_snapshot\"",
          "config": {"materialized": "snapshot", "unique_key": "customer_id"}
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
          "resource_type": "source", "name": "raw_orders", "schema": "jaffle_raw",
          "relation_name": "\"wh\".\"jaffle_raw\".\"raw_orders\"",
          "freshness": {"warn_after": {"count": 1000, "period": "day"}}
        }
      },
      "parent_map": {
        "model.jaffle_shop.customers": ["model.jaffle_shop.orders_daily"],
        "model.jaffle_shop.orders_daily": ["source.jaffle_shop.jaffle_raw.raw_orders"]
      }
    }"#;

    fn ingested() -> IngestedManifest {
        let m: Manifest = serde_json::from_str(MANIFEST).unwrap();
        ingest_manifest(&m, "f/prod/wh")
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

    // The whole reason for `table://`: a model's key is the physical relation,
    // so a native script reading the same table lands on the same node. The
    // database segment is dropped (the resource identifies the warehouse) and
    // identifiers are canonicalized.
    #[test]
    fn models_become_table_assets_keyed_on_the_resource_path() {
        let i = ingested();
        assert_eq!(
            node(&i, "model.jaffle_shop.customers")
                .asset_path
                .as_deref(),
            Some("f/prod/wh/jaffle_dbt/customers")
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
                    "f/prod/wh/jaffle_dbt/customers".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "f/prod/wh/jaffle_dbt/order_events".into(),
                    Some(AssetUsageAccessType::W)
                ),
                (
                    "f/prod/wh/jaffle_dbt/orders_daily".into(),
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
                (
                    "source.jaffle_shop.jaffle_raw.raw_orders".to_string(),
                    "model.jaffle_shop.orders_daily".to_string()
                ),
            ]
        );
    }
}
