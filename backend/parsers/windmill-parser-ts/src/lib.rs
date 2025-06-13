/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
// use deno_core::{serde_v8, v8, JsRuntime, RuntimeOptions};
use serde_json::Value;
use std::collections::HashSet;
use swc_ecma_visit::{noop_visit_type, Visit, VisitWith};
use windmill_parser::{
    json_to_typ, to_snake_case, Arg, MainArgSignature, ObjectProperty, OneOfVariant, Typ,
};

use swc_common::{sync::Lrc, FileName, SourceMap, SourceMapper, Span, Spanned};
use swc_ecma_ast::{
    ArrayLit, AssignPat, BigInt, BindingIdent, Bool, Decl, ExportDecl, Expr, FnDecl, Ident,
    IdentName, Lit, MemberExpr, MemberProp, ModuleDecl, ModuleItem, Number, ObjectLit, ObjectPat,
    Param, Pat, Str, TsArrayType, TsEntityName, TsKeywordType, TsKeywordTypeKind, TsLit, TsLitType,
    TsOptionalType, TsParenthesizedType, TsPropertySignature, TsType, TsTypeAnn, TsTypeElement,
    TsTypeLit, TsTypeRef, TsUnionOrIntersectionType, TsUnionType,
};
use swc_ecma_parser::{lexer::Lexer, EsSyntax, Parser, StringInput, Syntax, TsSyntax};

use regex::Regex;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

struct ImportsFinder {
    imports: HashSet<String>,
}

impl Visit for ImportsFinder {
    noop_visit_type!();

    fn visit_import_decl(&mut self, n: &swc_ecma_ast::ImportDecl) {
        if let Some(ref s) = n.src.raw {
            let s = s.to_string();
            if s.starts_with("'") && s.ends_with("'") {
                self.imports.insert(s[1..s.len() - 1].to_string());
            } else if s.starts_with("\"") && s.ends_with("\"") {
                self.imports.insert(s[1..s.len() - 1].to_string());
            }
        }
    }
}

pub fn parse_expr_for_imports(code: &str) -> anyhow::Result<Vec<String>> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("main.d.ts".into()).into(), code.into());
    let mut tss = TsSyntax::default();
    tss.disallow_ambiguous_jsx_like;
    tss.tsx = true;
    tss.no_early_errors = true;
    let lexer = Lexer::new(
        Syntax::Typescript(tss),
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

    let expr = parser.parse_module().map_err(|e| {
        anyhow::anyhow!("Error while parsing code, it is invalid TypeScript: {err_s}, {e:?}")
    })?;

    let mut visitor = ImportsFinder { imports: HashSet::new() };
    visitor.visit_module(&expr);

    let mut imports: Vec<_> = visitor.imports.into_iter().collect();
    imports.sort();
    Ok(imports)
}

struct OutputFinder {
    idents: HashSet<(String, String)>,
}

impl Visit for OutputFinder {
    noop_visit_type!();

    fn visit_member_expr(&mut self, m: &MemberExpr) {
        m.obj.visit_with(self);
        if let MemberProp::Computed(c) = &m.prop {
            c.visit_with(self);
        }
        match m {
            MemberExpr { obj, prop: MemberProp::Ident(IdentName { sym, .. }), .. } => {
                match *obj.to_owned() {
                    Expr::Ident(Ident { sym: sym_i, .. }) => {
                        self.idents.insert((sym_i.to_string(), sym.to_string()));
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
}

pub fn parse_expr_for_ids(code: &str) -> anyhow::Result<Vec<(String, String)>> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("main.ts".into()).into(), code.into());
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Es(EsSyntax { jsx: false, ..Default::default() }),
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

    let expr = parser.parse_module().map_err(|e| {
        anyhow::anyhow!("Error while parsing code, it is invalid TypeScript: {err_s}, {e:?}")
    })?;

    let mut visitor = OutputFinder { idents: HashSet::new() };
    visitor.visit_module(&expr);

    Ok(visitor.idents.into_iter().collect())
}

/// skip_params is a micro optimization for when we just want to find the main
/// function without parsing all the params.
pub fn parse_deno_signature(
    code: &str,
    skip_dflt: bool,
    skip_params: bool,
    main_override: Option<String>,
) -> anyhow::Result<MainArgSignature> {
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

    let has_preprocessor = ast.iter().any(|x| match x {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            decl: Decl::Fn(FnDecl { ident: Ident { sym, .. }, .. }),
            ..
        })) => &sym.to_string() == "preprocessor",
        _ => false,
    });

