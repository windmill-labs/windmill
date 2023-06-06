/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
// use deno_core::{serde_v8, v8, JsRuntime, RuntimeOptions};
use serde_json::Value;
use windmill_parser::{json_to_typ, Arg, MainArgSignature, ObjectProperty, Typ};

use swc_common::{sync::Lrc, FileName, SourceMap, SourceMapper, Span, Spanned};
use swc_ecma_ast::{
    ArrayLit, AssignPat, BigInt, BindingIdent, Bool, Decl, ExportDecl, Expr, FnDecl, Ident, Lit,
    ModuleDecl, ModuleItem, Number, ObjectLit, Param, Pat, Str, TsArrayType, TsEntityName,
    TsKeywordType, TsKeywordTypeKind, TsLit, TsLitType, TsOptionalType, TsPropertySignature,
    TsType, TsTypeElement, TsTypeLit, TsTypeRef, TsUnionOrIntersectionType, TsUnionType,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub fn parse_deno_signature(code: &str, skip_dflt: bool) -> Result<MainArgSignature, String> {
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
        .map_err(|_| format!("Error while parsing code, it is invalid typescript"))?
        .body;

    // println!("{ast:?}");
    let params = ast.into_iter().find_map(|x| match x {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
            decl: Decl::Fn(FnDecl { ident: Ident { sym, .. }, function, .. }),
            ..
        })) if &sym.to_string() == "main" => Some(function.params),
        _ => None,
    });

    if let Some(params) = params {
        let r = MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: params
                .into_iter()
                .map(|x| parse_param(x, &cm, skip_dflt))
                .collect::<Result<Vec<Arg>, String>>()?,
        };
        Ok(r)
    } else {
        Err(
            "main function was not findable (expected to find 'export function main(...)'"
                .to_string(),
        )
    }
}

fn parse_param(x: Param, cm: &Lrc<SourceMap>, skip_dflt: bool) -> Result<Arg, String> {
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
        Pat::Assign(AssignPat { left, right, .. }) => {
            let (name, mut typ, _nullable) =
                left.as_ident().map(binding_ident_to_arg).ok_or_else(|| {
                    format!(
                        "parameter syntax unsupported: `{}`",
                        cm.span_to_snippet(left.span())
                            .unwrap_or_else(|_| cm.span_to_string(left.span()))
                    )
                })?;

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
        _ => Err(format!(
            "parameter syntax unsupported: `{}`",
            cm.span_to_snippet(x.span())
                .unwrap_or_else(|_| cm.span_to_string(x.span()))
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

fn binding_ident_to_arg(BindingIdent { id, type_ann }: &BindingIdent) -> (String, Typ, bool) {
    let (typ, nullable) = type_ann
        .as_ref()
        .map(|x| tstype_to_typ(&*x.type_ann))
        .unwrap_or((Typ::Unknown, false));
    (id.sym.to_string(), typ, nullable)
}

fn tstype_to_typ(ts_type: &TsType) -> (Typ, bool) {
    //println!("{:?}", ts_type);
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
                _ => (Typ::Unknown, false),
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

}

#[cfg(target_arch = "wasm32")]
pub fn eval_sync(code: &str) -> Result<serde_json::Value, String> {
    serde_wasm_bindgen::from_value(eval(format!("let x = {}; x", code).as_str()))
        .map_err(|err| format!("Cannot deserialize value: {:?}", err))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn eval_sync(code: &str) -> Result<serde_json::Value, String> {
    panic!("eval_sync is only available in wasm32")
}
