use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{CallExpr, Expr, Lit, MemberExpr, MemberProp, Str, TsLit};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};
use windmill_parser::asset_parser::{
    merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets<'a>(
    code: &'a str,
    paths_storage: &'a mut Vec<String>,
) -> anyhow::Result<Vec<ParseAssetsResult<'a>>> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("main.ts".into()).into(), code.into());
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Typescript(TsSyntax::default()),
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    let mut err_s = "".to_string();
    for e in parser.take_errors() {
        err_s += &e.into_kind().msg().to_string();
    }

    let ast = parser
        .parse_module()
        .map_err(|e| {
            anyhow::anyhow!("Error while parsing code, it is invalid TypeScript: {err_s}, {e:?}")
        })?
        .body;
    let mut assets_finder = AssetsFinder { assets: vec![], paths_storage };
    assets_finder.visit_module_items(&ast);
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

impl<'a> Visit for AssetsFinder<'a> {
    // visit_call_expr will not recurse if it detects an asset,
    // so this will only be called when no further context was found
    fn visit_lit(&mut self, node: &swc_ecma_ast::Lit) {
        match node {
            swc_ecma_ast::Lit::Str(str) => {
                if let Some((kind, path)) = parse_asset_syntax(str.value.as_str()) {
                    self.paths_storage.push(path.to_string());
                    self.assets
                        .push(ParseAssetsResult { kind, path: "", access_type: None });
                }
            }
            _ => <Lit as VisitWith<Self>>::visit_children_with(node, self),
        }
    }

    fn visit_call_expr(&mut self, node: &swc_ecma_ast::CallExpr) {
        match self.visit_call_expr_inner(node) {
            Ok(_) => {}
            Err(_) => <CallExpr as VisitWith<Self>>::visit_children_with(node, self),
        }
    }
}

impl<'a> AssetsFinder<'a> {
    fn visit_call_expr_inner(&mut self, node: &swc_ecma_ast::CallExpr) -> Result<(), ()> {
        let ident = match node.callee.as_expr().map(AsRef::as_ref) {
            Some(Expr::Ident(i)) => i.sym.as_str(),
            Some(Expr::Member(MemberExpr { prop: MemberProp::Ident(i), .. })) => i.sym.as_str(),
            _ => return Err(()),
        };
        let (kind, access_type) = match ident {
            "getResource" => (AssetKind::Resource, None),
            "loadS3File" => (AssetKind::S3Object, Some(R)),
            "writeS3File" => (AssetKind::S3Object, Some(W)),
            _ => return Err(()),
        };
        if node.args.len() < 1 {
            return Err(());
        }
        match node.args[0].expr.as_ref() {
            Expr::Lit(Lit::Str(Str { value, .. })) => {
                let path = parse_asset_syntax(&value).map(|(_, p)| p).unwrap_or(&value);
                self.paths_storage.push(path.to_string());
                self.assets
                    .push(ParseAssetsResult { kind, path: "", access_type });
            }
            _ => return Err(()),
        }
        Ok(())
    }
}
