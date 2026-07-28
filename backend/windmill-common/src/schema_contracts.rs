//! Save-time schema-contract check (pipelines gap #2b): validate a consumer
//! script's asset references against the latest *captured* producer schema
//! (`materialized_asset_schema`, written post-materialize by #2a) and return
//! WARNINGS — never errors. A deliberate upstream reshape must not fail every
//! consumer save; blocking (`on_schema_change=fail`) is deliberately not
//! offered in v1.
//!
//! Ducklake-only: `// materialize` targets are ducklake-only in v1, so nothing
//! else has a captured schema to check against; an asset with no captured
//! schema produces no warnings (first deploy, datatable, external tables).
//!
//! The comparison itself (`diff_contract`) is pure so it can be unit-tested
//! and mirrored 1:1 by the editor-side TS check
//! (frontend/src/lib/components/assets/AssetGraph/schemaContracts.ts); the
//! async wrapper owns the DB reads (schemas + producer resolution) and runs on
//! the caller's RLS-scoped transaction.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};
use windmill_parser::asset_parser::{
    ColumnLineage, DataTest, Dimension, MaterializeSpec, Measure, OnSchemaChange, PARTITION_TOKEN,
};
use windmill_types::assets::{AssetKind, AssetWithAltAccessType};

use crate::error::Result;
use crate::materialization::SchemaColumn;

/// Columns the materialize engine adds/manages; never part of the captured
/// schema, so consumer reads of them must not warn (`_wm_partition` is
/// filtered out of the DESCRIBE capture on purpose).
const RESERVED_COLUMNS: &[&str] = &["_wm_partition"];

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractWarningKind {
    /// A column the body reads/writes is absent from the captured schema.
    MissingColumn,
    /// A `// column … <- <asset>.<col>` source column is absent.
    MissingLineageSource,
    /// A `// data_test relationships … -> <asset>.<col>` ref column is absent.
    MissingRelationshipColumn,
    /// Relationship join columns have different captured types (may still
    /// coerce at run time — phrased as "differs", not "will fail").
    RelationshipTypeMismatch,
    /// A `// measure` body reads a column absent from the producer's own
    /// captured (target) schema.
    MissingMeasureColumn,
    /// A `// dimension` body reads a column absent from the producer's own
    /// captured (target) schema.
    MissingDimensionColumn,
    /// A `// measure` body contains no aggregate (it is a row-level expression),
    /// so grouping it by a dimension produces an invalid query.
    NonAggregateMeasure,
    /// Warnings for this asset were suppressed by the producer's
    /// `on_schema_change=ignore` (one informational entry per asset).
    Suppressed,
}

/// One save-time contract warning. `schema_version`/`captured_at` identify the
/// capture the check ran against, so a stale-capture warning is
/// self-explaining (the schema is as-of the producer's last run, not its
/// latest save).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ContractWarning {
    pub kind: ContractWarningKind,
    /// Normalized ducklake asset path (`<lake>/<table>`).
    pub asset_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub found_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<DateTime<Utc>>,
    pub message: String,
}

/// The latest captured schema of one asset, as loaded by the wrapper.
#[derive(Debug, Clone)]
pub struct CapturedSchema {
    pub columns: Vec<SchemaColumn>,
    pub version: i64,
    pub captured_at: DateTime<Utc>,
}

impl CapturedSchema {
    /// Case-insensitive column lookup — DuckDB matches unquoted identifiers
    /// case-insensitively, and the body parser preserves source casing while
    /// DESCRIBE returns stored casing.
    fn find(&self, name: &str) -> Option<&SchemaColumn> {
        self.columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }
}

fn is_reserved(name: &str) -> bool {
    RESERVED_COLUMNS
        .iter()
        .any(|r| r.eq_ignore_ascii_case(name))
}

/// Strip the `{partition}` token a declared URI may carry (`// on
/// ducklake://lake/t/{partition}` or pasted refs) so lookups hit the captured
/// path. Body-inferred paths never carry it, but annotation refs can.
pub fn normalize_asset_path(path: &str) -> String {
    path.replace(&format!("/{}", PARTITION_TOKEN), "")
        .replace(PARTITION_TOKEN, "")
        .trim_end_matches('/')
        .to_string()
}

