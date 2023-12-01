use convert_case::{Case, Casing};
use regex::Regex;
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
use windmill_parser::{json_to_typ, Arg, MainArgSignature, ObjectProperty, Typ};

use swc_common::{sync::Lrc, FileName, SourceMap, SourceMapper, Span, Spanned};
use swc_ecma_ast::{
    ArrayLit, AssignPat, BigInt, BindingIdent, Bool, Decl, ExportDecl, Expr, FnDecl, Ident, Lit,
    MemberExpr, MemberProp, ModuleDecl, ModuleItem, Number, ObjectLit, ObjectPat, Param, Pat, Str,
    TsArrayType, TsEntityName, TsKeywordType, TsKeywordTypeKind, TsLit, TsLitType, TsOptionalType,
    TsParenthesizedType, TsPropertySignature, TsType, TsTypeAnn, TsTypeElement, TsTypeLit,
    TsTypeRef, TsUnionOrIntersectionType, TsUnionType,
};
use swc_ecma_parser::{lexer::Lexer, EsConfig, Parser, StringInput, Syntax, TsConfig};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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
            MemberExpr { obj, prop: MemberProp::Ident(Ident { sym, .. }), .. } => {
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
    let fm = cm.new_source_file(FileName::Custom("main.ts".into()), code.into());
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Es(EsConfig { jsx: false, ..Default::default() }),
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

    let expr = parser
        .parse_module()
        .map_err(|_| anyhow::anyhow!("Error while parsing code, it is invalid TypeScript"))?;

    let mut visitor = OutputFinder { idents: HashSet::new() };
    swc_ecma_visit::visit_module(&mut visitor, &expr);

    Ok(visitor.idents.into_iter().collect())
}

pub fn parse_deno_signature(code: &str, skip_dflt: bool) -> anyhow::Result<MainArgSignature> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("main.ts".into()), code.into());
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Typescript(TsConfig::default()),
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
        .map_err(|_| anyhow::anyhow!("Error while parsing code, it is invalid TypeScript"))?
        .body;

    let params = ast.into_iter().find_map(|x| match x {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            decl: Decl::Fn(FnDecl { ident: Ident { sym, .. }, function, .. }),
            ..
        })) if &sym.to_string() == "main" => Some(function.params),
        _ => None,
    });

    let mut c: u16 = 0;
    if let Some(params) = params {
        let r = MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: params
                .into_iter()
                .map(|x| parse_param(x, &cm, skip_dflt, &mut c))
                .collect::<anyhow::Result<Vec<Arg>>>()?,
        };
        Ok(r)
    } else {
        Err(anyhow::anyhow!(
            "main function was not findable (expected to find 'export function main(...)'"
                .to_string(),
        ))
    }
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
                typ = json_to_typ(dflt.as_ref().unwrap());
            }
            Ok(Arg { otyp: None, name, typ, default: dflt, has_default: true })
        }
        Pat::Object(ObjectPat { type_ann, .. }) => {
            let (typ, nullable) = eval_type_ann(&type_ann);
            *counter += 1;
            let name = format!("anon{}", counter);
            Ok(Arg { otyp: None, name, typ, default: None, has_default: nullable })
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

fn to_snake_case(s: &str) -> String {
    let r = s.to_case(Case::Snake);

    // s_3 => s3
    let re = Regex::new(r"_(\d)").unwrap();
    re.replace_all(&r, "$1").to_string()
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
            if let Some(p) = if types.len() != 2 {
                None
            } else {
                types.into_iter().position(|x| match **x {
                    TsType::TsKeywordType(TsKeywordType { kind, .. }) => {
                        kind == TsKeywordTypeKind::TsUndefinedKeyword
                            || kind == TsKeywordTypeKind::TsNullKeyword
                    }
                    _ => false,
                })
            } {
                let other_p = if p == 0 { 1 } else { 0 };
                (tstype_to_typ(&types[other_p]).0, true)
            } else {
                let literals = types
                    .into_iter()
                    .filter(|x| match ***x {
                        TsType::TsKeywordType(TsKeywordType { kind, .. }) => {
                            kind != TsKeywordTypeKind::TsStringKeyword
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
                        false,
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
                x @ _ => (Typ::Resource(to_snake_case(x)), false),
            }
        }
        _ => (Typ::Unknown, false),
    }
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
