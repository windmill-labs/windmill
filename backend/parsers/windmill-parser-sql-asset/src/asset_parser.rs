use std::collections::{BTreeMap, HashSet};

use sqlparser::{
    ast::{
        CopyTarget, Expr, FunctionArg, FunctionArgExpr, ObjectName, ObjectNamePart, SelectItem,
        TableFactor, TableObject, Value, ValueWithSpan, Visit, Visitor,
    },
    dialect::DuckDbDialect,
    parser::Parser,
};
use windmill_parser::asset_parser::{
    asset_was_used, merge_assets, merge_column_lineage, parse_asset_syntax,
    parse_pipeline_annotations, AssetKind, AssetUsageAccessType, ColumnLineage, ColumnRef,
    ParseAssetsOutput, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<ParseAssetsOutput> {
    let statements = Parser::parse_sql(&DuckDbDialect, input)?;

    let mut collector = AssetCollector::new();
    for statement in statements {
        let _ = statement.visit(&mut collector);
    }

    for (_, (kind, path)) in collector.var_identifiers {
        if !asset_was_used(&collector.assets, (kind, &path)) {
            collector.assets.push(ParseAssetsResult {
                kind,
                access_type: None,
                path: path,
                columns: None,
            });
        }
    }

    let mut pipeline = parse_pipeline_annotations(input);
    // Scope inferred lineage to a single output asset so columns from an
    // auxiliary CTAS aren't attributed to the materialized one (the flat list
    // has no per-entry output on the wire). The `// materialize` target, when
    // declared, IS the output: keep entries tagged with it plus untagged
    // top-level-SELECT entries (which describe that target). Without a declared
    // target, keep inference only when every tagged entry shares one output
    // asset — otherwise it's ambiguous which asset the flat list describes, so
    // drop it rather than show false dependencies.
    let target = pipeline
        .materialize
        .as_ref()
        .map(|m| (m.target_kind, m.target_path.clone()));
    let inferred: Vec<ColumnLineage> = match &target {
        Some(t) => collector
            .column_lineage
            .into_iter()
            .filter(|(out, _)| out.as_ref().map_or(true, |o| o == t))
            .map(|(_, cl)| cl)
            .collect(),
        None => {
            let mut first: Option<&(AssetKind, String)> = None;
            let mut ambiguous = false;
            for (out, _) in &collector.column_lineage {
                if let Some(o) = out {
                    match first {
                        None => first = Some(o),
                        Some(f) if f != o => {
                            ambiguous = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            if ambiguous {
                Vec::new()
            } else {
                collector
                    .column_lineage
                    .into_iter()
                    .map(|(_, cl)| cl)
                    .collect()
            }
        }
    };
    // Body-inferred column lineage, with `// column` annotations taking
    // precedence per output column (explicit declaration overrides inference).
    pipeline.column_lineage = merge_column_lineage(inferred, pipeline.column_lineage);
    // A bare string literal in query position is only weak read evidence: a
    // summary `SELECT 's3:///out.csv' AS target` after `COPY … TO
    // 's3:///out.csv'` must not turn the write into rw (which draws a
    // spurious read edge and an asset⇄script cycle in the pipeline graph).
    // Surface weak reads only for assets with no other recorded usage, so a
    // path that is *merely* mentioned still shows up linked to the script.
    let mut assets = merge_assets(collector.assets);
    for weak in merge_assets(collector.weak_string_reads) {
        if !assets
            .iter()
            .any(|a| a.kind == weak.kind && a.path == weak.path)
        {
            assets.push(weak);
        }
    }
    assets.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ParseAssetsOutput::new(assets, Vec::new(), pipeline))
}

/// Provenance of the innermost access context. The access type alone can't
/// tell a definitive read apart from a mere mention: a bare string literal in
/// generic query position (`QueryRead`) is only *weak* read evidence — e.g. a
/// summary `SELECT 's3:///out.csv' AS target` echoing a path — while the same
/// literal as a read-function argument or a `COPY` target is definitive.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AccessCtx {
    QueryRead,
    ReadFn,
    CopyWrite,
}

impl AccessCtx {
    fn access_type(self) -> AssetUsageAccessType {
        match self {
            AccessCtx::QueryRead | AccessCtx::ReadFn => R,
            AccessCtx::CopyWrite => W,
        }
    }
}

/// Visitor that collects S3 asset literals from SQL statements
struct AssetCollector {
    assets: Vec<ParseAssetsResult>,
    // Bare string literals seen in generic query position — weak read
    // evidence, surfaced by `parse_assets` only when the script has no other
    // recorded usage of the same asset (a real write must not gain a spurious
    // read edge from a mention).
    weak_string_reads: Vec<ParseAssetsResult>,
    // e.g set to QueryRead when we are inside a SELECT ... FROM ... statement
    current_access_type_stack: Vec<AccessCtx>,
    // e.g ATTACH 'ducklake://a' AS dl; => { "dl": (Ducklake, "a") }
    var_identifiers: BTreeMap<String, (AssetKind, String)>,
    // e.g USE dl;
    currently_used_asset: Option<(AssetKind, String)>,
    // CTE names in scope (stack for nested queries)
    cte_name_stack: Vec<HashSet<String>>,
    // Locally created tables (not attached to an asset)
    local_table_names: HashSet<String>,
    // Inferred column-level lineage: one entry per output column of an
    // output-producing query, mapping it to the upstream source columns its
    // expression reads. Each is tagged with the *output asset* it belongs to —
    // `Some((kind, path))` for a CTAS / CREATE VIEW into a real asset, `None`
    // for a top-level managed-materialize SELECT (its output is the `//
    // materialize` target, known only in `parse_assets`). `parse_assets` uses
    // the tag to scope the flat list to a single output asset so columns from an
    // auxiliary output don't get attributed to the materialized one. Best-effort:
    // dynamic/raw SQL, INSERT…SELECT, and wildcards are left to annotations.
    column_lineage: Vec<(Option<(AssetKind, String)>, ColumnLineage)>,
}

impl AssetCollector {
    fn new() -> Self {
        Self {
            assets: Vec::new(),
            weak_string_reads: Vec::new(),
            current_access_type_stack: Vec::with_capacity(8),
            var_identifiers: BTreeMap::new(),
            currently_used_asset: None,
            cte_name_stack: Vec::new(),
            local_table_names: HashSet::new(),
            column_lineage: Vec::new(),
        }
    }

    /// Record a `CREATE TABLE`/`VIEW` target. A *temporary* table/view is always
    /// local — even a one-part name under an active `USE dl`, which would
    /// otherwise resolve to an asset (`ducklake://…/tmp`) and then leak as a
    /// column source for later references. A non-temp name that resolves to an
    /// attached asset is recorded as that asset; anything else is registered
    /// local so subsequent references aren't attributed to the active asset.
    fn track_table_definition(&mut self, name: &ObjectName, is_temporary: bool) {
        if !is_temporary {
            if let Some(asset) = self.get_associated_asset_from_obj_name(name, Some(W)) {
                self.assets.push(asset);
                return;
            }
        }
        if let Some(simple_name) = get_trivial_obj_name(name) {
            self.local_table_names.insert(simple_name.to_lowercase());
        }
    }

    fn is_locally_defined(&self, name: &str) -> bool {
        let name_lower = name.to_lowercase();
        self.local_table_names.contains(&name_lower)
            || self
                .cte_name_stack
                .iter()
                .any(|set| set.contains(&name_lower))
    }

    // Detect when we do 'a.b' and 'a' is associated with an asset in var_identifiers
    // Or when we access 'b' and we did USE a;
    fn get_associated_asset_from_obj_name(
        &self,
        name: &ObjectName,
        access_type: Option<AssetUsageAccessType>,
    ) -> Option<ParseAssetsResult> {
        let access_type = access_type.or_else(|| {
            self.current_access_type_stack
                .last()
                .map(|c| c.access_type())
        });
        if let Some((kind, path)) = &self.currently_used_asset {
            // We don't want to infer that any simple identifier refers to an asset if
            // we are not in a known R/W context
            if access_type.is_none() {
                return None;
            }

            if name.0.len() == 1 {
                if let Some(ident) = name.0.first().and_then(|id| id.as_ident()) {
                    if self.is_locally_defined(&ident.value) {
                        return None;
                    }
                }
            }

            if name.0.len() == 1 || name.0.len() == 2 {
                if name
                    .0
                    .iter()
                    .any(|id| id.as_ident().and_then(|id| id.quote_style).is_some())
                {
                    return None;
                }

                let specific_table = &name
                    .0
                    .iter()
                    .map(|id| id.as_ident().map(|id| id.value.clone()))
                    .collect::<Option<Vec<String>>>()?
                    .join(".");

                // For Resource assets, use ?table= query parameter syntax
                // For Ducklake and DataTable, maintain /table syntax
                let path = if *kind == AssetKind::Resource {
                    format!("{}?table={}", path, specific_table)
                } else {
                    format!("{}/{}", path, specific_table)
                };
                return Some(ParseAssetsResult { kind: *kind, access_type, path, columns: None });
            }
        }

        // Check if the first part of the name (the a in a.b) is associated with an asset
        if name.0.len() < 2 {
            return None;
        }
        let ident = name.0.first()?.as_ident()?;
        let (kind, path) = self.var_identifiers.get(&ident.value)?;
        let path = if name.0.len() == 2 || name.0.len() == 3 {
            let specific_table = &name.0[1..]
                .iter()
                .map(|id| id.as_ident().map(|id| id.value.clone()))
                .collect::<Option<Vec<String>>>()?
                .join(".");

            // For Resource assets, use ?table= query parameter syntax
            // For Ducklake and DataTable, maintain /table syntax
            if *kind == AssetKind::Resource {
                format!("{}?table={}", path, specific_table)
            } else {
                format!("{}/{}", path, specific_table)
            }
        } else {
            path.clone()
        };
        Some(ParseAssetsResult { kind: *kind, access_type, path, columns: None })
    }

    /// If `table_factor` is a string literal used directly as a table name (e.g. FROM 's3:///file.parquet'),
    /// return a `ParseAssetsResult` for it.
    fn get_s3_asset_from_str_literal_table(
        &self,
        table_factor: &TableFactor,
    ) -> Option<ParseAssetsResult> {
        let name = match table_factor {
            TableFactor::Table { name, args: None, .. } => name,
            _ => return None,
        };

        let s3_str = get_str_lit_from_obj_name(name)?;
        let (kind, path) = parse_asset_syntax(s3_str, false)?;
        if kind != AssetKind::S3Object {
            return None;
        }

        Some(ParseAssetsResult {
            kind,
            path: path.to_string(),
            access_type: Some(R),
            columns: None,
        })
    }

    /// If `table_factor` is a read function (read_parquet/read_csv/read_json) whose first
    /// positional argument is an S3 string literal, return a `ParseAssetsResult` for it.
    fn get_s3_asset_from_table_function(
        &self,
        table_factor: &TableFactor,
    ) -> Option<ParseAssetsResult> {
        let (name, args) = match table_factor {
            TableFactor::Table { name, args: Some(args), .. } => (name, args),
            _ => return None,
        };

        let fname = get_trivial_obj_name(name)?;
        if !is_read_fn(fname) {
            return None;
        }

        let s3_str = args.args.first().and_then(|arg| match arg {
            FunctionArg::Unnamed(FunctionArgExpr::Expr(Expr::Value(ValueWithSpan {
                value: Value::SingleQuotedString(s),
                ..
            })))
            | FunctionArg::Unnamed(FunctionArgExpr::Expr(Expr::Value(ValueWithSpan {
                value: Value::DoubleQuotedString(s),
                ..
            }))) => Some(s.as_str()),
            _ => None,
        })?;

        let (kind, path) = parse_asset_syntax(s3_str, false)?;
        if kind != AssetKind::S3Object {
            return None;
        }

        Some(ParseAssetsResult {
            kind,
            path: path.to_string(),
            access_type: Some(R),
            columns: None,
        })
    }

    fn handle_string_literal(&mut self, s: &str) {
        // Check if the string matches our asset syntax patterns
        if let Some((kind, path)) = parse_asset_syntax(s, false) {
            if kind == AssetKind::S3Object {
                let ctx = self.current_access_type_stack.last().copied();
                let result = ParseAssetsResult {
                    kind,
                    path: path.to_string(),
                    access_type: ctx.map(AccessCtx::access_type),
                    columns: None,
                };
                if ctx == Some(AccessCtx::QueryRead) {
                    self.weak_string_reads.push(result);
                } else {
                    self.assets.push(result);
                }
            }
        }
    }

    fn handle_obj_name_pre(&mut self, name: &ObjectName) {
        if let Some(fname) = get_trivial_obj_name(name) {
            if is_read_fn(fname) {
                self.current_access_type_stack.push(AccessCtx::ReadFn);
            }
        }
        if let Some(str_lit) = get_str_lit_from_obj_name(name) {
            self.handle_string_literal(str_lit);
        }
    }

    fn handle_obj_name_post(&mut self, name: &ObjectName) {
        if self.current_access_type_stack.is_empty() {
            return;
        }
        if let Some(fname) = get_trivial_obj_name(name) {
            if is_read_fn(fname) {
                self.current_access_type_stack.pop();
            }
        }
    }

    // Collect the table-level reads (and column assets) of a query's top
    // SELECT. Table-level reads are only gathered here and in the statement
    // arms — the generic table-factor visitor picks up read-functions and
    // string literals, not plain `FROM <table>` references. Called for both
    // standalone SELECTs and the `AS SELECT` of CTAS / CREATE VIEW.
    fn handle_query_reads(&mut self, query: &sqlparser::ast::Query) {
        self.cte_name_stack.push(collect_cte_names(query));
        if let Some(select) = query.body.as_select() {
            for t in &select.from {
                self.handle_table_with_joins(t, Some(R));
            }
            self.extract_column_assets(&select.projection, &select.from);
        }
    }

    // Infer the output-column lineage of a query that produces an asset, tagging
    // each entry with its `output` asset. Called only for an output-producing
    // query — a top-level managed-materialize SELECT (`output: None`, resolved
    // to the `// materialize` target later) or a CTAS / CREATE VIEW into a real
    // asset (`output: Some`). A CTAS into a local/temp staging table is never
    // an output, so it's simply not passed here.
    fn infer_query_output(
        &mut self,
        query: &sqlparser::ast::Query,
        output: Option<(AssetKind, String)>,
    ) {
        if let Some(select) = query.body.as_select() {
            self.infer_column_lineage(&select.projection, &select.from, output);
        }
    }

    fn handle_table_with_joins(
        &mut self,
        table_with_joins: &sqlparser::ast::TableWithJoins,
        access_type: Option<AssetUsageAccessType>,
    ) {
        if let TableFactor::Table { name, args, .. } = &table_with_joins.relation {
            if args.is_some() && args.as_ref().map_or(0, |a| a.args.len()) > 0 {
                return;
            }
            if let Some(asset) = self.get_associated_asset_from_obj_name(name, access_type) {
                self.assets.push(asset);
            }
        }
        for join in &table_with_joins.joins {
            if let TableFactor::Table { name, .. } = &join.relation {
                if let Some(asset) = self.get_associated_asset_from_obj_name(name, access_type) {
                    self.assets.push(asset);
                }
            }
        }
    }

    // The alias-map entry (key → asset) for one FROM/JOIN table factor, or
    // `None` if it isn't an asset-backed table. The key is its alias, else the
    // bare table name; S3 table-functions and string-literal tables are only
    // keyed when aliased (an unaliased one is ambiguous). Returns the asset with
    // a single matched relation so the caller can attribute qualified columns.
    fn table_alias_entry(&self, relation: &TableFactor) -> Option<(String, ParseAssetsResult)> {
        let TableFactor::Table { name, alias, args, .. } = relation else {
            return None;
        };
        let has_args = args.as_ref().map_or(false, |a| !a.args.is_empty());
        if has_args {
            let alias = alias.as_ref()?;
            let asset = self.get_s3_asset_from_table_function(relation)?;
            return Some((alias.name.value.clone(), asset));
        }
        let asset = self
            .get_associated_asset_from_obj_name(name, Some(R))
            .or_else(|| self.get_s3_asset_from_str_literal_table(relation))?;
        if get_str_lit_from_obj_name(name).is_some() {
            // String-literal S3 table: only unambiguous when aliased.
            let alias = alias.as_ref()?;
            return Some((alias.name.value.clone(), asset));
        }
        let key = match alias {
            Some(a) => a.name.value.clone(),
            None => name
                .0
                .last()
                .and_then(|id| id.as_ident())
                .map(|id| id.value.clone())
                .unwrap_or_default(),
        };
        Some((key, asset))
    }

    // Resolve a query's FROM clause into (single-table asset, alias→asset map).
    // `single_table` is `Some` only for an unambiguous one-table FROM with no
    // joins (so bare column refs can be attributed); `table_to_asset` keys by
    // alias/table name for qualified refs and includes every JOINed table.
    // Shared by `extract_column_assets` (read columns) and `infer_column_lineage`
    // (output→input edges) so both resolve identically.
    fn build_from_maps(
        &self,
        from_tables: &[sqlparser::ast::TableWithJoins],
    ) -> (
        Option<ParseAssetsResult>,
        BTreeMap<String, ParseAssetsResult>,
    ) {
        // Single unambiguous table only when there's exactly one FROM entry AND
        // it has no joins — otherwise a bare column could belong to any side.
        let single_table = if from_tables.len() == 1 && from_tables[0].joins.is_empty() {
            let relation = &from_tables[0].relation;
            if let TableFactor::Table { name, args, .. } = relation {
                let has_args = args.as_ref().map_or(false, |a| !a.args.is_empty());
                if has_args {
                    self.get_s3_asset_from_table_function(relation)
                } else {
                    self.get_associated_asset_from_obj_name(name, Some(R))
                        .or_else(|| self.get_s3_asset_from_str_literal_table(relation))
                }
            } else {
                None
            }
        } else {
            None
        };

        // Alias → asset for qualified column refs, across every FROM entry AND
        // its JOINed tables (so `c.col` in `FROM a JOIN c` resolves).
        let mut table_to_asset: BTreeMap<String, ParseAssetsResult> = BTreeMap::new();
        for table_with_joins in from_tables {
            if let Some((k, a)) = self.table_alias_entry(&table_with_joins.relation) {
                table_to_asset.insert(k, a);
            }
            for join in &table_with_joins.joins {
                if let Some((k, a)) = self.table_alias_entry(&join.relation) {
                    table_to_asset.insert(k, a);
                }
            }
        }
        (single_table, table_to_asset)
    }

    // Extract columns from SELECT items and create individual asset results for each column
    // Only processes columns that reference known assets to avoid false positives
    fn extract_column_assets(
        &mut self,
        projection: &[SelectItem],
        from_tables: &[sqlparser::ast::TableWithJoins],
    ) {
        let (single_table, table_to_asset) = self.build_from_maps(from_tables);

        // Process each SELECT item
        for item in projection {
            match item {
                SelectItem::UnnamedExpr(Expr::Identifier(ident))
                | SelectItem::ExprWithAlias { expr: Expr::Identifier(ident), .. } => {
                    // Simple column: SELECT a
                    // Only add if we have a single table (unambiguous)
                    if let Some(asset) = &single_table {
                        let mut columns = BTreeMap::new();
                        columns.insert(ident.value.clone(), R);
                        self.assets.push(ParseAssetsResult {
                            kind: asset.kind,
                            path: asset.path.clone(),
                            access_type: Some(R),
                            columns: Some(columns),
                        });
                    }
                }
                SelectItem::UnnamedExpr(Expr::CompoundIdentifier(parts))
                | SelectItem::ExprWithAlias { expr: Expr::CompoundIdentifier(parts), .. } => {
                    // Qualified column: SELECT table1.a or SELECT x.table1.a
                    if parts.len() >= 2 {
                        let column_name = parts.last().map(|id| id.value.clone());

                        if let Some(column_name) = column_name {
                            // Check if the prefix matches a known table
                            let table_prefix = parts.first().map(|id| id.value.clone());

                            if let Some(table_prefix) = table_prefix {
                                if let Some(asset) = table_to_asset.get(&table_prefix) {
                                    // Found a matching table, add column asset
                                    let mut columns = BTreeMap::new();
                                    columns.insert(column_name.clone(), R);
                                    self.assets.push(ParseAssetsResult {
                                        kind: asset.kind,
                                        path: asset.path.clone(),
                                        access_type: Some(R),
                                        columns: Some(columns),
                                    });
                                } else if parts.len() >= 3 {
                                    // Could be x.table1.column format or db.schema.table.column
                                    // Convert Idents to ObjectNameParts
                                    let obj_parts: Vec<ObjectNamePart> = parts[..parts.len() - 1]
                                        .iter()
                                        .cloned()
                                        .map(|ident| ObjectNamePart::Identifier(ident))
                                        .collect();
                                    let obj_name = ObjectName(obj_parts);
                                    if let Some(asset) =
                                        self.get_associated_asset_from_obj_name(&obj_name, Some(R))
                                    {
                                        let mut columns = BTreeMap::new();
                                        columns.insert(column_name.clone(), R);
                                        self.assets.push(ParseAssetsResult {
                                            kind: asset.kind,
                                            path: asset.path.clone(),
                                            access_type: Some(R),
                                            columns: Some(columns),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Ignore wildcards, expressions, etc.
                }
            }
        }
    }

    // Infer column-level lineage for an output-producing query's projection:
    // each *named* output column → the upstream source columns its expression
    // reads. Covers passthroughs (`amount`) and computed columns
    // (`amount + tax AS total`) alike. Skipped: wildcards and unaliased
    // expressions (no stable output name), and inputs that don't resolve to a
    // known asset. A column with no resolved inputs is dropped.
    //
    // Best-effort and intentionally flat: results from every output query in the
    // script accumulate into one list (the graph hangs them off the materialize
    // write-edge), with no per-output-table association. This is exact for the
    // common single-output member (a managed-materialize SELECT, or one CTAS),
    // but a multi-statement script that stages through a TEMP table reports the
    // *intermediate* column names (the final SELECT reads the temp table, whose
    // columns don't resolve to an asset, so they drop out). A `// column`
    // annotation overrides any output column where inference is wrong or coarse.
    fn infer_column_lineage(
        &mut self,
        projection: &[SelectItem],
        from_tables: &[sqlparser::ast::TableWithJoins],
        output: Option<(AssetKind, String)>,
    ) {
        let (single_table, table_to_asset) = self.build_from_maps(from_tables);
        for item in projection {
            let (out_col, expr) = match item {
                SelectItem::ExprWithAlias { expr, alias } => (alias.value.clone(), expr),
                SelectItem::UnnamedExpr(expr @ Expr::Identifier(id)) => (id.value.clone(), expr),
                SelectItem::UnnamedExpr(expr @ Expr::CompoundIdentifier(parts)) => {
                    match parts.last() {
                        Some(last) => (last.value.clone(), expr),
                        None => continue,
                    }
                }
                _ => continue,
            };
            let mut collector = ColumnIdentCollector { refs: Vec::new(), query_depth: 0 };
            let _ = expr.visit(&mut collector);
            let mut inputs: Vec<ColumnRef> = Vec::new();
            for parts in &collector.refs {
                if let Some(cr) = self.resolve_column_ref(parts, &single_table, &table_to_asset) {
                    if !inputs.contains(&cr) {
                        inputs.push(cr);
                    }
                }
            }
            if !inputs.is_empty() {
                self.column_lineage
                    .push((output.clone(), ColumnLineage { column: out_col, inputs }));
            }
        }
    }

    // Resolve identifier `parts` (e.g. `["t","amount"]` or `["amount"]`) to the
    // source asset column it reads, mirroring `extract_column_assets`'
    // resolution: a bare ident needs an unambiguous single-table FROM; a
    // qualified ident resolves its prefix via the alias map, or (≥3 parts) as a
    // db/schema-qualified object name.
    fn resolve_column_ref(
        &self,
        parts: &[String],
        single_table: &Option<ParseAssetsResult>,
        table_to_asset: &BTreeMap<String, ParseAssetsResult>,
    ) -> Option<ColumnRef> {
        let asset_to_ref = |asset: &ParseAssetsResult, col: &str| ColumnRef {
            from_kind: asset.kind,
            from_path: asset.path.clone(),
            from_column: col.to_string(),
        };
        match parts {
            [col] => single_table.as_ref().map(|a| asset_to_ref(a, col)),
            [.., col] => {
                let prefix = parts.first()?;
                if let Some(asset) = table_to_asset.get(prefix) {
                    Some(asset_to_ref(asset, col))
                } else if parts.len() >= 3 {
                    let obj_parts: Vec<ObjectNamePart> = parts[..parts.len() - 1]
                        .iter()
                        .map(|p| ObjectNamePart::Identifier(sqlparser::ast::Ident::new(p.clone())))
                        .collect();
                    let asset =
                        self.get_associated_asset_from_obj_name(&ObjectName(obj_parts), Some(R))?;
                    Some(asset_to_ref(&asset, col))
                } else {
                    None
                }
            }
            [] => None,
        }
    }
}

// Collects the identifier paths an expression reads, for column-lineage
// inference: `Expr::Identifier(a)` → `["a"]`, `Expr::CompoundIdentifier(t.a)` →
// `["t","a"]`. The derived `Visit` walk recurses through operators, functions,
// casts and CASE, so every leaf identifier of the outer expression is captured.
struct ColumnIdentCollector {
    refs: Vec<Vec<String>>,
    // Depth of nested (sub)queries inside the expression. Identifiers are only
    // captured at depth 0: a scalar/correlated subquery's columns belong to ITS
    // own FROM, not the outer projection's, so descending would misattribute
    // (e.g. `(SELECT x FROM other) AS c FROM orders` must NOT bind `c` to
    // `orders.x`). Subquery-derived columns are simply left to annotations.
    query_depth: usize,
}

impl Visitor for ColumnIdentCollector {
    type Break = ();

    fn pre_visit_query(&mut self, _query: &sqlparser::ast::Query) -> std::ops::ControlFlow<()> {
        self.query_depth += 1;
        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_query(&mut self, _query: &sqlparser::ast::Query) -> std::ops::ControlFlow<()> {
        self.query_depth = self.query_depth.saturating_sub(1);
        std::ops::ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> std::ops::ControlFlow<Self::Break> {
        if self.query_depth == 0 {
            match expr {
                Expr::Identifier(id) => self.refs.push(vec![id.value.clone()]),
                Expr::CompoundIdentifier(parts) => self
                    .refs
                    .push(parts.iter().map(|id| id.value.clone()).collect()),
                _ => {}
            }
        }
        std::ops::ControlFlow::Continue(())
    }
}

impl Visitor for AssetCollector {
    type Break = ();

    fn pre_visit_table_factor(
        &mut self,
        table_factor: &TableFactor,
    ) -> std::ops::ControlFlow<Self::Break> {
        match table_factor {
            TableFactor::Table { name, args, .. } => {
                if args.is_none() {
                    // FROM 's3:///…' is a definitive read — record it directly
                    // so it isn't demoted to a weak in-query mention.
                    if let Some(asset) = self.get_s3_asset_from_str_literal_table(table_factor) {
                        self.assets.push(asset);
                    }
                }
                // For a read-function table factor this pushes ReadFn, making
                // every literal inside its arguments a definitive read — the
                // direct form (read_csv('s3:///…')) but also list and named
                // arguments (read_parquet(['s3:///…'])). Must run for BOTH the
                // plain-table and table-function branches: post_visit_table_factor
                // pops via handle_obj_name_post unconditionally, so skipping the
                // push here would unbalance the stack.
                self.handle_obj_name_pre(name);
            }
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_table_factor(
        &mut self,
        table_factor: &TableFactor,
    ) -> std::ops::ControlFlow<Self::Break> {
        match table_factor {
            TableFactor::Table { name, .. } => self.handle_obj_name_post(name),
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> std::ops::ControlFlow<Self::Break> {
        match expr {
            Expr::Value(ValueWithSpan { value: Value::SingleQuotedString(s), .. }) => {
                self.handle_string_literal(s)
            }
            Expr::Value(ValueWithSpan { value: Value::DoubleQuotedString(s), .. }) => {
                self.handle_string_literal(s);
            }
            // Read-function call in expression position: its argument literals
            // are definitive reads. Balances the pop in `post_visit_expr`.
            Expr::Function(func) => {
                if get_trivial_obj_name(&func.name).is_some_and(is_read_fn) {
                    self.current_access_type_stack.push(AccessCtx::ReadFn);
                }
            }
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_expr(&mut self, expr: &Expr) -> std::ops::ControlFlow<Self::Break> {
        match expr {
            Expr::Function(func) => self.handle_obj_name_post(&func.name),
            _ => {}
        }
        std::ops::ControlFlow::Continue(())
    }

    fn pre_visit_statement(
        &mut self,
        statement: &sqlparser::ast::Statement,
    ) -> std::ops::ControlFlow<Self::Break> {
        match statement {
            sqlparser::ast::Statement::Query(q) => {
                // A top-level SELECT is the managed-materialize output, so its
                // columns ARE the materialized asset's columns (output resolved
                // to the `// materialize` target in `parse_assets`).
                self.handle_query_reads(q);
                self.infer_query_output(q, None);
            }

            sqlparser::ast::Statement::Insert(insert) => {
                let access_type = if insert.returning.is_some() { RW } else { W };
                match insert.table {
                    TableObject::TableName(ref name) => {
                        if let Some(asset) =
                            self.get_associated_asset_from_obj_name(name, Some(access_type))
                        {
                            // Add table-level asset
                            self.assets.push(ParseAssetsResult {
                                kind: asset.kind,
                                path: asset.path.clone(),
                                access_type: asset.access_type,
                                columns: None,
                            });

                            // Extract column information for INSERT with explicit columns (Write access)
                            if !insert.columns.is_empty() {
                                for col in &insert.columns {
                                    let columns = BTreeMap::from([(col.value.clone(), W)]);
                                    self.assets.push(ParseAssetsResult {
                                        kind: asset.kind,
                                        path: asset.path.clone(),
                                        access_type: Some(W),
                                        columns: Some(columns),
                                    });
                                }
                            }

                            // Extract column information from RETURNING clause (Read access)
                            if let Some(returning) = &insert.returning {
                                for item in returning {
                                    match item {
                                        SelectItem::UnnamedExpr(Expr::Identifier(ident))
                                        | SelectItem::ExprWithAlias {
                                            expr: Expr::Identifier(ident),
                                            ..
                                        } => {
                                            let mut col_map = BTreeMap::new();
                                            col_map.insert(ident.value.clone(), R);
                                            self.assets.push(ParseAssetsResult {
                                                kind: asset.kind,
                                                path: asset.path.clone(),
                                                access_type: Some(R),
                                                columns: Some(col_map),
                                            });
                                        }
                                        _ => {
                                            // Ignore wildcards and complex expressions
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            sqlparser::ast::Statement::Update { returning, table, from, assignments, .. } => {
                if let Some(from_tables) = from {
                    let from_tables = match from_tables {
                        sqlparser::ast::UpdateTableFromKind::AfterSet(tables) => tables,
                        sqlparser::ast::UpdateTableFromKind::BeforeSet(tables) => tables,
                    };
                    for table_with_joins in from_tables {
                        self.handle_table_with_joins(table_with_joins, Some(R));
                    }
                }

                let access_type = if returning.is_some() { RW } else { W };
                self.handle_table_with_joins(table, Some(access_type));

                // Extract column information from UPDATE SET clauses (Write access)
                // Only process if it's a single table update
                if let TableFactor::Table { name, .. } = &table.relation {
                    if let Some(asset) =
                        self.get_associated_asset_from_obj_name(name, Some(access_type))
                    {
                        // Process each assignment to extract column names
                        for assignment in assignments {
                            // assignment.target is an AssignmentTarget enum
                            // We only handle simple column names (ColumnName variant)
                            if let sqlparser::ast::AssignmentTarget::ColumnName(col_name) =
                                &assignment.target
                            {
                                // For simple column updates, this is typically a single ident
                                if col_name.0.len() == 1 {
                                    if let Some(col_ident) =
                                        col_name.0.first().and_then(|p| p.as_ident())
                                    {
                                        let mut col_map = BTreeMap::new();
                                        col_map.insert(col_ident.value.clone(), W);
                                        self.assets.push(ParseAssetsResult {
                                            kind: asset.kind,
                                            path: asset.path.clone(),
                                            access_type: Some(W),
                                            columns: Some(col_map),
                                        });
                                    }
                                }
                            }
                        }

                        // Extract column information from RETURNING clause (Read access)
                        if let Some(returning_items) = returning {
                            for item in returning_items {
                                match item {
                                    SelectItem::UnnamedExpr(Expr::Identifier(ident))
                                    | SelectItem::ExprWithAlias {
                                        expr: Expr::Identifier(ident),
                                        ..
                                    } => {
                                        let mut col_map = BTreeMap::new();
                                        col_map.insert(ident.value.clone(), R);
                                        self.assets.push(ParseAssetsResult {
                                            kind: asset.kind,
                                            path: asset.path.clone(),
                                            access_type: Some(R),
                                            columns: Some(col_map),
                                        });
                                    }
                                    _ => {
                                        // Ignore wildcards and complex expressions
                                    }
                                }
                            }
                        }
                    }
                }
            }

            sqlparser::ast::Statement::Delete(delete) => {
                let access_type = if delete.returning.is_some() { RW } else { W };
                for name in &delete.tables {
                    if let Some(asset) =
                        self.get_associated_asset_from_obj_name(name, Some(access_type))
                    {
                        self.assets.push(asset);
                    }
                }
                let tables = match &delete.from {
                    sqlparser::ast::FromTable::WithFromKeyword(tables) => tables,
                    sqlparser::ast::FromTable::WithoutKeyword(tables) => tables,
                };
                for table_with_joins in tables {
                    self.handle_table_with_joins(table_with_joins, Some(access_type));
                }
            }

            sqlparser::ast::Statement::CreateTable(create_table) => {
                self.track_table_definition(&create_table.name, create_table.temporary);
                // `CREATE TABLE x AS SELECT … FROM y` reads y. The AS-query
                // isn't a `Statement::Query`, so its FROM tables are only
                // caught here. Only infer output lineage when `x` is a real
                // asset — a CTAS into a local/temp staging table is not the
                // materialized output (its columns aren't the asset's).
                if let Some(query) = &create_table.query {
                    self.handle_query_reads(query);
                    // Infer only when `x` is a real asset (its output columns
                    // ARE that asset's), tagged with it so `parse_assets` can
                    // scope lineage per output. A local/temp staging table is
                    // not an asset → not inferred.
                    if let Some(asset) =
                        self.get_associated_asset_from_obj_name(&create_table.name, Some(W))
                    {
                        self.infer_query_output(query, Some((asset.kind, asset.path)));
                    }
                }
            }

            sqlparser::ast::Statement::CreateView { name, query, temporary, .. } => {
                self.track_table_definition(name, *temporary);
                self.handle_query_reads(query);
                if let Some(asset) = self.get_associated_asset_from_obj_name(name, Some(W)) {
                    self.infer_query_output(query, Some((asset.kind, asset.path)));
                }
            }

            // DROP TABLE/VIEW is a write to the dropped object — the
            // canonical idempotent-refresh pattern (`DROP TABLE IF EXISTS x;
            // CREATE TABLE x AS …`) must resolve to a table-level write.
            // Without this arm, a tagged-template snippet containing only the
            // DROP yields no table-level asset and the TS/Python SDK parsers
            // fall back to a db-level `datatable://<db>` reference, putting a
            // stray database node on the pipeline canvas.
            sqlparser::ast::Statement::Drop { object_type, names, .. } => {
                if matches!(
                    object_type,
                    sqlparser::ast::ObjectType::Table
                        | sqlparser::ast::ObjectType::View
                        | sqlparser::ast::ObjectType::MaterializedView
                ) {
                    for name in names {
                        // DROP is a write to the named object; resolve it as an
                        // asset if it is one (not a temp-creation context).
                        self.track_table_definition(name, false);
                    }
                }
            }

            sqlparser::ast::Statement::Copy { target: CopyTarget::File { filename }, .. } => {
                self.current_access_type_stack.push(AccessCtx::CopyWrite);
                self.handle_string_literal(filename);
                self.current_access_type_stack.pop();
            }

            sqlparser::ast::Statement::AttachDuckDBDatabase {
                database_path,
                database_alias,
                ..
            } => {
                if let Some((kind, path)) = parse_asset_syntax(&database_path.value, true) {
                    if kind == AssetKind::Ducklake
                        || kind == AssetKind::DataTable
                        || kind == AssetKind::Resource
                    {
                        if let Some(database_alias) = database_alias {
                            self.var_identifiers
                                .insert(database_alias.value.clone(), (kind, path.to_string()));
                        }
                    }
                }
            }

            sqlparser::ast::Statement::DetachDuckDBDatabase { database_alias, .. } => {
                let asset = self.var_identifiers.remove(&database_alias.value);
                if self.currently_used_asset == asset {
                    self.currently_used_asset = None;
                }
            }

            sqlparser::ast::Statement::Use(sqlparser::ast::Use::Object(obj_name)) => {
                if let Some((kind, path)) = self.var_identifiers.get(&obj_name.to_string()) {
                    self.currently_used_asset = Some((*kind, path.clone()));
                } else {
                    self.currently_used_asset = None;
                }
            }

            _ => {}
        }

        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_statement(
        &mut self,
        statement: &sqlparser::ast::Statement,
    ) -> std::ops::ControlFlow<Self::Break> {
        // Balance the push done by handle_query_reads (called from the Query,
        // CreateView, and CTAS arms).
        let pushed = match statement {
            sqlparser::ast::Statement::Query(_) | sqlparser::ast::Statement::CreateView { .. } => {
                true
            }
            sqlparser::ast::Statement::CreateTable(ct) => ct.query.is_some(),
            _ => false,
        };
        if pushed {
            self.cte_name_stack.pop();
        }
        std::ops::ControlFlow::Continue(())
    }

    fn pre_visit_query(
        &mut self,
        query: &sqlparser::ast::Query,
    ) -> std::ops::ControlFlow<Self::Break> {
        self.current_access_type_stack.push(AccessCtx::QueryRead);
        self.cte_name_stack.push(collect_cte_names(query));
        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_query(
        &mut self,
        _query: &sqlparser::ast::Query,
    ) -> std::ops::ControlFlow<Self::Break> {
        self.current_access_type_stack.pop();
        self.cte_name_stack.pop();
        std::ops::ControlFlow::Continue(())
    }

    // We do not use pre_visit_relation because we cannot know if an ObjectName is a table or a function
}

fn collect_cte_names(query: &sqlparser::ast::Query) -> HashSet<String> {
    query.with.as_ref().map_or_else(HashSet::new, |with| {
        with.cte_tables
            .iter()
            .map(|cte| cte.alias.name.value.to_lowercase())
            .collect()
    })
}

fn is_read_fn(fname: &str) -> bool {
    fname.eq_ignore_ascii_case("read_parquet")
        || fname.eq_ignore_ascii_case("read_csv")
        || fname.eq_ignore_ascii_case("read_json")
}

fn get_trivial_obj_name(name: &sqlparser::ast::ObjectName) -> Option<&str> {
    if name.0.len() != 1 {
        return None;
    }
    Some(name.0.first()?.as_ident()?.value.as_str())
}

fn get_str_lit_from_obj_name(name: &ObjectName) -> Option<&str> {
    if name.0.len() != 1 {
        return None;
    }
    let ident = name.0.first()?.as_ident()?;
    if ident.quote_style != Some('\'') {
        return None;
    }
    Some(ident.value.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sql_asset_parser_s3_literals() {
        let input = r#"
            SELECT * FROM read_parquet('s3:///a.parquet');
            COPY (SELECT * FROM 's3://snd/b.parquet') TO 's3:///c.parquet';
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/a.parquet".to_string(),
                    access_type: Some(R),
                    columns: None
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/c.parquet".to_string(),
                    access_type: Some(W),
                    columns: None
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "snd/b.parquet".to_string(),
                    access_type: Some(R),
                    columns: None
                },
            ])
        );
    }

    #[test]
    fn test_duckdb_read_key_matches_sdk_write_key() {
        // Cross-language lineage: a TS `writeS3File({ s3: "exports/x" })` or
        // Python `write_s3_file(S3Object(s3="exports/x"))` records the asset
        // path `/exports/x` (default storage, leading slash). A DuckDB reader
        // of the same object uses the triple-slash default-storage URI and
        // must resolve to the identical path so the graph connects producer
        // and consumer. The bare `s3://exports/x` form names storage
        // `exports` instead — a different object, a different path.
        let input = "SELECT * FROM read_csv('s3:///exports/x');";
        let assets = parse_assets(input).expect("parse").assets;
        assert_eq!(
            assets,
            vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/exports/x".to_string(),
                access_type: Some(R),
                columns: None
            }],
        );

        let input = "SELECT * FROM read_csv('s3://exports/x');";
        let assets = parse_assets(input).expect("parse").assets;
        assert_eq!(
            assets,
            vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "exports/x".to_string(),
                access_type: Some(R),
                columns: None
            }],
        );
    }

    #[test]
    fn test_copy_target_echoed_in_select_stays_write_only() {
        // The trailing summary SELECT merely mentions the COPY target — it
        // must not add a read (rw would draw an asset⇄script cycle).
        let input = r#"
            COPY (SELECT 1 AS x) TO 's3:///out.csv';
            SELECT 's3:///out.csv' AS target, 42 AS rows_written;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/out.csv".to_string(),
                access_type: Some(W),
                columns: None
            }])
        );
    }

    #[test]
    fn test_bare_string_mention_without_other_usage_is_a_read() {
        let input = r#"
            SELECT 's3:///referenced.csv' AS path;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/referenced.csv".to_string(),
                access_type: Some(R),
                columns: None
            }])
        );
    }

    #[test]
    fn test_self_refresh_read_fn_plus_copy_stays_rw() {
        // A definitive read (read_csv) of the same file the script rewrites
        // is a real rw — only *bare-literal* mentions are demoted.
        let input = r#"
            CREATE TABLE tmp AS SELECT * FROM read_csv('s3:///data.csv');
            COPY (SELECT * FROM tmp) TO 's3:///data.csv';
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/data.csv".to_string(),
                access_type: Some(RW),
                columns: None
            }])
        );
    }

    #[test]
    fn test_read_fn_list_arg_plus_copy_stays_rw() {
        // read_parquet's list form is as definitive as the direct literal —
        // it must not be demoted to a weak mention when the file is rewritten.
        let input = r#"
            CREATE TABLE tmp AS SELECT * FROM read_parquet(['s3:///data.parquet']);
            COPY (SELECT * FROM tmp) TO 's3:///data.parquet';
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/data.parquet".to_string(),
                access_type: Some(RW),
                columns: None
            }])
        );
    }

    #[test]
    fn test_read_fn_list_arg_multiple_files_are_reads() {
        let input = r#"
            SELECT * FROM read_parquet(['s3:///a.parquet', 's3:///b.parquet']);
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/a.parquet".to_string(),
                    access_type: Some(R),
                    columns: None
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/b.parquet".to_string(),
                    access_type: Some(R),
                    columns: None
                }
            ])
        );
    }

    #[test]
    fn test_from_string_literal_of_written_file_stays_rw() {
        // FROM-position string literal is likewise a definitive read.
        let input = r#"
            CREATE TABLE tmp AS SELECT * FROM 's3:///data.parquet';
            COPY (SELECT * FROM tmp) TO 's3:///data.parquet';
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/data.parquet".to_string(),
                access_type: Some(RW),
                columns: None
            }])
        );
    }

    #[test]
    fn test_sql_asset_parser_attach_no_usage_is_registered_as_unknown() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT 2;
            USE dl;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl".to_string(),
                access_type: None,
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_attach_dot_notation_read() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT * FROM dl.table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/table1".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_attach_dot_notation_write() {
        let input = r#"
            ATTACH 'datatable://my_dt' AS dt;
            SELECT dt.read_bait FROM unrelated_table; -- dt. doesn't access the asset
            INSERT INTO dt.table1 VALUES ('test');
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "my_dt/table1".to_string(),
                access_type: Some(W),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_detach() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            DETACH dl;
            SELECT * FROM dl.table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(s.map_err(|e| e.to_string()), Ok(vec![]));
    }

    #[test]
    fn test_sql_asset_parser_implicit_use_asset() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            INSERT INTO table1 VALUES ('test');
            USE memory;
            SELECT * FROM table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/table1".to_string(),
                access_type: Some(W),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_default_main() {
        let input = r#"
            ATTACH 'datatable' AS dl;
            INSERT INTO dl.table1 VALUES ('test');
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "main/table1".to_string(),
                access_type: Some(W),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_create_table() {
        let input = r#"
            ATTACH 'ducklake' AS dl; USE dl;
            CREATE TABLE friends (
                name text,
                age int
            );
            INSERT INTO friends VALUES ($name, $age);
            SELECT * FROM friends;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/friends".to_string(),
                access_type: Some(RW),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_drop_table_is_write() {
        // A lone DROP (e.g. one SDK tagged-template snippet of an
        // idempotent-refresh script) must resolve to a table-level write,
        // not fall through to nothing.
        let input = r#"
            ATTACH 'datatable://main' AS dt; USE dt;
            DROP TABLE IF EXISTS orders_raw;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "main/orders_raw".to_string(),
                access_type: Some(W),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_drop_then_create_qualified() {
        // Canonical refresh pattern over an attached catalog: DROP + CREATE
        // AS on the output, SELECT on the input.
        let input = r#"
            ATTACH 'datatable://main' AS pg;
            DROP TABLE IF EXISTS pg.daily_revenue;
            CREATE TABLE pg.daily_revenue AS SELECT * FROM pg.orders_clean;
        "#;
        let s = parse_assets(input).map(|mut s| {
            s.assets.sort_by(|a, b| a.path.cmp(&b.path));
            s.assets
        });
        // The DROP+CTAS combo yields a clean write of the output plus the
        // read of the CTAS source (`SELECT * FROM pg.orders_clean`).
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "main/daily_revenue".to_string(),
                    access_type: Some(W),
                    columns: None
                },
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "main/orders_clean".to_string(),
                    access_type: Some(R),
                    columns: None
                },
            ])
        );
    }

    // Make sure a_function is not detected as main/a_function
    #[test]
    fn test_sql_asset_parser_function_table() {
        let input = r#"
            ATTACH 'ducklake' AS dl; USE dl;
            SELECT * FROM a_function('');
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main".to_string(),
                access_type: None,
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_delete() {
        let input = r#"
            ATTACH 'ducklake' AS dl;
            USE dl;
            DELETE FROM table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/table1".to_string(),
                access_type: Some(W),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_update() {
        let input = r#"
            ATTACH 'ducklake' AS dl;
            USE dl;
            UPDATE table1 SET id = NULL;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/table1".to_string(),
                access_type: Some(W),
                columns: Some(BTreeMap::from([("id".to_string(), W)])),
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_resource() {
        let input = r#"
            ATTACH 'res://u/user/pg_resource' AS db (TYPE postgres);
            USE db;
            SELECT * FROM table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Resource,
                path: "u/user/pg_resource?table=table1".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_update_with_dot_notation() {
        let input = r#"
            ATTACH 'ducklake' AS dl;
            UPDATE dl.table1 SET id = NULL;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/table1".to_string(),
                access_type: Some(W),
                columns: Some(BTreeMap::from([("id".to_string(), W)])),
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_resource_vs_ducklake_syntax() {
        // Test that Resource uses ?table= while Ducklake uses /table
        let input_resource = r#"
            ATTACH 'res://u/user/pg_resource' AS db (TYPE postgres);
            SELECT * FROM db.users;
        "#;
        let s = parse_assets(input_resource).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Resource,
                path: "u/user/pg_resource?table=users".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );

        let input_ducklake = r#"
            ATTACH 'ducklake://my_lake' AS dl;
            SELECT * FROM dl.users;
        "#;
        let s = parse_assets(input_ducklake).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_lake/users".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );

        let input_datatable = r#"
            ATTACH 'datatable://dt1' AS dt;
            SELECT * FROM dt.users;
        "#;
        let s = parse_assets(input_datatable).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "dt1/users".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_resource_with_long_path() {
        // Test that Resource works with paths longer than 3 components
        let input = r#"
            ATTACH 'res://u/diego/a/b/c/my_postgres_resource' AS db (TYPE postgres);
            USE db;
            SELECT * FROM my_table;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Resource,
                path: "u/diego/a/b/c/my_postgres_resource?table=my_table".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_table_with_schema() {
        let input = r#"
            ATTACH 'ducklake' AS dl;
            UPDATE dl.sch.table1 SET id = NULL;
            SELECT * FROM dl.sch.table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/sch.table1".to_string(),
                access_type: Some(RW),
                columns: Some(BTreeMap::from([("id".to_string(), W)])),
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_table_with_schema_implicit() {
        let input = r#"
            ATTACH 'ducklake' AS dl;
            USE dl;
            UPDATE sch.table1 SET id = NULL;
            SELECT * FROM sch.table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/sch.table1".to_string(),
                access_type: Some(RW),
                columns: Some(BTreeMap::from([("id".to_string(), W)])),
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_single_table_column_detection() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT a, b FROM dl.table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Should have one asset with merged columns
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "my_dl/table1");
        assert_eq!(result[0].access_type, Some(R));

        // Check that both columns are present in the merged asset
        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.len(), 2);
        assert_eq!(columns.get("a"), Some(&R));
        assert_eq!(columns.get("b"), Some(&R));
    }

    #[test]
    fn test_sql_asset_parser_explicit_table_prefix_columns() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT dl.table1.a, dl.table1.b FROM dl.table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        // Should detect columns with explicit table prefix
        let result = s.unwrap();
        // Check we have the table asset

        // Check we have column assets
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("a"))
        }));
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("b"))
        }));
    }

    #[test]
    fn test_sql_asset_parser_multi_table_no_simple_columns() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT a, b FROM dl.table1, dl.table2;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        // Simple columns (a, b) should NOT be detected with multiple tables
        // Only table-level assets should be present
        let result = s.unwrap();

        // Should have 2 table assets
        assert_eq!(result.iter().filter(|a| a.columns.is_none()).count(), 2);

        // Should have NO column assets (ambiguous which table they belong to)
        assert_eq!(result.iter().filter(|a| a.columns.is_some()).count(), 0);
    }

    #[test]
    fn test_sql_asset_parser_multi_table_with_qualified_columns() {
        let input = r#"
            ATTACH 'ducklake://my_dl1' AS dl1;
            ATTACH 'ducklake://my_dl2' AS dl2;
            SELECT table1.a, table2.b FROM dl1.table1, dl2.table2;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        // Qualified columns should be detected even with multiple tables
        let result = s.unwrap();

        // Check we have column assets for both tables
        assert!(result.iter().any(|a| {
            a.path == "my_dl1/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("a"))
        }));
        assert!(result.iter().any(|a| {
            a.path == "my_dl2/table2"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("b"))
        }));
    }

    #[test]
    fn test_sql_asset_parser_use_with_simple_columns() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            SELECT a, b, c FROM table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Should detect columns since it's a single table
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("a"))
        }));
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("b"))
        }));
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("c"))
        }));
    }

    #[test]
    fn test_sql_asset_parser_wildcard_no_columns() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT * FROM dl.table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Wildcard should NOT create column assets, only table asset
        assert_eq!(result.len(), 1);
        assert!(result[0].columns.is_none());
    }

    #[test]
    fn test_sql_asset_parser_columns_with_alias() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT a AS column_a, b AS column_b FROM dl.table1;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Should detect columns even when aliased
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("a"))
        }));
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("b"))
        }));
    }

    #[test]
    fn test_sql_asset_parser_columns_with_table_alias() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT t.a, t.b FROM dl.table1 AS t;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Should detect columns using the table alias
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("a"))
        }));
        assert!(result.iter().any(|a| {
            a.path == "my_dl/table1"
                && a.columns
                    .as_ref()
                    .map_or(false, |cols| cols.contains_key("b"))
        }));
    }

    #[test]
    fn test_sql_asset_parser_insert_with_columns() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            INSERT INTO dl.table1 (name, age, email) VALUES ('John', 30, 'john@example.com');
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Should have one asset with merged columns
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "my_dl/table1");
        assert_eq!(result[0].access_type, Some(W));

        // Check that all columns are present in the merged asset
        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.len(), 3);
        assert_eq!(columns.get("name"), Some(&W));
        assert_eq!(columns.get("age"), Some(&W));
        assert_eq!(columns.get("email"), Some(&W));
    }

    #[test]
    fn test_sql_asset_parser_insert_without_columns() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            INSERT INTO dl.table1 VALUES ('John', 30);
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Should have one asset without column information
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "my_dl/table1");
        assert_eq!(result[0].access_type, Some(W));
        assert!(result[0].columns.is_none());
    }

    #[test]
    fn test_sql_asset_parser_update_multiple_columns() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            UPDATE dl.table1 SET name = 'Jane', age = 25, active = true;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Should have one asset with merged columns
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "my_dl/table1");
        assert_eq!(result[0].access_type, Some(W));

        // Check that all columns are present
        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.len(), 3);
        assert_eq!(columns.get("name"), Some(&W));
        assert_eq!(columns.get("age"), Some(&W));
        assert_eq!(columns.get("active"), Some(&W));
    }

    #[test]
    fn test_sql_asset_parser_update_returning() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            UPDATE dl.table1 SET name = 'Jane', age = 26 RETURNING id, name;
        "#;
        let s = parse_assets(input).map(|s| s.assets);

        let result = s.unwrap();

        // Should have RW access type when RETURNING is used
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "my_dl/table1");
        assert_eq!(result[0].access_type, Some(RW));

        // Check that columns are present with correct access types
        // name and age are written (W), id and name are read (R)
        // name should be RW (both written and read)
        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.len(), 3);
        assert_eq!(columns.get("name"), Some(&RW)); // Written in SET, read in RETURNING
        assert_eq!(columns.get("age"), Some(&W)); // Only written
        assert_eq!(columns.get("id"), Some(&R)); // Only read
    }

    #[test]
    fn test_sql_asset_parser_s3_single_table_column_detection() {
        let input = r#"
            SELECT col1, col2 FROM read_parquet('s3:///example_file.parquet');
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, AssetKind::S3Object);
        assert_eq!(result[0].path, "/example_file.parquet");
        assert_eq!(result[0].access_type, Some(R));

        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.len(), 2);
        assert_eq!(columns.get("col1"), Some(&R));
        assert_eq!(columns.get("col2"), Some(&R));
    }

    #[test]
    fn test_sql_asset_parser_s3_single_table_column_with_alias() {
        let input = r#"
            SELECT col1 AS c1, col2 AS c2 FROM read_parquet('s3:///example_file.parquet');
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 1);
        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.get("col1"), Some(&R));
        assert_eq!(columns.get("col2"), Some(&R));
    }

    #[test]
    fn test_sql_asset_parser_s3_wildcard_no_columns() {
        let input = r#"
            SELECT * FROM read_parquet('s3:///example_file.parquet');
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, AssetKind::S3Object);
        assert_eq!(result[0].path, "/example_file.parquet");
        assert!(result[0].columns.is_none());
    }

    #[test]
    fn test_sql_asset_parser_s3_table_alias_qualified_columns() {
        let input = r#"
            SELECT t.col1, t.col2 FROM read_parquet('s3:///example_file.parquet') AS t;
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, AssetKind::S3Object);
        assert_eq!(result[0].path, "/example_file.parquet");

        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.get("col1"), Some(&R));
        assert_eq!(columns.get("col2"), Some(&R));
    }

    #[test]
    fn test_sql_asset_parser_s3_multi_table_aliased_columns() {
        let input = r#"
            SELECT t1.col1, t2.col2
            FROM read_parquet('s3:///file1.parquet') AS t1,
                 read_csv('s3://bucket/file2.csv') AS t2;
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 2);

        assert!(result.iter().any(|a| {
            a.path == "/file1.parquet"
                && a.columns.as_ref().map_or(false, |c| c.contains_key("col1"))
        }));
        assert!(result.iter().any(|a| {
            a.path == "bucket/file2.csv"
                && a.columns.as_ref().map_or(false, |c| c.contains_key("col2"))
        }));
    }

    #[test]
    fn test_sql_asset_parser_s3_multi_table_no_alias_no_columns() {
        // Without aliases, unqualified columns in a multi-table query are ambiguous
        let input = r#"
            SELECT col1, col2
            FROM read_parquet('s3:///file1.parquet'),
                 read_parquet('s3:///file2.parquet');
        "#;
        let result = parse_assets(input).unwrap().assets;

        // Table-level assets should still be detected, but no columns
        assert_eq!(result.iter().filter(|a| a.columns.is_some()).count(), 0);
    }

    #[test]
    fn test_sql_asset_parser_s3_str_literal_table_column_detection() {
        // FROM 's3:///file.parquet' (string literal as table, no read_parquet wrapper)
        let input = r#"
            SELECT a,b,c FROM 's3:///test.parquet';
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, AssetKind::S3Object);
        assert_eq!(result[0].path, "/test.parquet");
        assert_eq!(result[0].access_type, Some(R));

        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.len(), 3);
        assert_eq!(columns.get("a"), Some(&R));
        assert_eq!(columns.get("b"), Some(&R));
        assert_eq!(columns.get("c"), Some(&R));
    }

    #[test]
    fn test_sql_asset_parser_s3_str_literal_table_with_alias_columns() {
        let input = r#"
            SELECT t.col1, t.col2 FROM 's3://bucket/file.parquet' AS t;
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, AssetKind::S3Object);
        assert_eq!(result[0].path, "bucket/file.parquet");

        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.get("col1"), Some(&R));
        assert_eq!(columns.get("col2"), Some(&R));
    }

    #[test]
    fn test_sql_asset_parser_s3_str_literal_wildcard_no_columns() {
        let input = r#"
            SELECT * FROM 's3:///test.parquet';
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, AssetKind::S3Object);
        assert!(result[0].columns.is_none());
    }

    #[test]
    fn test_sql_asset_parser_cte_not_treated_as_asset() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            WITH tmp AS (SELECT 1 AS x)
            SELECT * FROM tmp;
            SELECT * FROM real_table;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/real_table".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_cte_scope_does_not_leak() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            WITH tmp AS (SELECT 1) SELECT * FROM tmp;
            SELECT * FROM tmp;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/tmp".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_multiple_ctes() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            WITH cte1 AS (SELECT 1), cte2 AS (SELECT 2)
            SELECT * FROM cte1 JOIN cte2 ON true;
            SELECT * FROM real_table;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/real_table".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_local_create_table_overrides_asset() {
        let input = r#"
            CREATE TABLE local_tbl (id INT);
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            SELECT * FROM local_tbl;
            SELECT * FROM asset_table;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/asset_table".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_create_table_with_use_is_still_asset() {
        let input = r#"
            ATTACH 'ducklake' AS dl; USE dl;
            CREATE TABLE friends (
                name text,
                age int
            );
            INSERT INTO friends VALUES ($name, $age);
            SELECT * FROM friends;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/friends".to_string(),
                access_type: Some(RW),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_local_create_view_overrides_asset() {
        let input = r#"
            CREATE VIEW my_view AS SELECT 1;
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            SELECT * FROM my_view;
            SELECT * FROM asset_table;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/asset_table".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_create_view_with_use_is_still_asset() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            CREATE VIEW my_view AS SELECT 1;
            SELECT * FROM my_view;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/my_view".to_string(),
                access_type: Some(RW),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_cte_mixed_with_asset_tables() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            WITH tmp AS (SELECT 1 AS x)
            SELECT * FROM tmp JOIN real_table ON true;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/real_table".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_local_table_insert_and_select() {
        let input = r#"
            CREATE TABLE staging (id INT, val TEXT);
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            INSERT INTO staging VALUES (1, 'a');
            SELECT * FROM staging;
            INSERT INTO real_table VALUES (2, 'b');
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/real_table".to_string(),
                access_type: Some(W),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_qualified_ref_bypasses_local() {
        // Even if 'tbl' is local, 'dl.tbl' is an explicit asset reference
        let input = r#"
            CREATE TABLE tbl (id INT);
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT * FROM dl.tbl;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/tbl".to_string(),
                access_type: Some(R),
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_cte_case_insensitive() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            USE dl;
            WITH MyTable AS (SELECT 1)
            SELECT * FROM mytable;
        "#;
        let s = parse_assets(input).map(|s| s.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl".to_string(),
                access_type: None,
                columns: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_s3_read_csv_columns() {
        let input = r#"
            SELECT name, age FROM read_csv('s3://my-bucket/data.csv');
        "#;
        let result = parse_assets(input).unwrap().assets;

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, AssetKind::S3Object);
        assert_eq!(result[0].path, "my-bucket/data.csv");

        let columns = result[0].columns.as_ref().expect("Should have columns");
        assert_eq!(columns.get("name"), Some(&R));
        assert_eq!(columns.get("age"), Some(&R));
    }
}

#[cfg(test)]
mod ctas_read_tests {
    use super::*;

    #[test]
    fn test_ctas_collects_upstream_read() {
        let input = r#"
            ATTACH 'datatable://main' AS pg;
            CREATE TABLE IF NOT EXISTS pg.exciting_809 AS
            SELECT * FROM pg.fx_rates;
        "#;
        let assets = parse_assets(input).unwrap().assets;
        assert!(
            assets.iter().any(|a| a.path == "main/fx_rates"
                && a.kind == AssetKind::DataTable
                && a.access_type == Some(R)),
            "expected read of main/fx_rates, got {:?}",
            assets
        );
        assert!(
            assets
                .iter()
                .any(|a| a.path == "main/exciting_809" && a.access_type == Some(W)),
            "expected write of main/exciting_809, got {:?}",
            assets
        );
    }

    #[test]
    fn test_create_view_collects_upstream_read() {
        let input = r#"
            ATTACH 'datatable://main' AS pg;
            CREATE VIEW pg.v AS SELECT * FROM pg.fx_rates;
        "#;
        let assets = parse_assets(input).unwrap().assets;
        assert!(
            assets
                .iter()
                .any(|a| a.path == "main/fx_rates" && a.access_type == Some(R)),
            "expected read of main/fx_rates, got {:?}",
            assets
        );
    }

    fn lineage(input: &str) -> Vec<ColumnLineage> {
        parse_assets(input).unwrap().column_lineage
    }

    #[test]
    fn test_infer_lineage_computed_and_passthrough() {
        // CTAS with a computed column (amount + tax) and a passthrough (id).
        let input = r#"
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TABLE dl.orders_daily AS
            SELECT dl.orders.id, dl.orders.amount + dl.orders.tax AS order_total
            FROM dl.orders;
        "#;
        let got = lineage(input);
        assert_eq!(
            got,
            vec![
                ColumnLineage {
                    column: "id".to_string(),
                    inputs: vec![ColumnRef {
                        from_kind: AssetKind::Ducklake,
                        from_path: "warehouse/orders".to_string(),
                        from_column: "id".to_string(),
                    }],
                },
                ColumnLineage {
                    column: "order_total".to_string(),
                    inputs: vec![
                        ColumnRef {
                            from_kind: AssetKind::Ducklake,
                            from_path: "warehouse/orders".to_string(),
                            from_column: "amount".to_string(),
                        },
                        ColumnRef {
                            from_kind: AssetKind::Ducklake,
                            from_path: "warehouse/orders".to_string(),
                            from_column: "tax".to_string(),
                        },
                    ],
                },
            ]
        );
    }

    #[test]
    fn test_infer_lineage_bare_column_single_table() {
        // Managed-materialize form: a plain top-level SELECT, bare columns
        // attributed to the single FROM table.
        let input = r#"
            ATTACH 'ducklake://warehouse' AS dl;
            USE dl;
            SELECT amount AS revenue FROM orders;
        "#;
        let got = lineage(input);
        assert_eq!(
            got,
            vec![ColumnLineage {
                column: "revenue".to_string(),
                inputs: vec![ColumnRef {
                    from_kind: AssetKind::Ducklake,
                    from_path: "warehouse/orders".to_string(),
                    from_column: "amount".to_string(),
                }],
            }]
        );
    }

    #[test]
    fn test_infer_lineage_annotation_overrides() {
        // The `// column` annotation for `order_total` wins; `id` stays inferred.
        let input = r#"
            -- column order_total <- datatable://prod/manual.grand_total
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TABLE dl.orders_daily AS
            SELECT dl.orders.id, dl.orders.amount + dl.orders.tax AS order_total
            FROM dl.orders;
        "#;
        let got = lineage(input);
        // Annotation entry is authoritative and listed first.
        assert_eq!(got[0].column, "order_total");
        assert_eq!(got[0].inputs[0].from_path, "prod/manual");
        assert_eq!(got[0].inputs[0].from_column, "grand_total");
        // Inferred `id` survives; inferred `order_total` dropped (no dup).
        assert!(got.iter().any(|c| c.column == "id"));
        assert_eq!(got.iter().filter(|c| c.column == "order_total").count(), 1);
    }

    #[test]
    fn test_infer_lineage_skips_local_staging_ctas() {
        // A CTAS into a TEMP/local table is NOT the materialized output, so its
        // columns must not be reported (they'd be anchored to the script's
        // `// materialize` target as if they were the final asset's columns).
        // The final SELECT reads the local staging table → unresolved → empty.
        let input = r#"
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TEMP TABLE tmp AS SELECT dl.orders.amount AS amt FROM dl.orders;
            SELECT amt AS total FROM tmp;
        "#;
        assert!(
            lineage(input).is_empty(),
            "staging columns must not be reported as final output; got {:?}",
            lineage(input)
        );
    }

    #[test]
    fn test_infer_lineage_ctas_into_asset_still_inferred() {
        // A CTAS whose target IS an asset is the output, so it's still inferred.
        let input = r#"
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TABLE dl.orders_daily AS SELECT dl.orders.amount AS amt FROM dl.orders;
        "#;
        assert_eq!(
            lineage(input),
            vec![ColumnLineage {
                column: "amt".to_string(),
                inputs: vec![ColumnRef {
                    from_kind: AssetKind::Ducklake,
                    from_path: "warehouse/orders".to_string(),
                    from_column: "amount".to_string(),
                }],
            }]
        );
    }

    #[test]
    fn test_infer_lineage_temp_table_under_use_is_local() {
        // A one-part TEMP table name under an active `USE dl` must NOT resolve to
        // an asset (`warehouse/tmp`); it's local, so the final SELECT reading it
        // can't invent `warehouse/tmp.amt` as a column source for the output.
        let input = r#"
            -- materialize ducklake://warehouse/final
            ATTACH 'ducklake://warehouse' AS dl;
            USE dl;
            CREATE TEMP TABLE tmp AS SELECT amount AS amt FROM orders;
            SELECT amt AS total FROM tmp;
        "#;
        let got = lineage(input);
        assert!(
            got.is_empty(),
            "temp staging under USE must not leak warehouse/tmp as a source; got {:?}",
            got
        );
        // And no phantom `warehouse/tmp` asset is recorded.
        let assets = parse_assets(input).unwrap().assets;
        assert!(
            !assets.iter().any(|a| a.path == "warehouse/tmp"),
            "temp table must not be recorded as an asset; got {:?}",
            assets
        );
    }

    #[test]
    fn test_infer_lineage_scopes_to_materialize_target() {
        // A script with a `// materialize` target plus an AUXILIARY CTAS into a
        // different asset: only the materialized target's columns are reported;
        // the auxiliary output's columns must not be attributed to it.
        let input = r#"
            -- materialize ducklake://warehouse/final
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TABLE dl.audit AS SELECT dl.orders.id AS aid FROM dl.orders;
            SELECT dl.orders.amount AS total FROM dl.orders;
        "#;
        assert_eq!(
            lineage(input),
            vec![ColumnLineage {
                column: "total".to_string(),
                inputs: vec![ColumnRef {
                    from_kind: AssetKind::Ducklake,
                    from_path: "warehouse/orders".to_string(),
                    from_column: "amount".to_string(),
                }],
            }],
            "auxiliary `audit` columns must not appear on the materialized target"
        );
    }

    #[test]
    fn test_infer_lineage_drops_ambiguous_multi_output() {
        // No `// materialize` target and two real CTAS outputs: which asset the
        // flat lineage describes is ambiguous, so inference is dropped rather
        // than attributed to an arbitrary one.
        let input = r#"
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TABLE dl.a AS SELECT dl.orders.id AS x FROM dl.orders;
            CREATE TABLE dl.b AS SELECT dl.orders.amount AS y FROM dl.orders;
        "#;
        assert!(
            lineage(input).is_empty(),
            "ambiguous multi-output must drop inference"
        );
    }

    #[test]
    fn test_infer_lineage_wildcard_yields_nothing() {
        // `SELECT *` has no enumerable output columns → no inferred lineage.
        let input = r#"
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TABLE dl.orders_daily AS SELECT * FROM dl.orders;
        "#;
        assert!(lineage(input).is_empty());
    }

    #[test]
    fn test_infer_lineage_resolves_joined_inputs() {
        // Columns from BOTH sides of an explicit JOIN must resolve, incl. a
        // computed column mixing the two. A bare column is dropped (ambiguous
        // across the join) rather than misattributed to the first table.
        let input = r#"
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TABLE dl.orders_daily AS
            SELECT o.id, c.region AS cust_region, o.amount + c.discount AS net
            FROM dl.orders o
            JOIN dl.customers c ON c.id = o.customer_id;
        "#;
        let got = lineage(input);
        assert_eq!(
            got,
            vec![
                ColumnLineage {
                    column: "id".to_string(),
                    inputs: vec![ColumnRef {
                        from_kind: AssetKind::Ducklake,
                        from_path: "warehouse/orders".to_string(),
                        from_column: "id".to_string(),
                    }],
                },
                ColumnLineage {
                    column: "cust_region".to_string(),
                    inputs: vec![ColumnRef {
                        from_kind: AssetKind::Ducklake,
                        from_path: "warehouse/customers".to_string(),
                        from_column: "region".to_string(),
                    }],
                },
                ColumnLineage {
                    column: "net".to_string(),
                    inputs: vec![
                        ColumnRef {
                            from_kind: AssetKind::Ducklake,
                            from_path: "warehouse/orders".to_string(),
                            from_column: "amount".to_string(),
                        },
                        ColumnRef {
                            from_kind: AssetKind::Ducklake,
                            from_path: "warehouse/customers".to_string(),
                            from_column: "discount".to_string(),
                        },
                    ],
                },
            ]
        );
    }

    #[test]
    fn test_infer_lineage_does_not_descend_into_subqueries() {
        // A scalar subquery's bare column (`amount`) belongs to the subquery's
        // own FROM, NOT the outer `dl.orders` — it must not be attributed to the
        // outer table. The subquery column is left to annotations; the
        // passthrough `id` still resolves.
        let input = r#"
            ATTACH 'ducklake://warehouse' AS dl;
            CREATE TABLE dl.orders_daily AS
            SELECT dl.orders.id, (SELECT amount FROM dl.other LIMIT 1) AS c
            FROM dl.orders;
        "#;
        let got = lineage(input);
        assert_eq!(
            got,
            vec![ColumnLineage {
                column: "id".to_string(),
                inputs: vec![ColumnRef {
                    from_kind: AssetKind::Ducklake,
                    from_path: "warehouse/orders".to_string(),
                    from_column: "id".to_string(),
                }],
            }],
            "subquery column `c` must be dropped, not misattributed to orders"
        );
    }
}
