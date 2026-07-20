//! Catalog of table-scoped metric declarations (`// measure`, `// dimension`).
//!
//! Declarations live in the producing script's annotation header and are mirrored
//! into the `data_metric` table on deploy, so a reader can ask "what does this
//! table declare?" or "what is declared under this folder?" without fetching and
//! parsing script bodies. Nothing here rewrites or executes SQL: the catalog is
//! read by the script editor and by agents, which compose their own queries.

use crate::error::{Error, Result};
use serde::Serialize;
use windmill_parser::asset_parser::{parse_pipeline_annotations, AssetKind};

pub const KIND_MEASURE: &str = "measure";
pub const KIND_DIMENSION: &str = "dimension";

// Mirror the `data_metric` column widths so oversized annotations fail at deploy
// with a clear message instead of a Postgres error.
const MAX_NAME_LEN: usize = 255;
const MAX_TABLE_PATH_LEN: usize = 510;

/// Canonical `<lake>/<schema>.<table>`, defaulting the schema to DuckLake's `main`.
///
/// A producer's `// materialize ducklake://lake/orders` target omits the schema
/// while a consumer reading `lake.main.orders` records its asset as
/// `lake/main.orders`. Both sides must land on the same catalog key or a consumer
/// can never resolve the table's declarations.
pub fn canonical_table_path(path: &str) -> String {
    let p = path.strip_prefix("ducklake://").unwrap_or(path);
    match p.split_once('/') {
        Some((lake, rest)) if !rest.contains('.') => format!("{lake}/main.{rest}"),
        _ => p.to_string(),
    }
}

/// Whether a canonical table path is safe to interpolate into an `ATTACH` string.
/// A real DuckLake path is dotted/slashed identifiers; anything else could break
/// out of the string literal or the statement.
pub fn is_safe_table_path(path: &str) -> bool {
    !path.is_empty()
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'))
}

/// One declared measure or dimension.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct MetricEntry {
    /// The declaring script, and the path reads are authorized against.
    pub script_path: String,
    /// Canonical scheme-less ducklake path, `<lake>/<schema>.<table>`.
    pub table_path: String,
    pub kind: String,
    pub name: String,
    pub expr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
}

