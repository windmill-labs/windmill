/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::{
    error,
    parser::{Arg, InnerTyp, MainArgSignature, Typ},
};

use swc_common::{sync::Lrc, FileName, SourceMap};
use swc_ecma_ast::{
    AssignPat, BindingIdent, Decl, ExportDecl, FnDecl, Ident, ModuleDecl, ModuleItem, Pat, Str,
    TsArrayType, TsEntityName, TsKeywordTypeKind, TsLit, TsLitType, TsType, TsTypeRef,
    TsUnionOrIntersectionType, TsUnionType,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsConfig};

pub fn parse_deno_signature(code: &str) -> error::Result<MainArgSignature> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("test.ts".into()), code.into());
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
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: params
                .into_iter()
                .map(|x| match x.pat {
                    Pat::Ident(ident) => {
                        let (name, typ) = binding_ident_to_arg(&ident)?;
                        Ok(Arg { name, typ, default: None, has_default: ident.id.optional })
                    }
                    Pat::Assign(AssignPat { left, right, .. }) => {
                        let (name, typ) =
                            left.as_ident().map(binding_ident_to_arg).ok_or_else(|| {
                                error::Error::ExecutionErr(format!(
                                    "Arg {left:?} has unexpected syntax"
                                ))
                            })??;
                        Ok(Arg {
                            name,
                            typ,
                            default: serde_json::to_value(right)
                                .map_err(|e| error::Error::ExecutionErr(e.to_string()))?
                                .as_object()
                                .and_then(|x| x.get("value").to_owned())
                                .cloned(),

                            has_default: true,
                        })
                    }
                    _ => Err(error::Error::ExecutionErr(format!(
                        "Arg {x:?} has unexpected syntax"
                    ))),
                })
                .collect::<Result<Vec<Arg>, error::Error>>()?,
        })
    } else {
        Err(error::Error::ExecutionErr(
            "main function was not findable (expected to find 'export main function(...)'"
                .to_string(),
        ))
    }
}

fn binding_ident_to_arg(
    BindingIdent { id, type_ann }: &BindingIdent,
) -> anyhow::Result<(String, Typ)> {
    Ok((
        id.sym.to_string(),
        type_ann
            .as_ref()
            .map(|x| {
                println!("");
                println!("{:?}", id.sym.to_string());
                println!("{:?}", x);
                match &*x.type_ann {
                    TsType::TsKeywordType(t) => match t.kind {
                        TsKeywordTypeKind::TsObjectKeyword => Typ::Dict,
                        TsKeywordTypeKind::TsBooleanKeyword => Typ::Bool,
                        TsKeywordTypeKind::TsBigIntKeyword => Typ::Int,
                        TsKeywordTypeKind::TsNumberKeyword => Typ::Float,
                        TsKeywordTypeKind::TsStringKeyword => Typ::Str(None),
                        _ => Typ::Unknown,
                    },
                    // TODO: we can do better here and extract the inner type of array
                    TsType::TsArrayType(TsArrayType { elem_type, .. }) => {
                        match &**elem_type {
                            TsType::TsTypeRef(TsTypeRef {
                                type_name: TsEntityName::Ident(Ident { sym, .. }),
                                ..
                            }) => match sym.to_string().as_str() {
                                "Base64" => Typ::List(InnerTyp::Bytes),
                                "Email" => Typ::List(InnerTyp::Email),
                                "bigint" => Typ::List(InnerTyp::Int),
                                "number" => Typ::List(InnerTyp::Float),
                                _ => Typ::List(InnerTyp::Str),
                            },
                            //TsType::TsKeywordType(())
                            _ => Typ::List(InnerTyp::Str),
                        }
                    }
                    TsType::TsLitType(TsLitType { lit: TsLit::Str(Str { value, .. }), .. }) => {
                        Typ::Str(Some(vec![value.to_string()]))
                    }
                    TsType::TsUnionOrIntersectionType(TsUnionOrIntersectionType::TsUnionType(
                        TsUnionType { types, .. },
                    )) => {
                        let literals = types
                            .into_iter()
                            .map(|x| match &**x {
                                TsType::TsLitType(TsLitType {
                                    lit: TsLit::Str(Str { value, .. }),
                                    ..
                                }) => Some(value.to_string()),
                                _ => None,
                            })
                            .collect::<Vec<_>>();
                        if literals.iter().find(|x| x.is_none()).is_some() {
                            Typ::Unknown
                        } else {
                            Typ::Str(Some(literals.into_iter().filter_map(|x| x).collect()))
                        }
                    }
                    TsType::TsTypeRef(TsTypeRef { type_name, type_params, .. }) => {
                        let sym = match type_name {
                            TsEntityName::Ident(Ident { sym, .. }) => sym,
                            TsEntityName::TsQualifiedName(p) => &*p.right.sym,
                        };
                        match sym.to_string().as_str() {
                            "Resource" => Typ::Resource(
                                type_params
                                    .as_ref()
                                    .and_then(|x| {
                                        x.params.get(0).and_then(|y| {
                                            y.as_ts_lit_type().and_then(|z| {
                                                z.lit
                                                    .as_str()
                                                    .map(|a| a.to_owned().value.to_string())
                                            })
                                        })
                                    })
                                    .unwrap_or_else(|| "unknown".to_string()),
                            ),
                            "Base64" => Typ::Bytes,
                            "Email" => Typ::Email,
                            "Sql" => Typ::Sql,
                            _ => Typ::Unknown,
                        }
                    }
                    _ => Typ::Unknown,
                }
            })
            .unwrap_or(Typ::Unknown),
    ))
}

#[cfg(test)]
mod tests {

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parse_deno_sig() -> anyhow::Result<()> {
        let code = "

export function main(test1?: string, test2: string = \"burkina\",
    test3: wmill.Resource<'postgres'>, b64: Base64, ls: Base64[], 
    email: Email, literal: \"test\", literal_union: \"test\" | \"test2\") {
    console.log(42)
}

";
        println!("{}", serde_json::to_string(&parse_deno_signature(code)?)?);

        Ok(())
    }
}
