#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;

#[cfg(not(target_arch = "wasm32"))]
use regex::Regex;
#[cfg(target_arch = "wasm32")]
use regex_lite::Regex;

use serde_json::json;

use windmill_parser::{Arg, MainArgSignature, ObjectType, Typ};

pub fn parse_graphql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_graphql_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            no_main_func: None,
            has_preprocessor: None,
        })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

lazy_static::lazy_static! {
    static ref RE_ARG_GRAPHQL: Regex = Regex::new(r#"\$(\w+)\s*:\s*(?:(\w+)(!)?|\[(\w+)!?\])(!)?\s*(?:=\s*"?(\w+)"?\s*)?"#).unwrap();
}

fn parse_graphql_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut args: Vec<Arg> = vec![];

    for cap in RE_ARG_GRAPHQL.captures_iter(code) {
        let name = cap.get(1).map(|x| x.as_str().to_string()).unwrap();
        let mut typ = cap.get(2).map(|x| x.as_str().to_string());

        let parsed_typ = if typ.is_none() {
            let inner_typ = cap.get(4).map(|x| x.as_str().to_string());
            typ = inner_typ.clone().map(|x| format!("[{}]", x.to_string()));
            Typ::List(Box::new(parse_graphql_typ(inner_typ.unwrap().as_str())))
        } else {
            parse_graphql_typ(typ.clone().unwrap().as_str())
        };

        let (has_default, default) = match cap.get(6).map(|x| x.as_str().to_string()) {
            Some(x) => (true, Some(x)), // default value => optional
            None => (cap.get(3).is_none() && cap.get(5).is_none(), None), // optional if no !
        };

        let parsed_default = default.and_then(|x| match parsed_typ {
            Typ::Int => x.parse::<i64>().ok().map(|x| json!(x)),
            Typ::Float => x.parse::<f64>().ok().map(|x| json!(x)),
            _ => Some(json!(x)),
        });
        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(typ.unwrap()),
            has_default,
            oidx: None,
        });
    }

    Ok(Some(args))
}

pub fn parse_graphql_typ(typ: &str) -> Typ {
    match typ {
        "String" | "ID" => Typ::Str(None),
        "Int" => Typ::Int,
        "Boolean" => Typ::Bool,
        "Float" => Typ::Float,
        _ => Typ::Object(ObjectType::new(None, Some(vec![]))),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_graphql_sig() -> anyhow::Result<()> {
        let code = r#"
query($i: Int, $arr: [String]!, $wahoo: String = "wahoo") {
    books {
        title
    }
}        
"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_graphql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("Int".to_string()),
                        name: "i".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("[String]".to_string()),
                        name: "arr".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("String".to_string()),
                        name: "wahoo".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("wahoo")),
                        has_default: true,
                        oidx: None
                    }
                ],
                no_main_func: None,
                has_preprocessor: None
            }
        );

        Ok(())
    }
}