    let main_name = main_override.unwrap_or("main".to_string());
    let params = ast.into_iter().find_map(|x| match x {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            decl: Decl::Fn(FnDecl { ident: Ident { sym, .. }, function, .. }),
            ..
        })) if &sym.to_string() == &main_name => Some(function.params),
        _ => None,
    });

    let mut c: u16 = 0;
    let no_main_func = params.is_none();
    let r = MainArgSignature {
        star_args: false,
        star_kwargs: false,
        args: if skip_params {
            vec![]
        } else {
            params
                .map(|x| {
                    x.into_iter()
                        .map(|x| parse_param(x, &cm, skip_dflt, &mut c))
                        .collect::<anyhow::Result<Vec<Arg>>>()
                })
                .transpose()?
                .unwrap_or_else(|| vec![])
        },
        no_main_func: Some(no_main_func),
        has_preprocessor: Some(has_preprocessor),
    };
    Ok(r)
}

fn parse_param(
    x: Param,
    cm: &Lrc<SourceMap>,
    skip_dflt: bool,
    counter: &mut u16,
) -> anyhow::Result<Arg> {
    let r = match x.pat {
        Pat::Ident(ident) => {
            let (name, typ, nullable) = binding_ident_to_arg(&ident);
            Ok(Arg {
                otyp: None,
                name,
                typ,
                default: None,
                has_default: ident.id.optional || nullable,
                oidx: None,
            })
        }
        // Pat::Object(ObjectPat { ... }) = todo!()
        Pat::Assign(AssignPat { left, right, .. }) => {
            let (name, mut typ, _nullable) = match *left {
                Pat::Ident(ident) => binding_ident_to_arg(&ident),
                Pat::Object(ObjectPat { type_ann, .. }) => {
                    let (typ, nullable) = eval_type_ann(&type_ann);
                    *counter += 1;
                    let name = format!("anon{}", counter);
                    (name, typ, nullable)
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "parameter syntax unsupported: `{}`: {:#?}",
                        cm.span_to_snippet(left.span())
                            .unwrap_or_else(|_| cm.span_to_string(left.span())),
                        *left
                    ))
                }
            };

            let dflt = if skip_dflt {
                None
            } else {
                match *right {
                    Expr::Lit(Lit::Str(Str { value, .. })) => {
                        Some(Value::String(value.to_string()))
                    }
                    Expr::Lit(Lit::Num(Number { value, .. }))
                        if (value == (value as u64) as f64) =>
                    {
                        Some(serde_json::json!(value as u64))
                    }
                    Expr::Lit(Lit::Num(Number { value, .. })) => Some(serde_json::json!(value)),
                    Expr::Lit(Lit::BigInt(BigInt { value, .. })) => Some(serde_json::json!(value)),
                    Expr::Lit(Lit::Bool(Bool { value, .. })) => Some(Value::Bool(value)),
                    Expr::Object(ObjectLit { span, .. }) => eval_span(span, cm),
                    Expr::Array(ArrayLit { span, .. }) => eval_span(span, cm),
                    _ => None,
                }
            };

            if typ == Typ::Unknown && dflt.is_some() {
                typ = json_to_typ(dflt.as_ref().unwrap(), false);
            }
            Ok(Arg { otyp: None, name, typ, default: dflt, has_default: true, oidx: None })
        }
        Pat::Object(ObjectPat { type_ann, .. }) => {
            let (typ, nullable) = eval_type_ann(&type_ann);
            *counter += 1;
            let name = format!("anon{}", counter);
            Ok(Arg { otyp: None, name, typ, default: None, has_default: nullable, oidx: None })
        }
        _ => Err(anyhow::anyhow!(
            "parameter syntax unsupported: `{}`: {:#?}",
            cm.span_to_snippet(x.span())
                .unwrap_or_else(|_| cm.span_to_string(x.span())),
            x.pat
        )),
    };
    r
}

fn eval_span(span: Span, cm: &Lrc<SourceMap>) -> Option<Value> {
    let expr = cm
        .span_to_snippet(span)
        .ok()
        .map(|x| serde_json::from_str(&x).map_err(|_| x));

    match expr {
        Some(Ok(x)) => Some(x),
        Some(Err(x)) => eval_sync(&x).ok(),
        None => None,
    }
}

fn eval_type_ann(type_ann: &Option<Box<TsTypeAnn>>) -> (Typ, bool) {
    return type_ann
        .as_ref()
        .map(|x| tstype_to_typ(&*x.type_ann))
        .unwrap_or((Typ::Unknown, false));
}
fn binding_ident_to_arg(BindingIdent { id, type_ann }: &BindingIdent) -> (String, Typ, bool) {
    let (typ, nullable) = eval_type_ann(type_ann);
    (id.sym.to_string(), typ, nullable)
}

