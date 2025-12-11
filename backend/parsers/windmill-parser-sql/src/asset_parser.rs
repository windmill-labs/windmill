use std::collections::HashMap;

use sqlparser::{
    ast::{
        CopyTarget, Expr, ObjectName, TableFactor, TableObject, Value, ValueWithSpan, Visit,
        Visitor,
    },
    dialect::DuckDbDialect,
    parser::Parser,
};
use windmill_parser::asset_parser::{
    asset_was_used, merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType,
    ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult>> {
    let statements = Parser::parse_sql(&DuckDbDialect, input)?;

    let mut collector = AssetCollector::new();
    for statement in statements {
        let _ = statement.visit(&mut collector);
    }

    for (_, (kind, path)) in collector.var_identifiers {
        if !asset_was_used(&collector.assets, (kind, &path)) {
            collector
                .assets
                .push(ParseAssetsResult { kind, access_type: None, path: path });
        }
    }

    Ok(merge_assets(collector.assets))
}

/// Visitor that collects S3 asset literals from SQL statements
struct AssetCollector {
    assets: Vec<ParseAssetsResult>,
    // e.g set to Read when we are inside a SELECT ... FROM ... statement
    current_access_type_stack: Vec<AssetUsageAccessType>,
    // e.g ATTACH 'ducklake://a' AS dl; => { "dl": (Ducklake, "a") }
    var_identifiers: HashMap<String, (AssetKind, String)>,
    // e.g USE dl;
    currently_used_asset: Option<(AssetKind, String)>,
}

impl AssetCollector {
    fn new() -> Self {
        Self {
            assets: Vec::new(),
            current_access_type_stack: Vec::with_capacity(8),
            var_identifiers: HashMap::new(),
            currently_used_asset: None,
        }
    }

    // Detect when we do 'a.b' and 'a' is associated with an asset in var_identifiers
    // Or when we access 'b' and we did USE a;
    fn get_associated_asset_from_obj_name(&self, name: &ObjectName) -> Option<ParseAssetsResult> {
        let access_type = self.current_access_type_stack.last().copied();
        if name.0.len() == 1 {
            let ident = name.0.first()?.as_ident()?;
            if ident.quote_style.is_some() {
                return None;
            }
            let specific_table = &ident.value;
            // We don't want to infer that any simple identifier refers to an asset if
            // we are not in a known R/W context
            if access_type.is_none() {
                return None;
            }

            if let Some((kind, path)) = &self.currently_used_asset {
                let path = format!("{}/{}", path, specific_table);
                return Some(ParseAssetsResult { kind: *kind, access_type, path });
            }
        }

        // Check if the first part of the name (the a in a.b) is associated with an asset
        if name.0.len() < 2 {
            return None;
        }
        let ident = name.0.first()?.as_ident()?;
        let (kind, path) = self.var_identifiers.get(&ident.value)?;
        let path = if name.0.len() == 2 {
            let specific_table = &name.0.get(1)?.as_ident()?.value;
            format!("{}/{}", path, specific_table)
        } else {
            path.clone()
        };
        Some(ParseAssetsResult { kind: *kind, access_type, path })
    }

    fn handle_string_literal(&mut self, s: &str) {
        // Check if the string matches our asset syntax patterns
        if let Some((kind, path)) = parse_asset_syntax(s, false) {
            if kind == AssetKind::S3Object {
                self.assets.push(ParseAssetsResult {
                    kind,
                    path: path.to_string(),
                    access_type: self.current_access_type_stack.last().copied(),
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

        // Writes to tables should be handled directly when visiting the statement
        if self.current_access_type_stack.last() == Some(&R) {
            if let Some(asset) = self.get_associated_asset_from_obj_name(name) {
                self.assets.push(asset);
            }
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

    fn handle_table_with_joins(&mut self, table_with_joins: &sqlparser::ast::TableWithJoins) {
        if let TableFactor::Table { name, .. } = &table_with_joins.relation {
            if let Some(asset) = self.get_associated_asset_from_obj_name(name) {
                self.assets.push(asset);
            }
        }
        for join in &table_with_joins.joins {
            if let TableFactor::Table { name, .. } = &join.relation {
                if let Some(asset) = self.get_associated_asset_from_obj_name(name) {
                    self.assets.push(asset);
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
            sqlparser::ast::Statement::Query(_) => {
                // don't forget pop() in post_visit_statement
                self.current_access_type_stack.push(R);
            }

            sqlparser::ast::Statement::Insert(insert) => {
                let access_type = if insert.returning.is_some() { RW } else { W };
                self.current_access_type_stack.push(access_type);
                match insert.table {
                    TableObject::TableName(ref name) => {
                        if let Some(asset) = self.get_associated_asset_from_obj_name(name) {
                            self.assets.push(asset);
                        }
                    }
                    _ => {}
                }
                self.current_access_type_stack.pop();
            }

            sqlparser::ast::Statement::Update { returning, table, from, .. } => {
                if let Some(from_tables) = from {
                    let from_tables = match from_tables {
                        sqlparser::ast::UpdateTableFromKind::AfterSet(tables) => tables,
                        sqlparser::ast::UpdateTableFromKind::BeforeSet(tables) => tables,
                    };
                    self.current_access_type_stack.push(R);
                    for table_with_joins in from_tables {
                        self.handle_table_with_joins(table_with_joins);
                    }
                    self.current_access_type_stack.pop();
                }

                let access_type = if returning.is_some() { RW } else { W };
                self.current_access_type_stack.push(access_type);

                self.handle_table_with_joins(table);

                self.current_access_type_stack.pop();
            }

            sqlparser::ast::Statement::Delete(delete) => {
                let access_type = if delete.returning.is_some() { RW } else { W };
                self.current_access_type_stack.push(access_type);
                for name in &delete.tables {
                    if let Some(asset) = self.get_associated_asset_from_obj_name(name) {
                        self.assets.push(asset);
                    }
                }
                let tables = match &delete.from {
                    sqlparser::ast::FromTable::WithFromKeyword(tables) => tables,
                    sqlparser::ast::FromTable::WithoutKeyword(tables) => tables,
                };
                for table_with_joins in tables {
                    self.handle_table_with_joins(table_with_joins);
                }
                self.current_access_type_stack.pop();
            }

            sqlparser::ast::Statement::CreateTable(create_table) => {
                self.current_access_type_stack.push(W);
                if let Some(asset) = self.get_associated_asset_from_obj_name(&create_table.name) {
                    self.assets.push(asset);
                }
                self.current_access_type_stack.pop();
            }

            sqlparser::ast::Statement::CreateView { name, .. } => {
                self.current_access_type_stack.push(W);
                if let Some(asset) = self.get_associated_asset_from_obj_name(name) {
                    self.assets.push(asset);
                }
                self.current_access_type_stack.pop();
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
        statement: &sqlparser::ast::Statement,
    ) -> std::ops::ControlFlow<Self::Break> {
        match statement {
            sqlparser::ast::Statement::Query(_) => {
                self.current_access_type_stack.pop();
            }
            _ => {}
        }
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/a.parquet".to_string(),
                    access_type: Some(R)
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/c.parquet".to_string(),
                    access_type: Some(W)
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "snd/b.parquet".to_string(),
                    access_type: Some(R)
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl".to_string(),
                access_type: None
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_attach_dot_notation_read() {
        let input = r#"
            ATTACH 'ducklake://my_dl' AS dl;
            SELECT * FROM dl.table1;
        "#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/table1".to_string(),
                access_type: Some(R)
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "my_dt/table1".to_string(),
                access_type: Some(W)
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
        let s = parse_assets(input);
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "my_dl/table1".to_string(),
                access_type: Some(W)
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_default_main() {
        let input = r#"
            ATTACH 'datatable' AS dl;
            INSERT INTO dl.table1 VALUES ('test');
        "#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "main/table1".to_string(),
                access_type: Some(W)
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/friends".to_string(),
                access_type: Some(RW)
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main".to_string(),
                access_type: None
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/table1".to_string(),
                access_type: Some(W)
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/table1".to_string(),
                access_type: Some(W)
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
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Resource,
                path: "u/user/pg_resource/table1".to_string(),
                access_type: Some(R)
            },])
        );
    }

    #[test]
    fn test_sql_asset_parser_update_with_dot_notation() {
        let input = r#"
            ATTACH 'ducklake' AS dl;
            UPDATE dl.table1 SET id = NULL;
        "#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "main/table1".to_string(),
                access_type: Some(W)
            },])
        );
    }
}