/// Replace `script_path`'s catalog rows with the declarations currently in its
/// source, so the catalog always describes the deployed state.
///
/// Declarations describe the table the script materializes, so a script without a
/// ducklake `// materialize` target contributes nothing: there would be no table
/// to attach them to.
///
/// **Authorization is the caller's responsibility.** This writes workspace-scoped
/// rows for whatever `w_id`/`script_path` it is handed and performs no permission
/// check of its own. Call it only where the caller has already established write
/// access to that script path, as script deployment does.
pub async fn sync_metric_catalog(
    db: &mut sqlx::PgConnection,
    w_id: &str,
    script_path: &str,
    old_path: Option<&str>,
    content: &str,
) -> Result<()> {
    // Clear the old path too, so a rename does not orphan its declarations at a
    // path this script no longer occupies.
    sqlx::query!(
        "DELETE FROM data_metric WHERE workspace_id = $1 AND (script_path = $2 OR script_path = $3)",
        w_id,
        script_path,
        old_path.unwrap_or(script_path)
    )
    .execute(&mut *db)
    .await?;

    let ann = parse_pipeline_annotations(content);
    let Some(spec) = ann
        .materialize
        .filter(|m| m.target_kind == AssetKind::Ducklake)
    else {
        return Ok(());
    };
    if ann.measures.is_empty() && ann.dimensions.is_empty() {
        return Ok(());
    }

    // The lake/table name is interpolated into an `ATTACH 'ducklake://<lake>' …`
    // string that a *reader* executes, so a quote or semicolon in it is stored SQL
    // injection.
    let canonical = canonical_table_path(&spec.target_path);
    if !is_safe_table_path(&canonical) {
        return Err(Error::BadRequest(format!(
            "`// materialize` target `{}` has an unsafe table path `{canonical}`: a metric's \
             lake/table name may contain only letters, digits, `_ - . /`",
            spec.target_path
        )));
    }

    // Reject caller-authored values longer than their columns (VARCHAR counts
    // characters) with a clear error at deploy, not a Postgres "value too long"
    // failure. script_path is the script's own already-validated path.
    if canonical.chars().count() > MAX_TABLE_PATH_LEN {
        return Err(Error::BadRequest(format!(
            "`// materialize` target table path `{canonical}` is too long (max {MAX_TABLE_PATH_LEN} characters)"
        )));
    }
    for name in ann
        .measures
        .iter()
        .map(|m| &m.name)
        .chain(ann.dimensions.iter().map(|d| &d.name))
    {
        if name.chars().count() > MAX_NAME_LEN {
            return Err(Error::BadRequest(format!(
                "metric name `{name}` is too long (max {MAX_NAME_LEN} characters)"
            )));
        }
    }

    // Declarations are interpolated verbatim into SQL that a *reader* executes, so
    // each one must be a single expression. Otherwise `count(*) FROM t; DELETE …`
    // stored as a measure would run as whoever opens the drawer.
    for (kind, name, body) in ann
        .measures
        .iter()
        .flat_map(|m| {
            std::iter::once((KIND_MEASURE, &m.name, m.expr.as_str()))
                .chain(m.filter.as_deref().map(|f| (KIND_MEASURE, &m.name, f)))
        })
        .chain(
            ann.dimensions
                .iter()
                .map(|d| (KIND_DIMENSION, &d.name, d.expr.as_str())),
        )
    {
        if !windmill_parser_sql_asset::is_single_sql_expression(body) {
            return Err(Error::BadRequest(format!(
                "{kind} `{name}` must be a single SQL expression; `{body}` is not \
                 (it would be executed by anyone reading this table's metrics)"
            )));
        }
    }

    // A measure's trailing `where` compiles to `<expr> FILTER (WHERE <pred>)`, and
    // SQL binds FILTER to one aggregate call. A composite like `sum(a)/count(b)`
    // would filter only `count(b)`, silently corrupting the canonical number, so a
    // filtered measure must be a single aggregate call.
    for m in ann.measures.iter().filter(|m| m.filter.is_some()) {
        if !windmill_parser_sql_asset::is_single_function_call(&m.expr) {
            return Err(Error::BadRequest(format!(
                "measure `{}` has a `where` filter, so `{}` must be a single aggregate call \
                 (e.g. `sum(amount)`): a `where` on a composite like `sum(a)/count(b)` would \
                 filter only part of it and silently produce the wrong number",
                m.name, m.expr
            )));
        }
    }

    let mut kinds: Vec<String> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut exprs: Vec<String> = Vec::new();
    let mut filters: Vec<Option<String>> = Vec::new();
    for m in &ann.measures {
        kinds.push(KIND_MEASURE.to_string());
        names.push(m.name.clone());
        exprs.push(m.expr.clone());
        filters.push(m.filter.clone());
    }
    for d in &ann.dimensions {
        kinds.push(KIND_DIMENSION.to_string());
        names.push(d.name.clone());
        exprs.push(d.expr.clone());
        filters.push(None);
    }

    sqlx::query!(
        "INSERT INTO data_metric (workspace_id, script_path, table_path, kind, name, expr, filter) \
         SELECT $1, $2, $3, k, n, e, f \
         FROM UNNEST($4::text[], $5::text[], $6::text[], $7::text[]) AS t(k, n, e, f)",
        w_id,
        script_path,
        &canonical,
        &kinds[..],
        &names[..],
        &exprs[..],
        &filters[..] as &[Option<String>]
    )
    .execute(&mut *db)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declaration_smuggling_extra_statements_is_rejected() {
        use windmill_parser_sql_asset::is_single_sql_expression;
        // The shape that would otherwise execute as whoever opens the drawer.
        assert!(!is_single_sql_expression(
            "count(*) FROM t; DELETE FROM secrets; SELECT count(*)"
        ));
        assert!(!is_single_sql_expression("1; DROP TABLE t"));
        assert!(!is_single_sql_expression("sum(amount) garbage"));
        // A trailing comment is skipped by the parser, so it is rejected explicitly.
        assert!(!is_single_sql_expression("sum(amount) --rest"));
        assert!(!is_single_sql_expression("sum(amount) /* c */"));
        // Ordinary declarations still pass.
        assert!(is_single_sql_expression("sum(amount)"));
        assert!(is_single_sql_expression("count(*)"));
        assert!(is_single_sql_expression("not is_refund"));
        assert!(is_single_sql_expression("date_trunc('month', ordered_at)"));
    }

    #[test]
    fn a_filtered_measure_must_be_a_single_aggregate_call() {
        use windmill_parser_sql_asset::is_single_function_call;
        // A filtered composite would mis-apply FILTER to only one of its aggregates.
        assert!(!is_single_function_call("sum(amount) / count(*)"));
        assert!(!is_single_function_call("sum(a) + sum(b)"));
        assert!(!is_single_function_call("sum(amount) * 2"));
        // A single aggregate call is fine (FILTER attaches unambiguously).
        assert!(is_single_function_call("sum(amount)"));
        assert!(is_single_function_call("count(*)"));
        assert!(is_single_function_call("count(distinct user_id)"));
    }

    #[test]
    fn an_unsafe_lake_path_is_rejected() {
        // The injection shape: a quote/semicolon in the lake name would break out of
        // the ATTACH string literal.
        assert!(!is_safe_table_path("dl';SELECT(1);--/main.orders"));
        assert!(!is_safe_table_path("lake\"/main.t"));
        assert!(!is_safe_table_path(""));
        // Ordinary canonical paths are fine.
        assert!(is_safe_table_path("sales/main.orders"));
        assert!(is_safe_table_path("my-lake/analytics.daily_v2"));
    }

    #[test]
    fn a_producers_target_and_a_consumers_read_canonicalize_alike() {
        // What a producer declares, and what a consumer's read records.
        assert_eq!(
            canonical_table_path("ducklake://sales/orders"),
            canonical_table_path("sales/main.orders")
        );
        assert_eq!(canonical_table_path("sales/orders"), "sales/main.orders");
        // An explicit schema is preserved rather than forced to `main`.
        assert_eq!(
            canonical_table_path("sales/analytics.orders"),
            "sales/analytics.orders"
        );
    }
}
