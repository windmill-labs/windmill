use serde_json::Value;
use windmill_parser::{to_snake_case, Arg, MainArgSignature, ObjectType, Typ};

use php_parser_rs::parser::{
    self,
    ast::{
        data_type::Type,
        functions::{FunctionParameterList, FunctionStatement},
        literals::Literal,
        Expression, Statement,
    },
};

fn parse_php_type(e: Type) -> Typ {
    match e {
        Type::Float(_) => Typ::Float,
        Type::Boolean(_) => Typ::Bool,
        Type::Integer(_) => Typ::Int,
        Type::String(_) => Typ::Str(None),
        Type::Array(_) => Typ::List(Box::new(Typ::Str(None))),
        Type::Object(_) => Typ::Object(ObjectType::new(None, Some(vec![]))),
        Type::Named(_, name) => Typ::Resource(to_snake_case(name.to_string().as_ref())),
        _ => Typ::Unknown,
    }
}

fn parse_default_expr(e: Expression) -> Option<Value> {
    match e {
        Expression::Literal(l) => match l {
            Literal::String(s) => Some(Value::String(s.value.to_string())),
            Literal::Integer(i) => match i.value.to_string().parse() {
                Ok(i) => Some(Value::Number(i)),
                Err(_) => None,
            },
            Literal::Float(f) => match f.value.to_string().parse() {
                Ok(i) => Some(Value::Number(i)),
                Err(_) => None,
            },
        },
        Expression::Bool(b) => Some(Value::Bool(b.value)),
        _ => None,
    }
}

pub fn parse_php_signature(
    code: &str,
    override_entrypoint: Option<String>,
) -> anyhow::Result<MainArgSignature> {
    let entrypoint_fn_name = override_entrypoint.unwrap_or("main".to_string());

    let ast = parser::parse(code)
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let mut entrypoint_params = None;
    let mut has_preprocessor = None;
    for node in ast.into_iter() {
        match node {
            Statement::Function(FunctionStatement {
                name,
                parameters: FunctionParameterList { parameters, .. },
                ..
            }) => {
                let fn_name = name.to_string();

                if has_preprocessor.is_none() && fn_name == "preprocessor" {
                    has_preprocessor = Some(true);
                }

                if entrypoint_params.is_none() && fn_name == entrypoint_fn_name {
                    entrypoint_params = Some(parameters);
                }

                if has_preprocessor.is_some() && entrypoint_params.is_some() {
                    break;
                }
            }
            _ => {}
        };
    }

    if let Some(params) = entrypoint_params {
        let args = params
            .into_iter()
            .map(|x| {
                let typ = x.data_type.map_or(Typ::Unknown, |e| parse_php_type(e));
                let default = x.default.map_or(None, |e| parse_default_expr(e));
                Arg {
                    otyp: None,
                    name: x.name.to_string().trim_start_matches('$').to_string(),
                    typ,
                    has_default: default.is_some(),
                    default,
                    oidx: None,
                }
            })
            .collect();

        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            no_main_func: Some(false),
            has_preprocessor,
        })
    } else {
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![],
            no_main_func: Some(true),
            has_preprocessor,
        })
    }
}

#[cfg(test)]
mod tests {

    use serde_json::Number;

    use super::*;

    #[test]
    fn test_parse_php_sig() -> anyhow::Result<()> {
        let code = "
<?php

class Stripe {}

function main(string $input1 = \"hey\", bool $input2 = false, int $input3 = 3, float $input4 = 4.5, Stripe $resource) {
    echo 'hello';
}

";
        assert_eq!(
            parse_php_signature(code, None)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "input1".to_string(),
                        typ: Typ::Str(None),
                        has_default: true,
                        default: Some(Value::String("hey".to_string())),
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "input2".to_string(),
                        typ: Typ::Bool,
                        has_default: true,
                        default: Some(Value::Bool(false)),
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "input3".to_string(),
                        typ: Typ::Int,
                        has_default: true,
                        default: Some(Value::Number(Number::from(3))),
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "input4".to_string(),
                        typ: Typ::Float,
                        has_default: true,
                        default: Some(Value::Number(Number::from_f64(f64::from(4.5)).unwrap())),
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "resource".to_string(),
                        typ: Typ::Resource("stripe".to_string()),
                        has_default: false,
                        default: None,
                        oidx: None
                    }
                ],
                no_main_func: Some(false),
                has_preprocessor: None
            }
        );

        Ok(())
    }
}
