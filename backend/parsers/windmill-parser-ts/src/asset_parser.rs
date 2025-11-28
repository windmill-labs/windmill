use std::collections::HashMap;

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
    let mut assets_finder = AssetsFinder {
        assets: vec![],
        datatable_identifiers: HashMap::new(),
        ducklake_identifiers: HashMap::new(),
    };
    assets_finder.visit_module_items(&ast);
    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder {
    assets: Vec<ParseAssetsResult<String>>,

    // The user will write code like:
    //   let sql = wmill.datatable('main')
    //   return await sql`SELECT * FROM friends WHERE age = ${21}`.fetch()
    // The goal is to remember that the identifier "sql" corresponds to the datatable "main"
    // so that when we see a tagged template expression with tag "sql" we know which datatable it
    // corresponds to. This allows us to infer if a datatable is Read or Write based on the SQL query.
    datatable_identifiers: HashMap<String, String>,

    // Similar to datatable_identifiers but for ducklakes
    ducklake_identifiers: HashMap<String, String>,
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

    fn visit_block_stmt(&mut self, node: &swc_ecma_ast::BlockStmt) {
        // Save current state before entering the block
        let saved_datatable = self.datatable_identifiers.clone();
        let saved_ducklake = self.ducklake_identifiers.clone();

        // Visit children (this may add new identifiers)
        node.visit_children_with(self);

        // If we found 'let sql = wmill.datatable(...)',
        // but no sql`` tagged templates were used, we add
        // the asset with unknown access type
        for datatable in self.datatable_identifiers.keys() {
            if saved_datatable.contains_key(datatable) {
                continue;
            }
            let path = &self.datatable_identifiers[datatable];
            if self
                .assets
                .iter()
                .any(|a| a.kind == AssetKind::DataTable && a.path == *path)
            {
                continue;
            }
            self.assets.push(ParseAssetsResult {
                kind: AssetKind::DataTable,
                access_type: None,
                path: path.clone(),
            });
        }
        // Same as above but for ducklake
        for ducklake in self.ducklake_identifiers.keys() {
            if saved_ducklake.contains_key(ducklake) {
                continue;
            }
            let path = &self.ducklake_identifiers[ducklake];
            if self
                .assets
                .iter()
                .any(|a| a.kind == AssetKind::Ducklake && a.path == *path)
            {
                continue;
            }
            self.assets.push(ParseAssetsResult {
                kind: AssetKind::Ducklake,
                access_type: None,
                path: path.clone(),
            });
        }

        // Restore state - identifiers declared in this block go out of scope
        self.datatable_identifiers = saved_datatable;
        self.ducklake_identifiers = saved_ducklake;
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
                                    self.datatable_identifiers.insert(var_name, asset_name);
                                    return;
                                }
                                "ducklake" => {
                                    self.ducklake_identifiers.insert(var_name, asset_name);
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

        // Check if it's a known datatable or ducklake identifier
        let (kind, asset_name) = if let Some(name) = self.datatable_identifiers.get(tag_name) {
            (AssetKind::DataTable, name.clone())
        } else if let Some(name) = self.ducklake_identifiers.get(tag_name) {
            (AssetKind::Ducklake, name.clone())
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
            .join(" ");

        // Determine access type based on SQL keywords
        let access_type = detect_sql_access_type(&sql);

        self.assets
            .push(ParseAssetsResult { kind, path: asset_name, access_type });
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

fn detect_sql_access_type(sql: &str) -> Option<AssetUsageAccessType> {
    let first_kw = sql
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();

    // Check for write operations
    let has_write = first_kw.starts_with("insert")
        || first_kw.starts_with("update")
        || first_kw.starts_with("delete")
        || first_kw.starts_with("drop")
        || first_kw.starts_with("create")
        || first_kw.starts_with("alter")
        || first_kw.starts_with("truncate")
        || first_kw.starts_with("merge");

    // Check for read operations
    let has_read = first_kw.starts_with("select")
        || first_kw.starts_with("with")  // CTEs, usually for reads
        || first_kw.starts_with("show")
        || first_kw.starts_with("describe")
        || first_kw.starts_with("explain");

    match (has_read, has_write) {
        (true, true) => Some(RW),
        (true, false) => Some(R),
        (false, true) => Some(W),
        (false, false) => None, // Unknown - couldn't determine
    }
}
