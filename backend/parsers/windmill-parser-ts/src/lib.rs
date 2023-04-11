/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */
use deno_core::{serde_v8, v8, JsRuntime, RuntimeOptions};
use serde_json::Value;
use windmill_common::error;
use windmill_parser::{json_to_typ, Arg, MainArgSignature, ObjectProperty, Typ};

use swc_common::{sync::Lrc, FileName, SourceMap, SourceMapper, Span, Spanned};
use swc_ecma_ast::{
    ArrayLit, AssignPat, BigInt, BindingIdent, Bool, Decl, ExportDecl, Expr, FnDecl, Ident, Lit,
    ModuleDecl, ModuleItem, Number, ObjectLit, Param, Pat, Str, TsArrayType, TsEntityName,
    TsKeywordType, TsKeywordTypeKind, TsLit, TsLitType, TsOptionalType, TsPropertySignature,
    TsType, TsTypeElement, TsTypeLit, TsTypeRef, TsUnionOrIntersectionType, TsUnionType,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

pub fn parse_deno_signature(code: &str, skip_dflt: bool) -> error::Result<MainArgSignature> {
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
        .map_err(|_| {
            error::Error::ExecutionErr(format!(
                "Error while parsing code, it is invalid typescript"
            ))
        })?
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
                .collect::<Result<Vec<Arg>, error::Error>>()?,
        };
        Ok(r)
    } else {
        Err(error::Error::ExecutionErr(
            "main function was not findable (expected to find 'export function main(...)'"
                .to_string(),
        ))
    }
}

fn parse_param(x: Param, cm: &Lrc<SourceMap>, skip_dflt: bool) -> error::Result<Arg> {
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
                    error::Error::ExecutionErr(format!(
                        "parameter syntax unsupported: `{}`",
                        cm.span_to_snippet(left.span())
                            .unwrap_or_else(|_| cm.span_to_string(left.span()))
                    ))
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
        _ => Err(error::Error::ExecutionErr(format!(
            "parameter syntax unsupported: `{}`",
            cm.span_to_snippet(x.span())
                .unwrap_or_else(|_| cm.span_to_string(x.span()))
        ))),
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
                "Base64" => (Typ::Bytes, false),
                "Email" => (Typ::Email, false),
                "Sql" => (Typ::Sql, false),
                _ => (Typ::Unknown, false),
            }
        }
        _ => (Typ::Unknown, false),
    }
}

pub fn eval_sync(code: &str) -> Result<serde_json::Value, String> {
    let mut context = JsRuntime::new(RuntimeOptions::default());
    let code = format!("let x = {}; x", code);
    let res = context.execute_script("<anon>", code);
    match res {
        Ok(global) => {
            let scope = &mut context.handle_scope();
            let local = v8::Local::new(scope, global);
            let deserialized_value = serde_v8::from_v8::<serde_json::Value>(scope, local);

            match deserialized_value {
                Ok(value) => Ok(value),
                Err(err) => Err(format!("Cannot deserialize value: {:?}", err)),
            }
        }
        Err(err) => Err(format!("Evaling error: {:?}", err)),
    }
}

#[cfg(test)]
mod tests {

    use serde_json::json;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse_deno_sig() -> anyhow::Result<()> {
        let code = "
export function main(test1?: string, test2: string = \"burkina\",
    test3: wmill.Resource<'postgres'>, b64: Base64, ls: Base64[], 
    email: Email, literal: \"test\", literal_union: \"test\" | \"test2\",
    opt_type?: string | null, opt_type_union: string | null, opt_type_union_union2: string | undefined,
    min_object: {a: string, b: number}) {
    console.log(42)
}
";
        assert_eq!(
            parse_deno_signature(code, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "test2".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("burkina")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "test3".to_string(),
                        typ: Typ::Resource("postgres".to_string()),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "b64".to_string(),
                        typ: Typ::Bytes,
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "ls".to_string(),
                        typ: Typ::List(Box::new(Typ::Bytes)),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "email".to_string(),
                        typ: Typ::Email,
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "literal".to_string(),
                        typ: Typ::Str(Some(vec!["test".to_string()])),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "literal_union".to_string(),
                        typ: Typ::Str(Some(vec!["test".to_string(), "test2".to_string()])),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "opt_type".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "opt_type_union".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "opt_type_union_union2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "min_object".to_string(),
                        typ: Typ::Object(vec![
                            ObjectProperty { key: "a".to_string(), typ: Box::new(Typ::Str(None)) },
                            ObjectProperty { key: "b".to_string(), typ: Box::new(Typ::Float) }
                        ]),
                        default: None,
                        has_default: false
                    }
                ]
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_deno_sig_implicit_types() -> anyhow::Result<()> {
        let code = "
export function main(test2 = \"burkina\",
    bool = true,
    float = 4.2,
    int = 42,
    ls = [\"test\"],
    min_object = {a: \"test\", b: 42}) {
    console.log(42)
}
";
        assert_eq!(
            parse_deno_signature(code, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test2".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("burkina")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "bool".to_string(),
                        typ: Typ::Bool,
                        default: Some(json!(true)),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "float".to_string(),
                        typ: Typ::Float,
                        default: Some(json!(4.2)),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "int".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(42)),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "ls".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: Some(json!(["test"])),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "min_object".to_string(),
                        typ: Typ::Object(vec![
                            ObjectProperty { key: "a".to_string(), typ: Box::new(Typ::Str(None)) },
                            ObjectProperty { key: "b".to_string(), typ: Box::new(Typ::Int) }
                        ]),
                        default: Some(json!({"a": "test", "b": 42})),
                        has_default: true
                    }
                ]
            }
        );

        Ok(())
    }
}
