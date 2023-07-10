#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;
use regex::Regex;
use serde_json::json;

use std::collections::HashMap;
use windmill_parser::{Arg, MainArgSignature, Typ};

pub fn parse_mysql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_pg_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

pub fn parse_pgsql_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_pg_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args })
    } else {
        Err(anyhow!("Error parsing sql".to_string()))
    }
}

lazy_static::lazy_static! {
    static ref RE_CODE_PGSQL: Regex = Regex::new(r#"(?m)\$(\d+)(::(\w+))?"#).unwrap();
    static ref RE_ARG_PGSQL: Regex = Regex::new(r#"(?m)^--\s+\$(\d+)\s+(\w+)\s+\((\w+)\)(?:\s*\:\s*(.*))?(\=)$"#).unwrap();

}

fn parse_pg_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut hm: HashMap<i32, String> = HashMap::new();
    for cap in RE_CODE_PGSQL.captures_iter(code) {
        hm.insert(
            cap.get(1)
                .and_then(|x| x.as_str().parse::<i32>().ok())
                .ok_or_else(|| anyhow!("Impossible to parse arg digit"))?,
            cap.get(2)
                .map(|x| x.as_str().to_string())
                .unwrap_or_else(|| "text".to_string()),
        );
    }
    let mut args = vec![];
    for i in 1..20 {
        if hm.contains_key(&i) {
            let typ = hm.get(&i).unwrap().to_lowercase();
            args.push(Arg {
                name: format!("${}", i),
                typ: parse_pg_typ(typ.as_str()),
                default: None,
                otyp: Some(typ),
                has_default: false,
            });
        } else {
            break;
        }
    }
    for cap in RE_ARG_PGSQL.captures_iter(code) {
        let i = cap
            .get(1)
            .and_then(|x| x.as_str().parse::<i32>().ok())
            .ok_or_else(|| anyhow!("Impossible to parse arg digit"))?;
        let name = cap.get(2).map(|x| x.as_str().to_string()).unwrap();
        let typ = cap.get(3).map(|x| x.as_str().to_string()).unwrap();
        let default = cap.get(4).map(|x| x.as_str().to_string());
        let has_default = default.is_some();
        let parsed_typ = parse_pg_typ(typ.as_str());
        let parsed_default = default.and_then(|x| match parsed_typ {
            Typ::Int => x.parse::<i64>().ok().map(|x| json!(x)),
            Typ::Float => x.parse::<f64>().ok().map(|x| json!(x)),
            _ => Some(json!(x)),
        });
        args.push(Arg {
            name,
            typ: parsed_typ,
            default: parsed_default,
            otyp: Some(typ),
            has_default,
        });
    }

    Ok(Some(args))
}

pub fn parse_pg_typ(typ: &str) -> Typ {
    match typ {
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
    }
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
            parse_pgsql_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: Some("text".to_string()),
                        name: "$1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: Some("bigint".to_string()),
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
