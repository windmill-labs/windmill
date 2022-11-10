#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use regex::Regex;

use std::collections::HashMap;
use windmill_parser::{Arg, MainArgSignature, Typ};

pub fn parse_bash_sig(code: &str) -> windmill_common::error::Result<MainArgSignature> {
    let parsed = parse_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args })
    } else {
        Err(windmill_common::error::Error::BadRequest(
            "Error parsing bash script".to_string(),
        ))
    }
}

fn parse_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut hm = HashMap::new();
    let re = Regex::new(r#"(?m)^(\w+)="\$(\d+)"$"#).unwrap();
    for cap in re.captures_iter(code) {
        hm.insert(cap[2].parse::<i32>()?, cap[1].to_string());
    }
    let mut args = vec![];
    for i in 1..20 {
        if hm.contains_key(&i) {
            args.push(Arg {
                name: hm[&i].clone(),
                typ: Typ::Str(None),
                default: None,
                otyp: None,
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
    fn test_parse_bash_sig() -> anyhow::Result<()> {
        let code = r#"
token="$1"
image="$2"
digest="${3:-latest}"
foo="$4"

"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_bash_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "token".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "image".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    }
                ]
            }
        );

        Ok(())
    }
}
