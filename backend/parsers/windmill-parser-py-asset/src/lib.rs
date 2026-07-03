use rustpython_ast::{Constant, Expr, ExprConstant, Visitor};
use rustpython_parser::{ast::Suite, Parse};
use std::collections::HashMap;
use windmill_parser::asset_parser::{
    asset_was_used, merge_assets, parse_asset_syntax, parse_pipeline_annotations, AssetKind,
    AssetUsageAccessType, ParseAssetsOutput, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<ParseAssetsOutput> {
    let ast = Suite::parse(input, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let mut assets_finder = AssetsFinder { assets: vec![], var_identifiers: HashMap::new() };
    ast.into_iter()
        .for_each(|stmt| assets_finder.visit_stmt(stmt));

    for (kind, path, _) in assets_finder.var_identifiers.into_values() {
        // if a db = wmill.datatable() was never used (e.g db.query(...)),
        // we still want to register the asset as unknown access type
        if asset_was_used(&assets_finder.assets, (kind, &path)) == false {
            assets_finder.assets.push(ParseAssetsResult {
                kind,
                path,
                access_type: None,
                columns: None,
            });
        }
    }

    let pipeline = parse_pipeline_annotations(input);
    Ok(ParseAssetsOutput::new(
        merge_assets(assets_finder.assets),
        Vec::new(),
        pipeline,
    ))
}

type VarAssetName = String;
type VarAssetSchema = Option<String>;
struct AssetsFinder {
    assets: Vec<ParseAssetsResult>,
    var_identifiers: HashMap<String, (AssetKind, VarAssetName, VarAssetSchema)>,
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
                Some((kind, path, _)) => {
                    if !asset_was_used(&self.assets, (kind, &path)) {
                        self.assets.push(ParseAssetsResult {
                            kind,
                            path,
                            access_type: None,
                            columns: None,
                        });
                    }
                }
                None => {}
            }

            if let Some((kind, name, schema)) = self.extract_asset_from_call(&node.value) {
                // Track target variable
                let Ok(var_name) = expr_name.id.parse::<String>();
                self.var_identifiers
                    .insert(var_name, (kind.clone(), name.clone(), schema.clone()));
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
                        columns: None,
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
                                columns: None,
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
    fn extract_asset_from_call(
        &self,
        expr: &Expr,
    ) -> Option<(AssetKind, VarAssetName, VarAssetSchema)> {
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
            });
        let (name, schema) = match name {
            None => ("main".to_string(), None),
            Some(name) => {
                if let Some((name, s)) = name.split_once(':') {
                    let schema = Some(s.to_string());
                    let name = if name.is_empty() { "main" } else { name };
                    (name.to_string(), schema)
                } else {
                    (name, None)
                }
            }
        };

        Some((kind, name, schema))
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
        } else if let Some((kind, ref path, ref schema)) = self.var_identifiers.get(&obj_name) {
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
                // We use the SQL parser to detect RW, specific tables, etc.
                let sql_assets = windmill_parser_sql_asset::parse_wmill_sdk_sql_assets(
                    *kind,
                    path,
                    schema.as_deref(),
                    &sql,
                );
                match sql_assets {
                    Ok(Some(sql_assets)) => self.assets.extend(sql_assets),
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
            // S3 helpers take an `S3Object` (constructor or dict literal) or an
            // `s3://bucket/key` string. Other helpers take a bare resource-path
            // string literal.
            Some(arg) if matches!(kind, AssetKind::S3Object) => {
                let path = s3_object_arg_path(arg).ok_or(())?;
                self.assets
                    .push(ParseAssetsResult { kind, path, access_type, columns: None });
            }
            Some(Expr::Constant(ExprConstant { value: Constant::Str(value), .. })) => {
                let path = parse_asset_syntax(&value, false)
                    .map(|(_, p)| p)
                    .unwrap_or(&value);
                self.assets.push(ParseAssetsResult {
                    kind,
                    path: path.to_string(),
                    access_type,
                    columns: None,
                });
            }
            _ => return Err(()),
        };
        Ok(())
    }
}

