#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;
use regex::Regex;
use serde_json::json;

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

lazy_static::lazy_static! {
    static ref RE: Regex = Regex::new(r#"(?m)^(\w+)="\$(?:(\d+)|\{(\d+):-(.*)\})"$"#).unwrap();
}

fn parse_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut hm: HashMap<i32, (String, Option<String>)> = HashMap::new();
    for cap in RE.captures_iter(code) {
        hm.insert(
            cap.get(2)
                .or(cap.get(3))
                .and_then(|x| x.as_str().parse::<i32>().ok())
                .ok_or_else(|| anyhow!("Impossible to parse arg digit"))?,
            (
                cap[1].to_string(),
                cap.get(4).map(|x| x.as_str().to_string()),
            ),
        );
    }
    let mut args = vec![];
    for i in 1..20 {
        if hm.contains_key(&i) {
            let (name, default) = hm.get(&i).unwrap();
            args.push(Arg {
                name: name.clone(),
                typ: Typ::Str(None),
                default: default.clone().map(|x| json!(x)),
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

    use serde_json::json;

    use super::*;

    #[test]
    fn test_parse_bash_sig() -> anyhow::Result<()> {
        let code = r#"
token="$1"
image="$2"
digest="${3:-latest with spaces}"

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
                    },
                    Arg {
                        otyp: None,
                        name: "digest".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("latest with spaces")),
                        has_default: false
                    }
                ]
            }
        );

        Ok(())
    }
}
