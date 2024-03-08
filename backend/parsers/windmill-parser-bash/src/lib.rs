#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;
use regex::Regex;
use serde_json::json;

use std::collections::HashMap;
use windmill_parser::{Arg, MainArgSignature, Typ};

pub fn parse_bash_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_bash_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args })
    } else {
        Err(anyhow!("Error parsing bash script".to_string()))
    }
}

pub fn parse_powershell_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_powershell_file(&code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args })
    } else {
        Err(anyhow!("Error parsing powershell script".to_string()))
    }
}

lazy_static::lazy_static! {
    static ref RE_BASH: Regex = Regex::new(r#"(?m)^(\w+)="\$(?:(\d+)|\{(\d+):-(.*)\})"(?:[\t ]*)?(?:#.*)?$"#).unwrap();

    pub static ref RE_POWERSHELL_PARAM: Regex = Regex::new(r#"(?m)param[\t ]*\(([^)]*)\)"#).unwrap();
    static ref RE_POWERSHELL_ARGS: Regex = Regex::new(r#"(?:\[(\w+)\])?\$(\w+)[\t ]*(?:=[\t ]*(?:(?:(?:"|')([^"\n\r\$]*)(?:"|'))|([\d.]+)))?"#).unwrap();
}

fn parse_bash_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut hm: HashMap<i32, (String, Option<String>)> = HashMap::new();
    for cap in RE_BASH.captures_iter(code) {
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
                has_default: default.is_some(),
            });
        } else {
            break;
        }
    }
    Ok(Some(args))
}

fn parse_powershell_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let param_wrapper = RE_POWERSHELL_PARAM.captures(code);
    let mut args = vec![];
    if let Some(param_wrapper) = param_wrapper {
        let param_wrapper = param_wrapper.get(1).unwrap().as_str();
        for cap in RE_POWERSHELL_ARGS.captures_iter(param_wrapper) {
            let typ = cap
                .get(1)
                .map(|x| x.as_str().to_string())
                .unwrap_or("string".to_string());
            let name = cap.get(2).unwrap().as_str().to_string();
            let default = cap
                .get(3)
                .or(cap.get(4))
                .map(|x| json!(x.as_str().to_string()));

            args.push(Arg {
                name: name,
                typ: match typ.as_str() {
                    "string" => Typ::Str(None),
                    "int" | "long" => Typ::Int,
                    "decimal" | "double" | "single" => Typ::Float,
                    "datetime" | "DateTime" => Typ::Datetime,
                    _ => Typ::Str(None),
                },
                default: default.clone(),
                otyp: None,
                has_default: default.is_some(),
            });
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
text="$4" # with comment
non_required="${5:-}"


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
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "text".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "non_required".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("")),
                        has_default: true
                    }
                ]
            }
        );

        Ok(())
    }
}
