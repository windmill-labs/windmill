use std::collections::HashMap;

use sqlparser::{
    ast::{CopyTarget, Expr, ObjectName, TableFactor, Value, ValueWithSpan, Visit, Visitor},
    dialect::GenericDialect,
    parser::Parser,
};
use windmill_parser::asset_parser::{
    merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let statements = Parser::parse_sql(&GenericDialect, input)?;

    let mut collector = AssetCollector::new();
    for statement in statements {
        let _ = statement.visit(&mut collector);
    }

    Ok(merge_assets(collector.assets))
}

/// Visitor that collects S3 asset literals from SQL statements
struct AssetCollector {
    assets: Vec<ParseAssetsResult<String>>,
    // e.g set to true when we are inside a SELECT ... FROM ... statement
    current_access_type_stack: Vec<AssetUsageAccessType>,
    // e.g ATTACH 'ducklake://a' AS dl; => { "dl": (Ducklake, "a") }
    var_identifiers: HashMap<String, (AssetKind, String)>,
}

impl AssetCollector {
    fn new() -> Self {
        Self {
            assets: Vec::new(),
            current_access_type_stack: Vec::with_capacity(8),
            var_identifiers: HashMap::new(),
        }
    }

    fn handle_string_literal(&mut self, s: &str) {
        // Check if the string matches our asset syntax patterns
        if let Some((kind, path)) = parse_asset_syntax(s) {
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
}

impl Visitor for AssetCollector {
    type Break = ();

    fn pre_visit_table_factor(
        &mut self,
        table_factor: &TableFactor,
    ) -> std::ops::ControlFlow<Self::Break> {
        match table_factor {
            TableFactor::Table { name, .. } => self.handle_obj_name_pre(name),
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
            Expr::Function(func) => self.handle_obj_name_pre(&func.name),
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
        if let Some(access_type) = get_stmt_access_type(statement) {
            self.current_access_type_stack.push(access_type);
        }

        match statement {
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
                if let Some((kind, path)) = parse_asset_syntax(&database_path.value) {
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
                self.var_identifiers.remove(&database_alias.value);
            }
            _ => {}
        }

        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_statement(
        &mut self,
        statement: &sqlparser::ast::Statement,
    ) -> std::ops::ControlFlow<Self::Break> {
        if let Some(_access_type) = get_stmt_access_type(statement) {
            self.current_access_type_stack.pop();
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
}

fn is_read_fn(fname: &str) -> bool {
    fname.eq_ignore_ascii_case("read_parquet")
        || fname.eq_ignore_ascii_case("read_csv")
        || fname.eq_ignore_ascii_case("read_json")
}

fn get_stmt_access_type(statement: &sqlparser::ast::Statement) -> Option<AssetUsageAccessType> {
    match statement {
        sqlparser::ast::Statement::Query(_) => Some(R),
        sqlparser::ast::Statement::Insert(insert) => {
            Some(if insert.returning.is_some() { RW } else { W })
        }
        sqlparser::ast::Statement::Update { returning, .. } => {
            Some(if returning.is_some() { RW } else { W })
        }
        sqlparser::ast::Statement::Delete(delete) => {
            Some(if delete.returning.is_some() { RW } else { W })
        }
        _ => None,
    }
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
}
