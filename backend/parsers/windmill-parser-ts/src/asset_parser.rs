use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{CallExpr, Expr, Lit, MemberExpr, MemberProp, Str};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};
use windmill_parser::asset_parser::{
    merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(code: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
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
    let mut assets_finder = AssetsFinder { assets: vec![] };
    assets_finder.visit_module_items(&ast);
    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder {
    assets: Vec<ParseAssetsResult<String>>,
}

impl Visit for AssetsFinder {
    // visit_call_expr will not recurse if it detects an asset,
    // so this will only be called when no further context was found
    fn visit_lit(&mut self, node: &swc_ecma_ast::Lit) {
        match node {
            swc_ecma_ast::Lit::Str(str) => {
                if let Some((kind, path)) = parse_asset_syntax(str.value.as_str()) {
                    self.assets.push(ParseAssetsResult {
                        kind,
                        path: path.to_string(),
                        access_type: None,
                    });
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

impl AssetsFinder {
    fn visit_call_expr_inner(&mut self, node: &swc_ecma_ast::CallExpr) -> Result<(), ()> {
        let ident = match node.callee.as_expr().map(AsRef::as_ref) {
            Some(Expr::Ident(i)) => i.sym.as_str(),
            Some(Expr::Member(MemberExpr { prop: MemberProp::Ident(i), .. })) => i.sym.as_str(),
            _ => return Err(()),
        };
        let (kind, access_type, arg_pos) = match ident {
            "loadS3File" => (AssetKind::S3Object, Some(R), 0),
            "loadS3FileStream" => (AssetKind::S3Object, Some(R), 0),
            "writeS3File" => (AssetKind::S3Object, Some(W), 0),
            "getResource" => (AssetKind::Resource, None, 0),
            "setResource" => (AssetKind::Resource, Some(W), 1),
            "databaseUrlFromResource" => (AssetKind::Resource, None, 0),
            "denoS3LightClientSettings" => (AssetKind::Resource, None, 0),
            "duckdbConnectionSettings" => (AssetKind::Resource, None, 0),
            "polarsConnectionSettings" => (AssetKind::Resource, None, 0),
            _ => return Err(()),
        };

        let arg_value = node.args.get(arg_pos);

        match arg_value.map(|e| e.expr.as_ref()) {
            Some(Expr::Lit(Lit::Str(Str { value, .. }))) => {
                let path = parse_asset_syntax(&value).map(|(_, p)| p).unwrap_or(&value);
                self.assets
                    .push(ParseAssetsResult { kind, path: path.to_string(), access_type });
            }
            _ => return Err(()),
        }
        Ok(())
    }
}
