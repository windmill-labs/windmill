use rustpython_ast::{Constant, Expr, ExprConstant, Visitor};
use rustpython_parser::{ast::Suite, Parse};
use std::collections::HashMap;
use windmill_parser::asset_parser::{
    detect_sql_access_type, merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType,
    ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let ast = Suite::parse(input, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let mut assets_finder = AssetsFinder { assets: vec![], var_identifiers: HashMap::new() };
    ast.into_iter()
        .for_each(|stmt| assets_finder.visit_stmt(stmt));

    for (kind, name) in assets_finder.var_identifiers.into_values() {
        // if a db = wmill.datatable() was never used (e.g db.query(...)),
        // we still want to register the asset as unknown access type
        if assets_finder
            .assets
            .iter()
            .all(|a| !(a.kind == kind && a.path == name))
        {
            assets_finder
                .assets
                .push(ParseAssetsResult { kind, access_type: None, path: name });
        }
    }

    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder {
    assets: Vec<ParseAssetsResult<String>>,
    var_identifiers: HashMap<String, (AssetKind, String)>,
}

impl Visitor for AssetsFinder {
    // Handle assignment statements like: x = wmill.datatable('name')
    fn visit_stmt_assign(&mut self, node: rustpython_ast::StmtAssign) {
        // Check if the value is a call to a tracked function
        if let Some((kind, name)) = self.extract_asset_from_call(&node.value) {
            // Track all target variables
            for target in &node.targets {
                if let Expr::Name(name_expr) = target {
                    let Ok(var_name) = name_expr.id.parse::<String>();
                    self.var_identifiers
                        .insert(var_name, (kind.clone(), name.clone()));
                }
            }
        } else {
            // If not wmill.datatable or similar, remove any tracked variables
            // It means the identifier is no longer refering to an asset
            for target in &node.targets {
                if let Expr::Name(name_expr) = target {
                    let Ok(var_name) = name_expr.id.parse::<String>();
                    let removed = self.var_identifiers.remove(&var_name);
                    // if a db = wmill.datatable() or similar was removed, but never used (e.g db.query(...)),
                    // we still want to register the asset as unknown access type
                    match removed {
                        Some((kind, name)) => {
                            if self
                                .assets
                                .iter()
                                .all(|a| !(a.kind == kind && a.path == name))
                            {
                                self.assets.push(ParseAssetsResult {
                                    kind,
                                    access_type: None,
                                    path: name,
                                });
                            }
                        }
                        None => {}
                    }
                }
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
                if let Some((kind, path)) = parse_asset_syntax(&s) {
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
                        if let Some((kind, path)) = parse_asset_syntax(s) {
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
            if let Expr::Name(name_expr) = value.as_ref() {
                name_expr.id.parse().map_err(|_| ())?
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
                let name_expr = node.args.get(0).or_else(|| {
                    node.keywords
                        .iter()
                        .find(|kw| kw.arg.as_deref() == Some("name"))
                        .map(|kw| &kw.value)
                });
                match name_expr {
                    Some(Expr::Constant(ExprConstant {
                        value: Constant::Str(sql_query), ..
                    })) => {
                        let access_type = detect_sql_access_type(&sql_query);
                        self.assets.push(ParseAssetsResult {
                            kind: *kind,
                            path: path.to_string(),
                            access_type,
                        });
                        return Ok(());
                    }
                    _ => return Err(()),
                };
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
                let path = parse_asset_syntax(&value).map(|(_, p)| p).unwrap_or(&value);
                self.assets
                    .push(ParseAssetsResult { kind, path: path.to_string(), access_type });
            }
            _ => return Err(()),
        };
        Ok(())
    }
}

struct Arg(usize, &'static str);
// Positional arguments in python can also be used by their name
