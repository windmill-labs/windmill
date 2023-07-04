#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;
use regex::Regex;

use std::collections::HashMap;
use windmill_parser::{Arg, MainArgSignature, Typ};

pub fn parse_sql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

lazy_static::lazy_static! {
    static ref RE: Regex = Regex::new(r#"(?m)\$(\d+)::(\w+)"#).unwrap();
}

fn parse_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut hm: HashMap<i32, String> = HashMap::new();
    for cap in RE.captures_iter(code) {
        hm.insert(
            cap.get(1)
                .and_then(|x| x.as_str().parse::<i32>().ok())
                .ok_or_else(|| anyhow!("Impossible to parse arg digit"))?,
            cap[2].to_string(),
        );
    }
    let mut args = vec![];
    for i in 1..20 {
        if hm.contains_key(&i) {
            let typ = hm.get(&i).unwrap().to_lowercase();
            args.push(Arg {
                name: format!("${}", i),
                typ: match typ.as_str() {
                    "varchar" => Typ::Str(None),
                    "text" => Typ::Str(None),
                    "int" => Typ::Int,
                    "bigint" => Typ::Int,
                    "bool" => Typ::Bool,
                    "char" => Typ::Str(None),
                    "smallint" => Typ::Int,
                    "smallserial" => Typ::Int,
                    "serial" => Typ::Int,
                    "bigserial" => Typ::Int,
                    "real" => Typ::Float,
                    "double precision" => Typ::Float,
                    "oid" => Typ::Int,
                    _ => Typ::Str(None),
                },
                default: None,
                otyp: Some(typ),
                has_default: false,
            });
        } else {
            break;
        }
    }
    Ok(Some(args))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_sql_sig() -> anyhow::Result<()> {
        let code = r#"
SELECT * FROM table WHERE token=$1::TEXT AND image=$2::BIGINT
"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_sql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "$1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "$2".to_string(),
                        typ: Typ::Int,
                        default: None,
                        has_default: false
                    },
                ]
            }
        );

        Ok(())
    }
}