// Positional arguments in python can also be used by their name
struct Arg(usize, &'static str);

/// Extract a string-literal keyword argument, e.g. `s3="value"` in a call.
fn keyword_str_value(keywords: &[rustpython_ast::Keyword], name: &str) -> Option<String> {
    keywords
        .iter()
        .find(|kw| kw.arg.as_deref() == Some(name))
        .and_then(|kw| match &kw.value {
            Expr::Constant(ExprConstant { value: Constant::Str(s), .. }) => Some(s.clone()),
            _ => None,
        })
}

/// Extract a string-literal value from a dict literal, e.g. `{"s3": "value"}`.
fn dict_str_value(dict: &rustpython_ast::ExprDict, name: &str) -> Option<String> {
    dict.keys
        .iter()
        .zip(dict.values.iter())
        .find_map(|(key, value)| match (key.as_ref()?, value) {
            (
                Expr::Constant(ExprConstant { value: Constant::Str(k), .. }),
                Expr::Constant(ExprConstant { value: Constant::Str(v), .. }),
            ) if k.as_str() == name => Some(v.clone()),
            _ => None,
        })
}

/// Resolve the SDK `S3Object` argument of `load_s3_file`/`load_s3_file_reader`/
/// `write_s3_file` to a canonical asset path, mirroring `windmill-parser-ts-asset`:
/// `S3Object(s3="<key>", storage="<bucket>"?)` — or the equivalent dict literal —
/// maps to the URI `s3://<bucket>/<key>` (empty bucket for default storage, i.e.
/// `s3:///<key>`), and a bare `"s3://bucket/key"` string is passed through.
/// The resulting URI is fed through `parse_asset_syntax` so the stored path
/// matches the TS object form and the `# on s3:///…` trigger form exactly.
fn s3_object_arg_path(expr: &Expr) -> Option<String> {
    let uri = match expr {
        Expr::Constant(ExprConstant { value: Constant::Str(s), .. }) => s.clone(),
        Expr::Call(call) => {
            // `S3Object(...)` imported directly or as `wmill.S3Object(...)`
            let func_name = call
                .func
                .as_name_expr()
                .map(|n| n.id.as_str())
                .or_else(|| call.func.as_attribute_expr().map(|a| a.attr.as_str()))?;
            if func_name != "S3Object" {
                return None;
            }
            let key = keyword_str_value(&call.keywords, "s3")?;
            let storage = keyword_str_value(&call.keywords, "storage").unwrap_or_default();
            format!("s3://{storage}/{key}")
        }
        Expr::Dict(dict) => {
            let key = dict_str_value(dict, "s3")?;
            let storage = dict_str_value(dict, "storage").unwrap_or_default();
            format!("s3://{storage}/{key}")
        }
        _ => return None,
    };
    Some(
        parse_asset_syntax(&uri, false)
            .map(|(_, p)| p.to_string())
            .unwrap_or(uri),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn test_py_asset_parser_load_s3() {
        let input = r#"
import wmill
def main():
    wmill.load_s3_file('s3:///test.csv')
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/test.csv".to_string(),
                access_type: Some(R),
                columns: None,
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_write_s3_object_constructor() {
        // The SDK signature is `write_s3_file(s3object: S3Object | str, ...)` and
        // its docstring recommends the constructor form with a bare key. It must
        // resolve to the same canonical path as the TS object form and a
        // `# on s3:///<key>` trigger.
        let input = r#"
import wmill
from wmill import S3Object
def main():
    wmill.write_s3_file(S3Object(s3="analytics/x.csv"), b"content")
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/analytics/x.csv".to_string(),
                access_type: Some(W),
                columns: None,
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_s3_object_with_storage() {
        // `S3Object(s3=..., storage=...)` maps to `s3://<storage>/<key>`,
        // matching the `s3://bucket/key` string form and the TS object form.
        let input = r#"
import wmill
from wmill import S3Object
def main():
    wmill.load_s3_file(S3Object(s3="dir/in.csv", storage="mybucket"))
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "mybucket/dir/in.csv".to_string(),
                access_type: Some(R),
                columns: None,
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_s3_object_reader_and_keyword_arg() {
        // load_s3_file_reader + passing the S3Object via the `s3object` keyword
        // and via the `wmill.S3Object` attribute form.
        let input = r#"
import wmill
def main():
    wmill.load_s3_file_reader(s3object=wmill.S3Object(s3="dir/in.csv"))
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::S3Object,
                path: "/dir/in.csv".to_string(),
                access_type: Some(R),
                columns: None,
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_s3_object_dict_literal() {
        // `S3Object` subclasses dict, so the SDK also accepts a plain dict
        // literal — same resolution as the constructor form.
        let input = r#"
import wmill
def main():
    wmill.write_s3_file({"s3": "out.json"}, b"{}")
    wmill.load_s3_file({"s3": "dir/in.csv", "storage": "mybucket"})
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/out.json".to_string(),
                    access_type: Some(W),
                    columns: None,
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "mybucket/dir/in.csv".to_string(),
                    access_type: Some(R),
                    columns: None,
                },
            ])
        );
    }

    #[test]
    fn test_py_asset_parser_multiple_s3_object_writes() {
        // Several direct constructor-form writes in main() — all outputs must
        // be detected (merge_assets returns a deterministic path-sorted order).
        let input = r#"
import wmill
from wmill import S3Object
def main():
    wmill.write_s3_file(S3Object(s3="pipelines/km_real/raw_events.json"), b"[]")
    wmill.write_s3_file(S3Object(s3="pipelines/km_real/enriched.json"), b"[]")
    wmill.write_s3_file(S3Object(s3="pipelines/km_real/summary.json"), b"[]")
    wmill.write_s3_file(S3Object(s3="pipelines/km_real/report.json"), b"{}")
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/pipelines/km_real/enriched.json".to_string(),
                    access_type: Some(W),
                    columns: None,
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/pipelines/km_real/raw_events.json".to_string(),
                    access_type: Some(W),
                    columns: None,
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/pipelines/km_real/report.json".to_string(),
                    access_type: Some(W),
                    columns: None,
                },
                ParseAssetsResult {
                    kind: AssetKind::S3Object,
                    path: "/pipelines/km_real/summary.json".to_string(),
                    access_type: Some(W),
                    columns: None,
                },
            ])
        );
    }

    #[test]
    fn test_py_asset_parser_s3_object_dynamic_key_no_false_positive() {
        // A computed key can't be resolved statically — must yield nothing
        // rather than a bogus path.
        let input = r#"
import wmill
from wmill import S3Object
def main(name: str):
    wmill.write_s3_file(S3Object(s3=f"pipelines/{name}.json"), b"{}")
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(s.map_err(|e| e.to_string()), Ok(vec![]));
    }

    #[test]
    fn test_py_asset_parser_unused_sql() {
        let input = r#"
import wmill
def main():
    db = wmill.datatable()
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "main".to_string(),
                access_type: None,
                columns: None,
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
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "dt/friends".to_string(),
                access_type: Some(R),
                columns: None,
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
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "dt/analytics".to_string(),
                    access_type: Some(R),
                    columns: None,
                },
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "dt/friends".to_string(),
                    access_type: Some(RW),
                    columns: Some(BTreeMap::from([("x".to_string(), AssetUsageAccessType::W)])),
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
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "another1/customers".to_string(),
                    access_type: Some(W),
                    columns: None,
                },
                ParseAssetsResult {
                    kind: AssetKind::Ducklake,
                    path: "another2".to_string(),
                    access_type: None,
                    columns: None,
                },
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "main/friends".to_string(),
                    access_type: Some(R),
                    columns: None,
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
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![
                ParseAssetsResult {
                    kind: AssetKind::DataTable,
                    path: "another1".to_string(),
                    access_type: None,
                    columns: None,
                },
                ParseAssetsResult {
                    kind: AssetKind::Ducklake,
                    path: "main".to_string(),
                    access_type: None,
                    columns: None,
                },
            ])
        );
    }

    #[test]
    fn test_py_asset_parser_datatable_with_schema() {
        let input = r#"
import wmill
def main(x: int):
    db = wmill.datatable('dt:public')
    return db.query('SELECT * FROM friends WHERE age = $1', x).fetch()
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "dt/public.friends".to_string(),
                access_type: Some(R),
                columns: None,
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_ducklake_with_schema() {
        let input = r#"
import wmill
def main():
    db = wmill.ducklake('lake1:analytics')
    return db.query('SELECT * FROM metrics').fetch()
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::Ducklake,
                path: "lake1/analytics.metrics".to_string(),
                access_type: Some(R),
                columns: None,
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_schema_with_write() {
        let input = r#"
import wmill
def main(x: int):
    db = wmill.datatable('dt:public')
    db.query('INSERT INTO users VALUES ($1)', x).fetch()
    return db.query('SELECT * FROM users').fetch()
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "dt/public.users".to_string(),
                access_type: Some(RW),
                columns: None,
            },])
        );
    }

    #[test]
    fn test_py_asset_parser_unused_datatable_with_schema() {
        let input = r#"
import wmill
def main():
    db = wmill.datatable('dt:public')
"#;
        let s = parse_assets(input).map(|o| o.assets);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "dt".to_string(),
                access_type: None,
                columns: None,
            },])
        );
    }
}