fn column_list(schema: &CapturedSchema) -> String {
    schema
        .columns
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Pure comparison: consumer refs vs captured schemas. `schemas` is keyed by
/// normalized ducklake path (the `_current` → base-table fallback is resolved
/// by the wrapper before this runs); `ignored` holds paths whose producer
/// declared `on_schema_change=ignore`.
pub fn diff_contract(
    assets: &[AssetWithAltAccessType],
    column_lineage: &[ColumnLineage],
    data_tests: &[DataTest],
    materialize: Option<&MaterializeSpec>,
    schemas: &HashMap<String, CapturedSchema>,
    ignored: &HashSet<String>,
) -> Vec<ContractWarning> {
    let mut warnings: Vec<ContractWarning> = vec![];

    // W1 — body-read/written columns missing from the captured schema. Assets
    // whose column set the parser could not derive (`columns: None`, e.g.
    // wildcard SELECT or non-SQL access) are skipped fail-safe; a literal "*"
    // key is skipped defensively for the same reason.
    for a in assets {
        if a.kind != AssetKind::Ducklake {
            continue;
        }
        let Some(columns) = a.columns.as_ref() else {
            continue;
        };
        let path = normalize_asset_path(&a.path);
        let Some(schema) = schemas.get(&path) else {
            continue;
        };
        for col in columns.keys() {
            if col == "*" || is_reserved(col) {
                continue;
            }
            if schema.find(col).is_none() {
                warnings.push(ContractWarning {
                    kind: ContractWarningKind::MissingColumn,
                    asset_path: path.clone(),
                    column: Some(col.clone()),
                    expected_type: None,
                    found_type: None,
                    schema_version: Some(schema.version),
                    captured_at: Some(schema.captured_at),
                    message: format!(
                        "column `{col}` of ducklake://{path} is not in its captured schema \
                         (v{}, columns: {})",
                        schema.version,
                        column_list(schema)
                    ),
                });
            }
        }
    }

    // W2 — `// column` lineage source refs. Only annotation-declared lineage
    // reaches this fn (AST-inferred lineage is redundant with W1 and can
    // mis-attribute aliases).
    for cl in column_lineage {
        for input in &cl.inputs {
            if input.from_kind != windmill_parser::asset_parser::AssetKind::Ducklake {
                continue;
            }
            let path = normalize_asset_path(&input.from_path);
            let Some(schema) = schemas.get(&path) else {
                continue;
            };
            if is_reserved(&input.from_column) {
                continue;
            }
            if schema.find(&input.from_column).is_none() {
                warnings.push(ContractWarning {
                    kind: ContractWarningKind::MissingLineageSource,
                    asset_path: path.clone(),
                    column: Some(input.from_column.clone()),
                    expected_type: None,
                    found_type: None,
                    schema_version: Some(schema.version),
                    captured_at: Some(schema.captured_at),
                    message: format!(
                        "`// column {}` reads `{}` from ducklake://{path}, which is not in \
                         its captured schema (v{})",
                        cl.column, input.from_column, schema.version
                    ),
                });
            }
        }
    }

    // W3 — relationships data-test refs: the referenced column must exist;
    // when both sides have captured types, flag a difference. Types come from
    // DuckDB DESCRIBE on both sides so verbatim spellings are comparable; the
    // runtime probe's IN-subquery still coerces, so a difference is "differs",
    // never "will fail".
    let own_schema = materialize
        .filter(|m| m.target_kind == windmill_parser::asset_parser::AssetKind::Ducklake)
        .and_then(|m| schemas.get(&normalize_asset_path(&m.target_path)));
    for dt in data_tests {
        let DataTest::Relationships { column, to_kind, to_path, to_column } = dt else {
            continue;
        };
        if *to_kind != windmill_parser::asset_parser::AssetKind::Ducklake {
            continue;
        }
        let path = normalize_asset_path(to_path);
        let Some(schema) = schemas.get(&path) else {
            continue;
        };
        match schema.find(to_column) {
            None => {
                warnings.push(ContractWarning {
                    kind: ContractWarningKind::MissingRelationshipColumn,
                    asset_path: path.clone(),
                    column: Some(to_column.clone()),
                    expected_type: None,
                    found_type: None,
                    schema_version: Some(schema.version),
                    captured_at: Some(schema.captured_at),
                    message: format!(
                        "`// data_test relationships {column}` references \
                         ducklake://{path}.{to_column}, which is not in its captured schema \
                         (v{})",
                        schema.version
                    ),
                });
            }
            Some(ref_col) => {
                if let Some(own_col) = own_schema.and_then(|s| s.find(column)) {
                    if !own_col.data_type.eq_ignore_ascii_case(&ref_col.data_type) {
                        warnings.push(ContractWarning {
                            kind: ContractWarningKind::RelationshipTypeMismatch,
                            asset_path: path.clone(),
                            column: Some(to_column.clone()),
                            expected_type: Some(own_col.data_type.clone()),
                            found_type: Some(ref_col.data_type.clone()),
                            schema_version: Some(schema.version),
                            captured_at: Some(schema.captured_at),
                            message: format!(
                                "`// data_test relationships {column}` joins `{}` ({}) to \
                                 ducklake://{path}.{to_column} ({}) — captured types differ",
                                column, own_col.data_type, ref_col.data_type
                            ),
                        });
                    }
                }
            }
        }
    }

    // W4 — producer opted the asset out (`on_schema_change=ignore`): drop its
    // warnings, leaving one informational entry per suppressed asset so the
    // response still records that a mismatch exists but was muted upstream.
    if !ignored.is_empty() {
        let mut suppressed_assets: Vec<String> = vec![];
        warnings.retain(|w| {
            if ignored.contains(&w.asset_path) {
                if !suppressed_assets.contains(&w.asset_path) {
                    suppressed_assets.push(w.asset_path.clone());
                }
                false
            } else {
                true
            }
        });
        for path in suppressed_assets {
            warnings.push(ContractWarning {
                kind: ContractWarningKind::Suppressed,
                asset_path: path.clone(),
                column: None,
                expected_type: None,
                found_type: None,
                schema_version: None,
                captured_at: None,
                message: format!(
                    "schema mismatches on ducklake://{path} suppressed by its producer's \
                     `on_schema_change=ignore`"
                ),
            });
        }
    }

    warnings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricRefKind {
    Measure,
    Dimension,
}

/// Warn about measures that contain no aggregate. A measure is meant to reduce
/// the table's rows, so grouping a row-level expression like `amount` by a
/// dimension yields an invalid query. Pure and schema-independent. A warning, not
/// an error: detection is deliberately lenient (any function call is accepted, to
/// avoid a reserved aggregate-name list), so a hard block would risk rejecting an
/// unusual-but-valid aggregate.
pub fn check_measures_aggregate(
    measures: &[Measure],
    materialize: Option<&MaterializeSpec>,
) -> Vec<ContractWarning> {
    let mut warnings = vec![];
    let target = materialize
        .filter(|m| m.target_kind == windmill_parser::asset_parser::AssetKind::Ducklake)
        .map(|m| normalize_asset_path(&m.target_path))
        .unwrap_or_default();
    for m in measures {
        if windmill_parser_sql_asset::measure_expr_may_aggregate(&m.expr) {
            continue;
        }
        warnings.push(ContractWarning {
            kind: ContractWarningKind::NonAggregateMeasure,
            asset_path: target.clone(),
            column: None,
            expected_type: None,
            found_type: None,
            schema_version: None,
            captured_at: None,
            message: format!(
                "`// measure {} = {}` contains no aggregate, so grouping it by a \
                 dimension produces an invalid query",
                m.name, m.expr
            ),
        });
    }
    warnings
}

/// A metric measure/dimension plus the column names its body reads, extracted by
/// the async wrapper (the pure diff never parses SQL). One per declared metric.
#[derive(Debug, Clone)]
pub struct MetricColumnRef {
    pub kind: MetricRefKind,
    pub name: String,
    pub columns: Vec<String>,
}

/// Validate metric measure/dimension bodies against the producer's own captured
/// (target) schema. Separate from `diff_contract` because these are the
/// producer's *own* declarations, not consumer reads of an upstream: a measure
/// citing a column the producer itself dropped is the producer's bug, and is
/// deliberately not muted by `on_schema_change=ignore` (which only governs
/// downstream drift). Pure — column refs are pre-extracted by the caller.
pub fn diff_metric_contract(
    metric_refs: &[MetricColumnRef],
    materialize: Option<&MaterializeSpec>,
    schemas: &HashMap<String, CapturedSchema>,
) -> Vec<ContractWarning> {
    let mut warnings: Vec<ContractWarning> = vec![];
    let Some(m) =
        materialize.filter(|m| m.target_kind == windmill_parser::asset_parser::AssetKind::Ducklake)
    else {
        return warnings;
    };
    let own_path = normalize_asset_path(&m.target_path);
    let Some(schema) = schemas.get(&own_path) else {
        return warnings;
    };
    for mr in metric_refs {
        for col in &mr.columns {
            if is_reserved(col) || schema.find(col).is_some() {
                continue;
            }
            let (kind, label) = match mr.kind {
                MetricRefKind::Measure => (ContractWarningKind::MissingMeasureColumn, "measure"),
                MetricRefKind::Dimension => {
                    (ContractWarningKind::MissingDimensionColumn, "dimension")
                }
            };
            warnings.push(ContractWarning {
                kind,
                asset_path: own_path.clone(),
                column: Some(col.clone()),
                expected_type: None,
                found_type: None,
                schema_version: Some(schema.version),
                captured_at: Some(schema.captured_at),
                message: format!(
                    "`// {label} {}` reads `{col}`, which is not in ducklake://{own_path}'s \
                     captured schema (v{})",
                    mr.name, schema.version
                ),
            });
        }
    }
    warnings
}

/// Load captured schemas + producer modes and run the contract check for one
/// consumer script's parsed refs.
///
/// Runs on the caller's RLS-scoped transaction (`user_db`), consistent with
/// the `listAssetSchemas` read path: a producer script the caller cannot read
/// simply stays unresolved and keeps the default `warn` behavior. Draft-only
/// producers have no `asset` write edges yet and likewise default to `warn`.
pub async fn check_schema_contracts(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    assets: &[AssetWithAltAccessType],
    column_lineage: &[ColumnLineage],
    data_tests: &[DataTest],
    materialize: Option<&MaterializeSpec>,
    measures: &[Measure],
    dimensions: &[Dimension],
) -> Result<Vec<ContractWarning>> {
    // Whether a measure aggregates is a pure property of its expression, so this
    // check runs regardless of whether a schema has been captured.
    let mut warnings = check_measures_aggregate(measures, materialize);

    // Referenced ducklake paths (normalized) across every ref family the diff
    // inspects — plus the consumer's own materialize target (for W3 types).
    let mut paths: HashSet<String> = HashSet::new();
    for a in assets {
        if a.kind == AssetKind::Ducklake && a.columns.is_some() {
            paths.insert(normalize_asset_path(&a.path));
        }
    }
    for cl in column_lineage {
        for input in &cl.inputs {
            if input.from_kind == windmill_parser::asset_parser::AssetKind::Ducklake {
                paths.insert(normalize_asset_path(&input.from_path));
            }
        }
    }
    for dt in data_tests {
        if let DataTest::Relationships { to_kind, to_path, .. } = dt {
            if *to_kind == windmill_parser::asset_parser::AssetKind::Ducklake {
                paths.insert(normalize_asset_path(to_path));
            }
        }
    }
    if let Some(m) = materialize {
        if m.target_kind == windmill_parser::asset_parser::AssetKind::Ducklake {
            paths.insert(normalize_asset_path(&m.target_path));
        }
    }
    if paths.is_empty() {
        // No captured schemas to diff against, but the schema-independent measure
        // checks above still stand.
        return Ok(warnings);
    }

    // A managed scd2 producer (re)creates a `<dim>_current` view with the base
    // table's columns; only the base table's schema is captured. Include the
    // base path in the lookup so `_current` readers can fall back to it (the
    // fallback itself is gated on the producer's spec below).
    let mut lookup_paths: HashSet<String> = paths.clone();
    for p in &paths {
        if let Some(base) = p.strip_suffix("_current") {
            if !base.is_empty() {
                lookup_paths.insert(base.to_string());
            }
        }
    }
    let lookup_vec: Vec<String> = lookup_paths.into_iter().collect();

    let schema_rows = sqlx::query!(
        r#"SELECT DISTINCT ON (asset_path)
                  asset_path, version, columns AS "columns: Json<Vec<SchemaColumn>>", captured_at
             FROM materialized_asset_schema
            WHERE workspace_id = $1 AND asset_kind = 'ducklake' AND asset_path = ANY($2)
            ORDER BY asset_path, version DESC"#,
        workspace_id,
        &lookup_vec,
    )
    .fetch_all(&mut **tx)
    .await?;
    let mut schemas: HashMap<String, CapturedSchema> = schema_rows
        .into_iter()
        .map(|r| {
            (
                r.asset_path,
                CapturedSchema {
                    columns: r.columns.0,
                    version: r.version,
                    captured_at: r.captured_at,
                },
            )
        })
        .collect();

    // Producer resolution: write edges on the referenced assets → latest
    // non-archived producer content → parsed `// materialize` spec. Drives
    // both the `on_schema_change=ignore` suppression and the `_current`
    // fallback. Flow writers are excluded — they cannot carry the annotation.
    let paths_vec: Vec<String> = paths.iter().cloned().collect();
    let producer_edges = sqlx::query!(
        r#"SELECT DISTINCT path AS "asset_path!", usage_path AS "producer_path!"
             FROM asset
            WHERE workspace_id = $1 AND kind = 'ducklake' AND path = ANY($2)
              AND usage_kind = 'script' AND usage_access_type IN ('w', 'rw')"#,
        workspace_id,
        &paths_vec,
    )
    .fetch_all(&mut **tx)
    .await?;

    let mut ignored: HashSet<String> = HashSet::new();
    if !producer_edges.is_empty() {
        let producer_paths: Vec<String> = producer_edges
            .iter()
            .map(|e| e.producer_path.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        // Same latest-content pattern as the asset-graph endpoint; NOT
        // `get_latest_script_hash`, whose `lock IS NOT NULL` filter transiently
        // excludes a just-deployed producer pending its dependency job.
        let producer_rows = sqlx::query!(
            r#"SELECT DISTINCT ON (path) path, content
                 FROM script
                WHERE workspace_id = $1 AND path = ANY($2)
                  AND archived = false AND deleted = false
                ORDER BY path, created_at DESC"#,
            workspace_id,
            &producer_paths,
        )
        .fetch_all(&mut **tx)
        .await?;
        let producer_specs: HashMap<String, Option<MaterializeSpec>> = producer_rows
            .into_iter()
            .map(|r| {
                (
                    r.path,
                    windmill_parser::asset_parser::parse_pipeline_annotations(&r.content)
                        .materialize,
                )
            })
            .collect();

        for edge in &producer_edges {
            let Some(Some(spec)) = producer_specs.get(&edge.producer_path) else {
                continue;
            };
            if spec.target_kind != windmill_parser::asset_parser::AssetKind::Ducklake {
                continue;
            }
            let target = normalize_asset_path(&spec.target_path);
            // `on_schema_change=ignore` — any producer declaring it wins.
            if spec.on_schema_change == OnSchemaChange::Ignore
                && (target == edge.asset_path
                    || (spec.scd2 && format!("{target}_current") == edge.asset_path))
            {
                ignored.insert(edge.asset_path.clone());
            }
            // `_current` fallback: a managed scd2 producer's view has exactly
            // the base table's columns.
            if spec.scd2
                && !spec.manual
                && format!("{target}_current") == edge.asset_path
                && !schemas.contains_key(&edge.asset_path)
            {
                if let Some(base) = schemas.get(&target).cloned() {
                    schemas.insert(edge.asset_path.clone(), base);
                }
            }
        }
    }

    // Extract the columns each measure/dimension body reads (measure filters
    // too), then validate them against the producer's own captured schema.
    let mut metric_refs: Vec<MetricColumnRef> =
        Vec::with_capacity(measures.len() + dimensions.len());
    for mm in measures {
        let mut columns = windmill_parser_sql_asset::extract_expr_column_idents(&mm.expr);
        if let Some(filter) = &mm.filter {
            columns.extend(windmill_parser_sql_asset::extract_expr_column_idents(
                filter,
            ));
        }
        metric_refs.push(MetricColumnRef {
            kind: MetricRefKind::Measure,
            name: mm.name.clone(),
            columns,
        });
    }
    for dd in dimensions {
        metric_refs.push(MetricColumnRef {
            kind: MetricRefKind::Dimension,
            name: dd.name.clone(),
            columns: windmill_parser_sql_asset::extract_expr_column_idents(&dd.expr),
        });
    }

    warnings.extend(diff_contract(
        assets,
        column_lineage,
        data_tests,
        materialize,
        &schemas,
        &ignored,
    ));
    warnings.extend(diff_metric_contract(&metric_refs, materialize, &schemas));
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use windmill_parser::asset_parser::{parse_pipeline_annotations, ColumnRef};
    use windmill_types::assets::AssetUsageAccessType;

    fn schema(cols: &[(&str, &str)]) -> CapturedSchema {
        CapturedSchema {
            columns: cols
                .iter()
                .map(|(n, t)| SchemaColumn { name: n.to_string(), data_type: t.to_string() })
                .collect(),
            version: 2,
            captured_at: DateTime::<Utc>::MIN_UTC,
        }
    }

    fn read_asset(path: &str, cols: &[&str]) -> AssetWithAltAccessType {
        AssetWithAltAccessType {
            path: path.to_string(),
            kind: AssetKind::Ducklake,
            access_type: Some(AssetUsageAccessType::R),
            alt_access_type: None,
            columns: Some(
                cols.iter()
                    .map(|c| (c.to_string(), AssetUsageAccessType::R))
                    .collect::<BTreeMap<_, _>>(),
            ),
        }
    }

    #[test]
    fn missing_read_column_warns_case_insensitively() {
        let schemas = HashMap::from([(
            "lake/orders".to_string(),
            schema(&[("Order_ID", "BIGINT"), ("amount_usd", "DOUBLE")]),
        )]);
        let assets = vec![read_asset("lake/orders", &["order_id", "amount"])];
        let w = diff_contract(&assets, &[], &[], None, &schemas, &HashSet::new());
        // order_id matches case-insensitively; amount is gone
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].kind, ContractWarningKind::MissingColumn);
        assert_eq!(w[0].column.as_deref(), Some("amount"));
        assert_eq!(w[0].schema_version, Some(2));
    }

    #[test]
    fn unknown_columns_wildcard_and_reserved_are_skipped() {
        let schemas = HashMap::from([("lake/orders".to_string(), schema(&[("id", "BIGINT")]))]);
        // columns: None (wildcard SELECT) — skipped entirely
        let mut a = read_asset("lake/orders", &[]);
        a.columns = None;
        assert!(diff_contract(&[a], &[], &[], None, &schemas, &HashSet::new()).is_empty());
        // literal "*" and the reserved partition column are skipped
        let a = read_asset("lake/orders", &["*", "_wm_partition", "id"]);
        assert!(diff_contract(&[a], &[], &[], None, &schemas, &HashSet::new()).is_empty());
    }

    #[test]
    fn metric_column_missing_from_own_schema_warns() {
        let ann = parse_pipeline_annotations("-- materialize ducklake://lake/orders\nSELECT 1;");
        let schemas = HashMap::from([(
            "lake/orders".to_string(),
            schema(&[("amount", "DOUBLE"), ("region", "VARCHAR")]),
        )]);
        let refs = vec![
            MetricColumnRef {
                kind: MetricRefKind::Measure,
                name: "revenue".to_string(),
                columns: vec!["amount".to_string()],
            },
            MetricColumnRef {
                kind: MetricRefKind::Measure,
                name: "refunds".to_string(),
                columns: vec!["refund_amt".to_string()],
            },
            MetricColumnRef {
                kind: MetricRefKind::Dimension,
                name: "zone".to_string(),
                columns: vec!["zone_id".to_string()],
            },
        ];
        let w = diff_metric_contract(&refs, ann.materialize.as_ref(), &schemas);
        // `amount` exists; the refund measure and zone dimension cite unknown
        // columns of the producer's own captured schema.
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].kind, ContractWarningKind::MissingMeasureColumn);
        assert_eq!(w[0].column.as_deref(), Some("refund_amt"));
        assert_eq!(w[1].kind, ContractWarningKind::MissingDimensionColumn);
        assert_eq!(w[1].column.as_deref(), Some("zone_id"));
        // No captured schema → silent (first deploy).
        assert!(diff_metric_contract(&refs, ann.materialize.as_ref(), &HashMap::new()).is_empty());
    }

    #[test]
    fn a_measure_with_no_aggregate_warns_schema_independently() {
        let ann = parse_pipeline_annotations(
            "-- materialize ducklake://sales/orders\n\
             -- measure revenue = amount\n\
             -- measure scaled = amount * 2\n\
             -- measure total = sum(amount)\n\
             -- measure n = count(*)\n\
             -- measure custom = my_udaf(x)\n\
             SELECT 1;",
        );
        // Runs without any captured schema. Bare column and pure arithmetic warn;
        // sum/count and an unknown function (benefit of the doubt, no aggregate-name
        // list) do not.
        let w = check_measures_aggregate(&ann.measures, ann.materialize.as_ref());
        assert_eq!(w.len(), 2);
        assert!(w
            .iter()
            .all(|w| w.kind == ContractWarningKind::NonAggregateMeasure));
        assert!(w[0].message.contains("revenue"));
        assert!(w[1].message.contains("scaled"));
    }

    #[test]
    fn asset_without_captured_schema_is_silent() {
        let assets = vec![read_asset("lake/unknown", &["whatever"])];
        assert!(
            diff_contract(&assets, &[], &[], None, &HashMap::new(), &HashSet::new()).is_empty()
        );
    }

    #[test]
    fn partition_token_is_normalized() {
        let schemas = HashMap::from([("lake/orders".to_string(), schema(&[("id", "BIGINT")]))]);
        let assets = vec![read_asset("lake/orders/{partition}", &["gone"])];
        let w = diff_contract(&assets, &[], &[], None, &schemas, &HashSet::new());
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].asset_path, "lake/orders");
    }

    #[test]
    fn lineage_ref_missing_column_warns() {
        let schemas = HashMap::from([("lake/orders".to_string(), schema(&[("id", "BIGINT")]))]);
        let lineage = vec![ColumnLineage {
            column: "total".to_string(),
            inputs: vec![ColumnRef {
                from_kind: windmill_parser::asset_parser::AssetKind::Ducklake,
                from_path: "lake/orders".to_string(),
                from_column: "amount".to_string(),
            }],
        }];
        let w = diff_contract(&[], &lineage, &[], None, &schemas, &HashSet::new());
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].kind, ContractWarningKind::MissingLineageSource);
    }

    #[test]
    fn relationships_missing_and_type_mismatch() {
        let schemas = HashMap::from([
            ("lake/customers".to_string(), schema(&[("id", "VARCHAR")])),
            (
                "lake/orders".to_string(),
                schema(&[("customer_id", "BIGINT")]),
            ),
        ]);
        let ann = parse_pipeline_annotations(
            "// materialize ducklake://lake/orders\n\
             // data_test relationships customer_id -> ducklake://lake/customers.id\n\
             // data_test relationships customer_id -> ducklake://lake/customers.uuid\n\
             SELECT 1;",
        );
        let w = diff_contract(
            &[],
            &[],
            &ann.data_tests,
            ann.materialize.as_ref(),
            &schemas,
            &HashSet::new(),
        );
        assert_eq!(w.len(), 2);
        assert!(w
            .iter()
            .any(|w| w.kind == ContractWarningKind::RelationshipTypeMismatch
                && w.expected_type.as_deref() == Some("BIGINT")
                && w.found_type.as_deref() == Some("VARCHAR")));
        assert!(w
            .iter()
            .any(|w| w.kind == ContractWarningKind::MissingRelationshipColumn
                && w.column.as_deref() == Some("uuid")));
    }

    #[test]
    fn ignored_asset_suppresses_to_single_note() {
        let schemas = HashMap::from([("lake/orders".to_string(), schema(&[("id", "BIGINT")]))]);
        let assets = vec![read_asset("lake/orders", &["a", "b"])];
        let ignored = HashSet::from(["lake/orders".to_string()]);
        let w = diff_contract(&assets, &[], &[], None, &schemas, &ignored);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].kind, ContractWarningKind::Suppressed);
        // and nothing at all when there was nothing to suppress
        let assets = vec![read_asset("lake/orders", &["id"])];
        assert!(diff_contract(&assets, &[], &[], None, &schemas, &ignored).is_empty());
    }
}
