use rustpython_ast::{Constant, Expr, ExprConstant, Visitor};
use rustpython_parser::{ast::Suite, Parse};
use windmill_parser::asset_parser::{
    merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let ast = Suite::parse(input, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let mut assets_finder = AssetsFinder { assets: vec![] };
    ast.into_iter()
        .for_each(|stmt| assets_finder.visit_stmt(stmt));
    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder {
    assets: Vec<ParseAssetsResult<String>>,
}
impl Visitor for AssetsFinder {
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
            Err(_) => self.generic_visit_expr_call(node),
        }
    }
}

impl AssetsFinder {
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
            "load_s3_file" => (S3Object, Some(R), Arg::Pos(0, "s3object")),
            "load_s3_file_reader" => (S3Object, Some(R), Arg::Pos(0, "s3object")),
            "write_s3_file" => (S3Object, Some(W), Arg::Pos(0, "s3object")),
            "get_resource" => (Resource, None, Arg::Pos(0, "path")),
            "set_resource" => (Resource, Some(W), Arg::Pos(0, "path")),
            "get_boto3_connection_settings" => (Resource, None, Arg::Pos(0, "s3_resource_path")),
            "get_polars_connection_settings" => (Resource, None, Arg::Pos(0, "s3_resource_path")),
            "get_duckdb_connection_settings" => (Resource, None, Arg::Pos(0, "s3_resource_path")),
            _ => return Err(()),
        };

        let arg_val = match arg {
            Arg::Pos(i, name) => node.args.get(i).or_else(|| {
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

enum Arg {
    // Positional arguments in python can also be used by their name
    Pos(usize, &'static str),
}
