use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{CallExpr, Expr, Lit, MemberExpr, MemberProp, Str};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};
use windmill_parser::asset_parser::{
    merge_assets, parse_resource_syntax, AssetKind, ParseAssetsResult,
};

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
    paths_storage: &'a mut Vec<String>,
}

impl<'a> Visit for AssetsFinder<'a> {
    fn visit_call_expr(&mut self, node: &swc_ecma_ast::CallExpr) {
        let kind = match node.callee.as_expr().map(AsRef::as_ref) {
            Some(Expr::Ident(ident)) if ident.sym.as_str() == "getResource" => AssetKind::Resource,
            Some(Expr::Member(MemberExpr { prop: MemberProp::Ident(ident), .. }))
                if ident.sym.as_str() == "getResource" =>
            {
                AssetKind::Resource
            }

            _ => {
                <CallExpr as VisitWith<Self>>::visit_children_with(node, self);
                return;
            }
        };
        if node.args.len() != 1 {
            return;
        }
        match node.args[0].expr.as_ref() {
            Expr::Lit(Lit::Str(Str { value, .. })) => {
                if let Some(path) = parse_resource_syntax(value.as_str()) {
                    self.paths_storage.push(path.to_string());
                    self.assets
                        .push(ParseAssetsResult { kind, path: "", access_type: None });
                }
            }
            _ => {
                <CallExpr as VisitWith<Self>>::visit_children_with(node, self);
                return;
            }
        }
    }
}
