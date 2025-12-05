use sqlparser::{
    ast::{Expr, Ident, ObjectName, TableFactor, Value, ValueWithSpan, Visit, Visitor},
    dialect::GenericDialect,
    parser::Parser,
};
use windmill_parser::asset_parser::{
    detect_sql_access_type, merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType,
    ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let statements = Parser::parse_sql(&GenericDialect, input)?;

    let mut collector = AssetCollector::new();
    for statement in statements {
        statement.visit(&mut collector);
    }

    Ok(merge_assets(collector.assets))
}

/// Visitor that collects S3 asset literals from SQL statements
struct AssetCollector {
    assets: Vec<ParseAssetsResult<String>>,
    // e.g set to true when we are inside a SELECT ... FROM ... statement
    current_access_type_stack: Vec<AssetUsageAccessType>,
}

impl AssetCollector {
    fn new() -> Self {
        Self { assets: Vec::new(), current_access_type_stack: Vec::with_capacity(8) }
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
    name.0
        .first()
        .and_then(|ident| ident.as_ident().map(|id| id.value.as_str()))
}

fn get_str_lit_from_obj_name(name: &ObjectName) -> Option<&str> {
    if name.0.len() != 1 {
        return None;
    }
    match &name.0.first()? {
        Ident { value, .. } => Some(value.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_assets() {
        let input = "SELECT * FROM read_parquet('s3:///READ.parquet'); SELECT * FROM f('s3:///UNKONWN.parquet');";
        let s = parse_assets(input);
        assert_eq!("", format!("{:?}", s));
    }
}