lazy_static::lazy_static! {
     static ref IMPORTS_VERSION: Regex = Regex::new(r"^((?:\@[^\/\@]+\/[^\/\@]+)|(?:[^\/\@]+))(?:\@(?:[^\/]+))?(.*)$").unwrap();

}

pub fn remove_pinned_imports(code: &str) -> anyhow::Result<String> {
    let mut imports = parse_expr_for_imports(code)?;
    imports.sort_by_key(|f| 0 - (f.len() as i32));
    let mut content = code.to_string();
    for import in imports {
        let to_c = IMPORTS_VERSION.captures(&import);
        if let Some(to) = to_c.and_then(|x| {
            x.get(1).map(|y| {
                format!(
                    "{}{}",
                    y.as_str(),
                    x.get(2).map(|z| z.as_str()).unwrap_or("")
                )
            })
        }) {
            content = content.replace(&import, &to);
        }
    }
    Ok(content)
}

fn tstype_to_typ(ts_type: &TsType) -> (Typ, bool) {
    match ts_type {
        TsType::TsKeywordType(t) => (
            match t.kind {
                TsKeywordTypeKind::TsObjectKeyword => Typ::Object(vec![]),
                TsKeywordTypeKind::TsBooleanKeyword => Typ::Bool,
                TsKeywordTypeKind::TsBigIntKeyword => Typ::Int,
                TsKeywordTypeKind::TsNumberKeyword => Typ::Float,
                TsKeywordTypeKind::TsStringKeyword => Typ::Str(None),
                _ => Typ::Unknown,
            },
            false,
        ),
        TsType::TsTypeLit(TsTypeLit { members, .. }) => {
            let properties = members
                .into_iter()
                .filter_map(|x| match x {
                    TsTypeElement::TsPropertySignature(TsPropertySignature {
                        key,
                        type_ann,
                        ..
                    }) => match (*key.to_owned(), type_ann) {
                        (Expr::Ident(Ident { sym, .. }), type_ann) => Some(ObjectProperty {
                            key: sym.to_string(),
                            typ: type_ann
                                .as_ref()
                                .map(|typ| Box::new(tstype_to_typ(&*typ.type_ann).0))
                                .unwrap_or(Box::new(Typ::Unknown)),
                        }),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            (Typ::Object(properties), false)
        }
        TsType::TsParenthesizedType(TsParenthesizedType { type_ann, .. }) => {
            tstype_to_typ(type_ann)
        }
        // TODO: we can do better here and extract the inner type of array
        TsType::TsArrayType(TsArrayType { elem_type, .. }) => {
            (Typ::List(Box::new(tstype_to_typ(&**elem_type).0)), false)
        }
        TsType::TsLitType(TsLitType { lit: TsLit::Str(Str { value, .. }), .. }) => {
            (Typ::Str(Some(vec![value.to_string()])), false)
        }
        TsType::TsOptionalType(TsOptionalType { type_ann, .. }) => {
            (tstype_to_typ(type_ann).0, true)
        }
        TsType::TsUnionOrIntersectionType(TsUnionOrIntersectionType::TsUnionType(
            TsUnionType { types, .. },
        )) => {
            let (is_undefined_option, undefined_position) = if types.len() == 2 {
                (true, find_undefined(types))
            } else if types.into_iter().all(|x| {
                x.as_ts_lit_type().is_some_and(|y| y.lit.as_str().is_some())
                    || x.as_ts_keyword_type().is_some_and(|y| {
                        y.kind == TsKeywordTypeKind::TsUndefinedKeyword
                            || y.kind == TsKeywordTypeKind::TsStringKeyword
                            || y.kind == TsKeywordTypeKind::TsNullKeyword
                    })
            }) {
                (false, find_undefined(types))
            } else {
                (false, None)
            };

            if is_undefined_option && undefined_position.is_some() {
                let other_p = if undefined_position.unwrap() == 0 {
                    1
                } else {
                    0
                };
                (tstype_to_typ(&types[other_p]).0, true)
            } else {
                if types.len() > 1 {
                    let one_of_values: Vec<OneOfVariant> =
                        types.into_iter().map_while(parse_one_of_type).collect();

                    if one_of_values.len() == types.len() {
                        return (Typ::OneOf(one_of_values), false);
                    }
                }

                let literals = types
                    .into_iter()
                    .filter(|x| match ***x {
                        TsType::TsKeywordType(TsKeywordType { kind, .. }) => {
                            kind != TsKeywordTypeKind::TsStringKeyword
                                && kind != TsKeywordTypeKind::TsUndefinedKeyword
                                && kind != TsKeywordTypeKind::TsNullKeyword
                        }
                        _ => true,
                    })
                    .map(|x| match &**x {
                        TsType::TsLitType(TsLitType {
                            lit: TsLit::Str(Str { value, .. }), ..
                        }) => Some(value.to_string()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if literals.iter().find(|x| x.is_none()).is_some() {
                    (Typ::Unknown, false)
                } else {
                    (
                        Typ::Str(Some(literals.into_iter().filter_map(|x| x).collect())),
                        undefined_position.is_some(),
                    )
                }
            }
        }
        TsType::TsTypeRef(TsTypeRef { type_name, type_params, .. }) => {
            let sym = match type_name {
                TsEntityName::Ident(Ident { sym, .. }) => sym,
                TsEntityName::TsQualifiedName(p) => &*p.right.sym,
            };
            match sym.to_string().as_str() {
                "Resource" => (
                    Typ::Resource(
                        type_params
                            .as_ref()
                            .and_then(|x| {
                                x.params.get(0).and_then(|y| {
                                    y.as_ts_lit_type().and_then(|z| {
                                        z.lit.as_str().map(|a| a.to_owned().value.to_string())
                                    })
                                })
                            })
                            .unwrap_or_else(|| "unknown".to_string()),
                    ),
                    false,
                ),
                "Date" => (Typ::Datetime, false),
                "Base64" => (Typ::Bytes, false),
                "Email" => (Typ::Email, false),
                "Sql" => (Typ::Sql, false),
                x @ _ if x.starts_with("DynSelect_") => (
                    Typ::DynSelect(x.strip_prefix("DynSelect_").unwrap().to_string()),
                    false,
                ),
                x @ _ => (Typ::Resource(to_snake_case(x)), false),
            }
        }
        _ => (Typ::Unknown, false),
    }
}

fn parse_one_of_type(x: &Box<TsType>) -> Option<OneOfVariant> {
    match &**x {
        TsType::TsTypeLit(TsTypeLit { members, .. }) => {
            let label = one_of_label(members)?;
            let properties = one_of_properties(members);
            Some(OneOfVariant { label, properties })
        }
        _ => None,
    }
}

fn one_of_label(members: &Vec<TsTypeElement>) -> Option<String> {
    members.iter().find_map(|y| {
        let TsTypeElement::TsPropertySignature(TsPropertySignature { key, type_ann, .. }) = y
        else {
            return None;
        };

        let Expr::Ident(Ident { sym, .. }) = &**key else {
            return None;
        };
        if sym != "label" && sym != "kind" {
            return None;
        }

        let Some(type_ann) = type_ann.as_ref() else {
            return None;
        };
        let TsType::TsLitType(TsLitType { lit: TsLit::Str(Str { value, .. }), .. }) =
            &*type_ann.type_ann
        else {
            return None;
        };

        Some(value.to_string())
    })
}

fn one_of_properties(members: &Vec<TsTypeElement>) -> Vec<ObjectProperty> {
    members
        .iter()
        .filter_map(|x| {
            let TsTypeElement::TsPropertySignature(TsPropertySignature { key, type_ann, .. }) = x
            else {
                return None;
            };

            let Expr::Ident(Ident { sym, .. }) = *key.to_owned() else {
                return None;
            };
            let typ = type_ann
                .as_ref()
                .map(|typ| Box::new(tstype_to_typ(&*typ.type_ann).0))
                .unwrap_or(Box::new(Typ::Unknown));

            Some(ObjectProperty { key: sym.to_string(), typ })
        })
        .collect()
}

fn find_undefined(types: &Vec<Box<TsType>>) -> Option<usize> {
    types.into_iter().position(|x| match **x {
        TsType::TsKeywordType(TsKeywordType { kind, .. }) => {
            kind == TsKeywordTypeKind::TsUndefinedKeyword
                || kind == TsKeywordTypeKind::TsNullKeyword
        }
        _ => false,
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    pub fn eval(s: &str) -> JsValue;
    pub fn alert(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

}

#[cfg(target_arch = "wasm32")]
pub fn eval_sync(code: &str) -> Result<serde_json::Value, String> {
    serde_wasm_bindgen::from_value(eval(format!("let x = {}; x", code).as_str()))
        .map_err(|err| format!("Cannot deserialize value: {:?}", err))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn eval_sync(_code: &str) -> Result<serde_json::Value, String> {
    panic!("eval_sync is only available in wasm32")
}
