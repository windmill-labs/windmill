use rustpython_ast::{Constant, Expr, ExprConstant, Visitor};
use rustpython_parser::{ast::Suite, Parse};
use std::collections::HashMap;
use windmill_parser::asset_parser::{
    asset_was_used, merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType,
    ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult>> {
    let ast = Suite::parse(input, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let mut assets_finder = AssetsFinder { assets: vec![], var_identifiers: HashMap::new() };
    ast.into_iter()
        .for_each(|stmt| assets_finder.visit_stmt(stmt));

    for (kind, path) in assets_finder.var_identifiers.into_values() {
        // if a db = wmill.datatable() was never used (e.g db.query(...)),
        // we still want to register the asset as unknown access type
        if asset_was_used(&assets_finder.assets, (kind, &path)) == false {
            assets_finder
                .assets
                .push(ParseAssetsResult { kind, access_type: None, path });
        }
    }

    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder {
    assets: Vec<ParseAssetsResult>,
    var_identifiers: HashMap<String, (AssetKind, String)>,
}

impl Visitor for AssetsFinder {
    // Handle assignment statements like: x = wmill.datatable('name')
    fn visit_stmt_assign(&mut self, node: rustpython_ast::StmtAssign) {
        // Check if the value is a call to a tracked function
        if let Some(Expr::Name(expr_name)) = node.targets.first() {
            // Remove any tracked variables with that name in case of reassignment
            let Ok(var_name) = expr_name.id.parse::<String>();
            let removed = self.var_identifiers.remove(&var_name);
            // if a db = wmill.datatable() or similar was removed, but never used (e.g db.query(...)),
            // we still want to register the asset as unknown access type
            match removed {
                Some((kind, path)) => {
                    if !asset_was_used(&self.assets, (kind, &path)) {
                        self.assets
                            .push(ParseAssetsResult { kind, access_type: None, path });
                    }
                }
                None => {}
            }

            if let Some((kind, name)) = self.extract_asset_from_call(&node.value) {
                // Track target variable
                let Ok(var_name) = expr_name.id.parse::<String>();
                self.var_identifiers
                    .insert(var_name, (kind.clone(), name.clone()));
            }
        }
        // Continue with generic visit to catch any other assets in the expression
        self.generic_visit_stmt_assign(node);
    }

    // visit_call_expr will not recurse if it detects an asset,
    // so this will only be called when no further context was found
    fn visit_expr_constant(&mut self, node: ExprConstant) {
        match node.value {
            Constant::Str(s) => {
                if let Some((kind, path)) = parse_asset_syntax(&s, false) {
                    self.assets.push(ParseAssetsResult {
                        kind,
                        path: path.to_string(),
                        access_type: None,
                    });
                }
            }
            _ => self.generic_visit_expr_constant(node),
        }
    }

    fn visit_expr_call(&mut self, node: rustpython_ast::ExprCall) {
        match self.visit_expr_call_inner(&node) {
            Ok(_) => {}
            Err(_) => {
                // Check keyword arguments for assets before falling back to generic visit
                for keyword in &node.keywords {
                    if let Expr::Constant(ExprConstant { value: Constant::Str(s), .. }) =
                        &keyword.value
                    {
                        if let Some((kind, path)) = parse_asset_syntax(s, false) {
                            self.assets.push(ParseAssetsResult {
                                kind,
                                path: path.to_string(),
                                access_type: None,
                            });
                        }
                    }
                }
                self.generic_visit_expr_call(node);
            }
        }
    }
}

impl AssetsFinder {
    /// Extract asset info from calls like wmill.datatable('name'), wmill.ducklake('name'), etc.
    fn extract_asset_from_call(&self, expr: &Expr) -> Option<(AssetKind, String)> {
        let call = expr.as_call_expr()?;

        // Check for wmill.datatable, wmill.ducklake pattern
        let attr = call.func.as_attribute_expr()?;

        // Verify the object is 'wmill'
        let obj_name = attr.value.as_name_expr()?;
        if obj_name.id.as_str() != "wmill" {
            return None;
        }

        let method_name = attr.attr.as_str();
        let kind = match method_name {
            "datatable" => AssetKind::DataTable,
            "ducklake" => AssetKind::Ducklake,
            _ => return None,
        };

        // Get the first argument (the asset name)
        let name = call
            .args
            .first()
            .or_else(|| {
                // Try keyword argument 'name' or 'path'
                call.keywords
                    .iter()
                    .find(|kw| matches!(kw.arg.as_deref(), Some("name") | Some("path")))
                    .map(|kw| &kw.value)
            })
            .and_then(|first_arg| {
                if let Expr::Constant(ExprConstant { value: Constant::Str(name), .. }) = first_arg {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "main".to_string());

        Some((kind, name))
    }

    fn visit_expr_call_inner(&mut self, node: &rustpython_ast::ExprCall) -> Result<(), ()> {
        let ident: String = node
            .func
            .as_name_expr()
            .and_then(|o| o.id.parse().ok())
            .or_else(|| {
                node.func
                    .as_attribute_expr()
                    .and_then(|attr| attr.attr.parse().ok())
            })
            .ok_or(())?;

        // `obj_name` is the object (receiver) - i.e., `obj` in `obj.method()`
        let obj_name: String = if let Expr::Attribute(rustpython_ast::ExprAttribute {
            value, ..
        }) = node.func.as_ref()
        {
            if let Expr::Name(expr_name) = value.as_ref() {
                expr_name.id.parse().map_err(|_| ())?
            } else {
                return Err(());
            }
        } else {
            return Err(());
        };

        if obj_name == "wmill" {
            // Continue
        } else if let Some((kind, ref path)) = self.var_identifiers.get(&obj_name) {
            if ident == "query" {
                let expr_name = node.args.get(0).or_else(|| {
                    node.keywords
                        .iter()
                        .find(|kw| kw.arg.as_deref() == Some("name"))
                        .map(|kw| &kw.value)
                });
                let sql = match expr_name {
                    Some(Expr::Constant(ExprConstant { value: Constant::Str(sql), .. })) => sql,
                    _ => return Err(()),
                };
                let duckdb_conn_prefix = match kind {
                    AssetKind::DataTable => "datatable",
                    AssetKind::Ducklake => "ducklake",
                    _ => return Ok(()),
                };
                let sql = format!("ATTACH '{duckdb_conn_prefix}://{path}' AS dt; USE dt; {sql}");

                // We use the SQL parser to detect if it's a read or write query
                match windmill_parser_sql::parse_assets(&sql) {
                    Ok(sql_assets) => {
                        self.assets.extend(sql_assets);
                    }
                    _ => {}
                }
                return Ok(());
            } else {
                return Err(());
            }
        } else {
            return Err(());
        }

        use AssetKind::*;
        let (kind, access_type, arg) = match ident.as_str() {
            "load_s3_file" => (S3Object, Some(R), Arg(0, "s3object")),
            "load_s3_file_reader" => (S3Object, Some(R), Arg(0, "s3object")),
            "write_s3_file" => (S3Object, Some(W), Arg(0, "s3object")),
            "get_resource" => (Resource, None, Arg(0, "path")),
            "set_resource" => (Resource, Some(W), Arg(0, "path")),
            "get_boto3_connection_settings" => (Resource, None, Arg(0, "s3_resource_path")),
            "get_polars_connection_settings" => (Resource, None, Arg(0, "s3_resource_path")),
            "get_duckdb_connection_settings" => (Resource, None, Arg(0, "s3_resource_path")),
            _ => return Err(()),
        };

        let arg_val = match arg {
            Arg(i, name) => node.args.get(i).or_else(|| {
                // Get arg by name
                node.keywords
                    .iter()
                    .find(|kw| kw.arg.as_deref() == Some(name))
                    .map(|kw| &kw.value)
            }),
        };

        match arg_val {
            Some(Expr::Constant(ExprConstant { value: Constant::Str(value), .. })) => {
                let path = parse_asset_syntax(&value, false)
                    .map(|(_, p)| p)
                    .unwrap_or(&value);
                self.assets
                    .push(ParseAssetsResult { kind, path: path.to_string(), access_type });
            }
            _ => return Err(()),
        };
        Ok(())
    }
}

// Positional arguments in python can also be used by their name
struct Arg(usize, &'static str);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_asset_parser_load_s3() {
        let input = r#"
import wmill
def main():
    wmill.load_s3_file('s3:///test.csv')
"#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/test.csv".to_string(),
                access_type: Some(R)
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_unused_sql() {
        let input = r#"
import wmill
def main():
    db = wmill.datatable()
"#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "main".to_string(),
                access_type: None
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_sql_read() {
        let input = r#"
import wmill
def main(x: int):
    db = wmill.datatable('dt')
    return db.query('SELECT * FROM friends WHERE age = $1', x).fetch()
"#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "dt/friends".to_string(),
                access_type: Some(R)
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_sql_read_write() {
        let input = r#"
import wmill
def main(x: int):
    db = wmill.datatable('dt')
    db.query('UPDATE friends SET x = $1', x).fetch()
    db.query('SELECT * FROM friends WHERE age = $1', x).fetch_one()
    db.query('SELECT * FROM analytics').fetch()
"#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "dt/analytics".to_string(),
                    access_type: Some(R)
                },
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "dt/friends".to_string(),
                    access_type: Some(RW)
                },
            ])
        );
    }

    #[test]
    fn test_py_asset_parser_multiple_sql_scopes() {
        let input = r#"
import wmill
def main():
    def f(x: int):
        db = wmill.datatable()
        return db.query('SELECT * FROM friends WHERE age = $1', x)
    db = wmill.datatable('another1')
    return db.query('INSERT INTO customers VALUES ($1)', 0)

def g():
    db = wmill.ducklake('another2')
"#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "another1/customers".to_string(),
                    access_type: Some(W)
                },
                ParseAssetsResult {
                    kind: AssetKind::Ducklake,
                    path: "another2".to_string(),
                    access_type: None
                },
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "main/friends".to_string(),
                    access_type: Some(R)
                },
            ])
        );
    }

    #[test]
    fn test_py_asset_parser_overriden_var_identifier() {
        let input = r#"
import wmill
def main():
    db = wmill.datatable('another1')
def g():
    db = wmill.ducklake()
"#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "another1".to_string(),
                    access_type: None
                },
                ParseAssetsResult {
                    kind: AssetKind::Ducklake,
                    path: "main".to_string(),
                    access_type: None
                },
            ])
        );
    }
}
