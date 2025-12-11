use std::collections::HashMap;

use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{CallExpr, Expr, Lit, MemberExpr, MemberProp, Str};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};
use windmill_parser::asset_parser::{
    asset_was_used, merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType,
    ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(code: &str) -> anyhow::Result<Vec<ParseAssetsResult>> {
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
    let mut assets_finder = AssetsFinder { assets: vec![], var_identifiers: HashMap::new() };
    assets_finder.visit_module_items(&ast);
    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder {
    assets: Vec<ParseAssetsResult>,

    // The user will write code like:
    //   let sql = wmill.datatable('main')
    //   return await sql`SELECT * FROM friends WHERE age = ${21}`.fetch()
    // The goal is to remember that the identifier "sql" corresponds to the datatable "main"
    // so that when we see a tagged template expression with tag "sql" we know which datatable it
    // corresponds to. This allows us to infer if a datatable is Read or Write based on the SQL query.
    var_identifiers: HashMap<String, (AssetKind, String)>,
}

impl Visit for AssetsFinder {
    // visit_call_expr will not recurse if it detects an asset,
    // so this will only be called when no further context was found
    fn visit_lit(&mut self, node: &swc_ecma_ast::Lit) {
        match node {
            swc_ecma_ast::Lit::Str(str) => {
                if let Some((kind, path)) = parse_asset_syntax(str.value.as_str(), false) {
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

    fn visit_block_stmt(&mut self, node: &swc_ecma_ast::BlockStmt) {
        // Save current state before entering the block
        let saved_var_identifiers = self.var_identifiers.clone();

        // Visit children (this may add new identifiers)
        node.visit_children_with(self);

        for var in self.var_identifiers.keys() {
            if saved_var_identifiers.contains_key(var) {
                continue;
            }
            let (kind, ref path) = self.var_identifiers[var];
            if asset_was_used(&self.assets, (kind, path)) {
                continue;
            }
            self.assets
                .push(ParseAssetsResult { kind, access_type: None, path: path.clone() });
        }

        // Restore state - identifiers declared in this block go out of scope
        self.var_identifiers = saved_var_identifiers;
    }

    fn visit_var_declarator(&mut self, node: &swc_ecma_ast::VarDeclarator) {
        // Extract the variable name (name1)
        let var_name = match &node.name {
            swc_ecma_ast::Pat::Ident(ident) => ident.sym.as_str().to_string(),
            _ => {
                node.visit_children_with(self);
                return;
            }
        };

        // Check if init is a call to wmill.datatable(...) or wmill.ducklake(...)
        if let Some(init) = &node.init {
            if let Expr::Call(call_expr) = init.as_ref() {
                if let Some(Expr::Member(member)) = call_expr.callee.as_expr().map(AsRef::as_ref) {
                    // Check if object is "wmill"
                    let is_wmill = matches!(
                        member.obj.as_ref(),
                        Expr::Ident(ident) if ident.sym.as_str() == "wmill"
                    );

                    if is_wmill {
                        if let MemberProp::Ident(prop) = &member.prop {
                            // Get the asset name from first arg, default to "main"
                            let asset_name = call_expr
                                .args
                                .first()
                                .and_then(|arg| match arg.expr.as_ref() {
                                    Expr::Lit(Lit::Str(s)) => Some(s.value.to_string()),
                                    _ => None,
                                })
                                .unwrap_or_else(|| "main".to_string());

                            match prop.sym.as_str() {
                                "datatable" => {
                                    self.var_identifiers
                                        .insert(var_name, (AssetKind::DataTable, asset_name));
                                    return;
                                }
                                "ducklake" => {
                                    self.var_identifiers
                                        .insert(var_name, (AssetKind::Ducklake, asset_name));
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Default: visit children
        node.visit_children_with(self);
    }

    fn visit_tagged_tpl(&mut self, node: &swc_ecma_ast::TaggedTpl) {
        // Get the tag identifier
        let tag_name = match node.tag.as_ref() {
            Expr::Ident(ident) => ident.sym.as_str(),
            _ => {
                node.visit_children_with(self);
                return;
            }
        };

        // Check if it's a known identifier
        let (kind, asset_name) = if let Some((kind, name)) = self.var_identifiers.get(tag_name) {
            (*kind, name.clone())
        } else {
            node.visit_children_with(self);
            return;
        };

        // Extract the SQL query from the template quasis (string parts)
        let sql: String = node
            .tpl
            .quasis
            .iter()
            .map(|quasi| quasi.raw.as_str())
            .collect::<Vec<_>>()
            .join("$1"); // placeholder for expressions

        let duckdb_conn_prefix = match kind {
            AssetKind::DataTable => "datatable",
            AssetKind::Ducklake => "ducklake",
            _ => return,
        };
        let sql = format!("ATTACH '{duckdb_conn_prefix}://{asset_name}' AS dt; USE dt; {sql}");

        // We use the SQL parser to detect if it's a read or write query
        match windmill_parser_sql::parse_assets(&sql) {
            Ok(sql_assets) => {
                self.assets.extend(sql_assets);
            }
            _ => {}
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
                let path = parse_asset_syntax(&value, false)
                    .map(|(_, p)| p)
                    .unwrap_or(&value);
                self.assets
                    .push(ParseAssetsResult { kind, path: path.to_string(), access_type });
            }
            _ => return Err(()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ts_asset_parser_load_s3() {
        let input = r#"
            import * as wmill from "windmill-client"
            export async function main() {
                wmill.loadS3File('s3:///test.csv')
            }
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
    fn test_ts_asset_parser_unused_sql() {
        let input = r#"
            import * as wmill from "windmill-client"
            export async function main() {
                let sql = wmill.datatable('dt')
            }
        "#;
        let s = parse_assets(input);
        assert_eq!(
            s.map_err(|e| e.to_string()),
            Ok(vec![ParseAssetsResult {
                kind: AssetKind::DataTable,
                path: "dt".to_string(),
                access_type: None
            },])
        );
    }

    #[test]
    fn test_ts_asset_parser_sql_read() {
        let input = r#"
            import * as wmill from "windmill-client"
            export async function main(x: number) {
                let sql = wmill.datatable('dt')
                return await sql`SELECT * FROM friends WHERE age = ${x}`.fetch()
            }
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
    fn test_ts_asset_parser_sql_read_write() {
        let input = r#"
            import * as wmill from "windmill-client"
            export async function main(x: number) {
                let sql = wmill.datatable('dt')
                await sql`UPDATE friends SET name = 'Pierre' WHERE age = ${x}`.fetch()
                let pierre = await sql`SELECT * FROM friends WHERE age = ${x}`.fetchOne()
                return await sql`SELECT * FROM analytics`.fetch()
            }
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
    fn test_ts_asset_parser_multiple_sql_scopes() {
        let input = r#"
            import * as wmill from "windmill-client"
            export async function main() {
                async function f(x: number) {
                    let sql = wmill.datatable()
                    return await sql`SELECT * FROM friends WHERE age = ${x}`.fetch()
                }
                let sql = wmill.datatable('another1')
                return await sql`INSERT INTO customers VALUES (${0})`.fetch()
            }

            function unused() {
                let sql = wmill.ducklake('another2')
            }
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
    fn test_ts_asset_parser_overriden_var_identifier() {
        let input = r#"
            import * as wmill from "windmill-client"
            export async function main() {
                let sql = wmill.datatable('another1')
            }
            function g() {
                let sql = wmill.ducklake()
            }
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
