use rustpython_ast::{Constant, Expr, ExprConstant, Visitor};
use rustpython_parser::{ast::Suite, Parse};
use std::collections::HashMap;
use windmill_parser::asset_parser::{
    merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let ast = Suite::parse(input, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let mut assets_finder = AssetsFinder { assets: vec![], var_identifiers: HashMap::new() };
    ast.into_iter()
        .for_each(|stmt| assets_finder.visit_stmt(stmt));
    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder {
    assets: Vec<ParseAssetsResult<String>>,
    var_identifiers: HashMap<String, (AssetKind, String)>,
}

impl Visitor for AssetsFinder {
    fn visit_stmt_function_def(&mut self, node: rustpython_ast::StmtFunctionDef) {
        // Save current scope
        let saved_vars = self.var_identifiers.clone();

        // Visit function body
        self.generic_visit_stmt_function_def(node);

        // Restore outer scope
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_async_function_def(&mut self, node: rustpython_ast::StmtAsyncFunctionDef) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_async_function_def(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_class_def(&mut self, node: rustpython_ast::StmtClassDef) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_class_def(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_for(&mut self, node: rustpython_ast::StmtFor) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_for(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_async_for(&mut self, node: rustpython_ast::StmtAsyncFor) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_async_for(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_while(&mut self, node: rustpython_ast::StmtWhile) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_while(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_if(&mut self, node: rustpython_ast::StmtIf) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_if(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_with(&mut self, node: rustpython_ast::StmtWith) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_with(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_async_with(&mut self, node: rustpython_ast::StmtAsyncWith) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_async_with(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_try(&mut self, node: rustpython_ast::StmtTry) {
        let saved_vars = self.var_identifiers.clone();
        self.generic_visit_stmt_try(node);
        self.var_identifiers = saved_vars;
    }

    fn visit_stmt_assign(&mut self, node: rustpython_ast::StmtAssign) {
        // Check if RHS is a wmill.datatable() or wmill.ducklake() call
        if let Some((kind, path)) = self.extract_asset_call(&node.value) {
            // Track all simple name targets
            for target in &node.targets {
                if let Expr::Name(name_expr) = target {
                    let Ok(var_name) = name_expr.id.parse::<String>();
                    self.var_identifiers
                        .insert(var_name, (kind.clone(), path.clone()));
                }
            }
        }

        // Continue visiting
        self.generic_visit_stmt_assign(node);
    }

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
    /// Extract asset kind and path from wmill.datatable('name') or wmill.ducklake('name') calls
    fn extract_asset_call(&self, expr: &Expr) -> Option<(AssetKind, String)> {
        let call = expr.as_call_expr()?;

        // Check for wmill.datatable or wmill.ducklake pattern
        let attr = call.func.as_attribute_expr()?;

        // Verify it's wmill.X
        let obj = attr.value.as_name_expr()?;
        if obj.id.as_str() != "wmill" {
            return None;
        }

        let method_name = attr.attr.as_str();
        let kind = match method_name {
            "datatable" => AssetKind::DataTable, // You may need to add this variant
            "ducklake" => AssetKind::Ducklake,   // You may need to add this variant
            _ => return None,
        };

        // Get the first argument (the name)
        let first_arg = call.args.first().or_else(|| {
            call.keywords
                .iter()
                .find(|kw| kw.arg.as_deref() == Some("name"))
                .map(|kw| &kw.value)
        })?;

        if let Expr::Constant(ExprConstant { value: Constant::Str(s), .. }) = first_arg {
            Some((kind, s.to_string()))
        } else {
            None
        }
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

// Positional arguments in python can also be used by their name
struct Arg(usize, &'static str);
