use rustpython_ast::{Constant, Expr, ExprConstant, Visitor};
use rustpython_parser::{ast::Suite, Parse};
use windmill_parser::asset_parser::{
    merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets<'a>(
    input: &'a str,
    paths_storage: &'a mut Vec<String>,
) -> anyhow::Result<Vec<ParseAssetsResult<'a>>> {
    let ast = Suite::parse(&input, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let mut assets_finder = AssetsFinder { assets: vec![], paths_storage };
    ast.into_iter()
        .for_each(|stmt| assets_finder.visit_stmt(stmt));
    for (asset, path) in assets_finder
        .assets
        .iter_mut()
        .zip(assets_finder.paths_storage.iter_mut())
    {
        asset.path = path;
    }
    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder<'a> {
    assets: Vec<ParseAssetsResult<'a>>,
    // We have to store paths separately because of lifetime concerns
    paths_storage: &'a mut Vec<String>,
}
impl<'a> Visitor for AssetsFinder<'a> {
    fn visit_expr_call(&mut self, node: rustpython_ast::ExprCall) {
        match self.visit_expr_call_inner(&node) {
            Ok(_) => {}
            Err(_) => self.generic_visit_expr_call(node),
        }
    }
}

impl<'a> AssetsFinder<'a> {
    fn visit_expr_call_inner(&mut self, node: &rustpython_ast::ExprCall) -> Result<(), ()> {
        let ident: String = node
            .func
            .as_name_expr()
            .and_then(|o| o.id.parse().ok())
            .or_else(|| {
                node.func
                    .as_attribute_expr()
                    .and_then(|attr| attr.value.as_name_expr().and_then(|o| o.id.parse().ok()))
            })
            .ok_or(())?;

        let (kind, access_type) = match ident.as_str() {
            "get_resource" => (AssetKind::Resource, None),
            "load_s3_file" => (AssetKind::S3Object, Some(R)),
            "write_s3_file" => (AssetKind::S3Object, Some(W)),
            _ => return Err(()),
        };

        if node.args.len() < 1 {
            return Err(());
        }

        match &node.args[0] {
            Expr::Constant(ExprConstant { value: Constant::Str(value), .. }) => {
                if let Some((k, path)) = parse_asset_syntax(value.as_str()) {
                    if k != kind {
                        return Err(());
                    }
                    self.paths_storage.push(path.to_string());
                    self.assets
                        .push(ParseAssetsResult { kind, path: "", access_type });
                }
            }
            _ => return Err(()),
        };
        Ok(())
    }
}
