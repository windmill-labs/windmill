use std::collections::BTreeMap;

use sqlparser::{
    ast::{
        CopyTarget, Expr, FunctionArg, FunctionArgExpr, ObjectName, ObjectNamePart, SelectItem,
        TableFactor, TableObject, Value, ValueWithSpan, Visit, Visitor,
    },
    dialect::DuckDbDialect,
    parser::Parser,
};
use windmill_parser::asset_parser::{
    asset_was_used, merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType,
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

    Ok(ParseAssetsOutput { assets: merge_assets(collector.assets), ..Default::default() })
}

/// Visitor that collects S3 asset literals from SQL statements
struct AssetCollector {
    assets: Vec<ParseAssetsResult>,
    // e.g set to Read when we are inside a SELECT ... FROM ... statement
    current_access_type_stack: Vec<AssetUsageAccessType>,
    // e.g ATTACH 'ducklake://a' AS dl; => { "dl": (Ducklake, "a") }
    var_identifiers: BTreeMap<String, (AssetKind, String)>,
    // e.g USE dl;
    currently_used_asset: Option<(AssetKind, String)>,
}

impl AssetCollector {
    fn new() -> Self {
        Self {
            assets: Vec::new(),
            current_access_type_stack: Vec::with_capacity(8),
            var_identifiers: BTreeMap::new(),
            currently_used_asset: None,
        }
    }

    // Detect when we do 'a.b' and 'a' is associated with an asset in var_identifiers
    // Or when we access 'b' and we did USE a;
    fn get_associated_asset_from_obj_name(
        &self,
        name: &ObjectName,
        access_type: Option<AssetUsageAccessType>,
    ) -> Option<ParseAssetsResult> {
        let access_type = access_type.or_else(|| self.current_access_type_stack.last().copied());
        if let Some((kind, path)) = &self.currently_used_asset {
            // We don't want to infer that any simple identifier refers to an asset if
            // we are not in a known R/W context
            if access_type.is_none() {
                return None;
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
                self.assets.push(ParseAssetsResult {
                    kind,
                    path: path.to_string(),
                    access_type: self.current_access_type_stack.last().copied(),
                    columns: None,
                });
            }
        }
    }

    fn handle_obj_name_pre(&mut self, name: &ObjectName) {
        if let Some(fname) = get_trivial_obj_name(name) {
            if is_read_fn(fname) {
                self.current_access_type_stack.push(R);
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

    // Extract columns from SELECT items and create individual asset results for each column
    // Only processes columns that reference known assets to avoid false positives
    fn extract_column_assets(
        &mut self,
        projection: &[SelectItem],
        from_tables: &[sqlparser::ast::TableWithJoins],
    ) {
        // Check if this is a single-table SELECT (to avoid ambiguity).
        // For S3 table functions (read_parquet/read_csv/read_json), detect the asset even
        // though args are present, since we know the file path from the string literal arg.
        let single_table = if from_tables.len() == 1 {
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

        // Build a map of table aliases/names to assets for multi-table queries.
        // For S3 table functions, only aliased references are unambiguous
        // (e.g. SELECT t.col1 FROM read_parquet('s3://...') AS t).
        let mut table_to_asset: BTreeMap<String, ParseAssetsResult> = BTreeMap::new();
        for table_with_joins in from_tables {
            if let TableFactor::Table { name, alias, args, .. } = &table_with_joins.relation {
                let has_args = args.as_ref().map_or(false, |a| !a.args.is_empty());
                if has_args {
                    // For table functions, only add to the alias map when an alias is present
                    if let Some(alias) = alias {
                        if let Some(asset) =
                            self.get_s3_asset_from_table_function(&table_with_joins.relation)
                        {
                            table_to_asset.insert(alias.name.value.clone(), asset);
                        }
                    }
                } else if let Some(asset) = self
                    .get_associated_asset_from_obj_name(name, Some(R))
                    .or_else(|| {
                        self.get_s3_asset_from_str_literal_table(&table_with_joins.relation)
                    })
                {
                    // For string literal S3 tables (e.g. FROM 's3:///file.parquet'), only add to
                    // the alias map when an alias is present (to avoid false positives).
                    // For regular named tables, use alias or table name as key.
                    let is_str_literal = get_str_lit_from_obj_name(name).is_some();
                    if is_str_literal {
                        if let Some(alias) = alias {
                            table_to_asset.insert(alias.name.value.clone(), asset);
                        }
                    } else {
                        let table_key = if let Some(alias) = alias {
                            alias.name.value.clone()
                        } else {
                            name.0
                                .last()
                                .and_then(|id| id.as_ident())
                                .map(|id| id.value.clone())
                                .unwrap_or_default()
                        };
                        table_to_asset.insert(table_key, asset);
                    }
                }
            }
        }

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
                    // Avoid Table Functions
                    self.handle_obj_name_pre(name);
                }
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
                if let Some(select) = q.body.as_select() {
                    // First, handle table references (adds table-level assets)
                    for t in &select.from {
                        self.handle_table_with_joins(t, Some(R));
                    }
                    // Then, extract column-level assets
                    self.extract_column_assets(&select.projection, &select.from);
                }
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
                if let Some(asset) =
                    self.get_associated_asset_from_obj_name(&create_table.name, Some(W))
                {
                    self.assets.push(asset);
                }
            }

            sqlparser::ast::Statement::CreateView { name, .. } => {
                if let Some(asset) = self.get_associated_asset_from_obj_name(name, Some(W)) {
                    self.assets.push(asset);
                }
            }

            sqlparser::ast::Statement::Copy { target: CopyTarget::File { filename }, .. } => {
                self.current_access_type_stack.push(W);
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
        _statement: &sqlparser::ast::Statement,
    ) -> std::ops::ControlFlow<Self::Break> {
        std::ops::ControlFlow::Continue(())
    }

    fn pre_visit_query(
        &mut self,
        _query: &sqlparser::ast::Query,
    ) -> std::ops::ControlFlow<Self::Break> {
        self.current_access_type_stack.push(R);
        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_query(
        &mut self,
        _query: &sqlparser::ast::Query,
    ) -> std::ops::ControlFlow<Self::Break> {
        self.current_access_type_stack.pop();
        std::ops::ControlFlow::Continue(())
    }

    // We do not use pre_visit_relation because we cannot know if an ObjectName is a table or a function
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
