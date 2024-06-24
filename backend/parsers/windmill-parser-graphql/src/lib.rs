#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;
use regex::Regex;
use serde_json::json;

use windmill_parser::{Arg, MainArgSignature, Typ};

pub fn parse_graphql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    parse_graphql_file(&code)
        .map(|args| MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args,
            no_main_func: None,
        })
        .ok_or_else(|| anyhow!("Error parsing sql".to_string()))
}

lazy_static::lazy_static! {
    static ref RE_ARG_GRAPHQL: Regex = Regex::new(r#"\$(\w+)\s*:\s*(?:(\w+)!?|\[(\w+)!?\])!?\s*(?:=\s*(\w+)\s*)?"#).unwrap();
}

fn parse_graphql_file(code: &str) -> Option<Vec<Arg>> {
    RE_ARG_GRAPHQL
        .captures_iter(code)
        .filter_map(|cap| Some(parse_arg_cap(cap)))
        .collect()
}
fn parse_arg_cap(cap: regex::Captures) -> Option<Arg> {
    let mut typ = cap.get(2).map(|x| x.as_str().to_string());
    let parsed_typ = match typ {
        Some(ref t) => parse_graphql_typ(t),
        None => {
            typ = cap.get(3).map(|x| format!("[{}]", x.as_str()));
            Typ::List(Box::new(parse_graphql_typ(typ.as_ref().unwrap())))
        }
    };

    let default = cap.get(4).map(|x| x.as_str().to_string());
    let has_default = default.is_some();

    let parsed_default = default.and_then(|x| match parsed_typ {
        Typ::Int => x.parse::<i64>().ok().map(|x| json!(x)),
        Typ::Float => x.parse::<f64>().ok().map(|x| json!(x)),
        _ => Some(json!(x)),
    });

    Some(Arg {
        name: cap
            .get(1)
            .map(|x| x.as_str().to_string())
            .expect("Failed to capture 'name'"),
        typ: parsed_typ,
        default: parsed_default,
        otyp: typ,
        has_default,
    })
}

pub fn parse_graphql_typ(typ: &str) -> Typ {
    match typ {
        "String" | "ID" => Typ::Str(None),
        "Int" => Typ::Int,
        "Boolean" => Typ::Bool,
        "Float" => Typ::Float,
        _ => Typ::Object(vec![]),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_graphql_sig() -> anyhow::Result<()> {
        let code = r#"
query($s: String, $arr: [String]) {
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
                        otyp: Some("String".to_string()),
                        name: "s".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: Some("[String]".to_string()),
                        name: "arr".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: None,
                        has_default: false
                    },
                ],
                no_main_func: None
            }
        );

        Ok(())
    }
}
