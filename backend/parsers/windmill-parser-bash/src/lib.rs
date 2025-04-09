#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;

#[cfg(not(target_arch = "wasm32"))]
use regex::Regex;
#[cfg(target_arch = "wasm32")]
use regex_lite::Regex;

use serde_json::json;

use std::{collections::HashMap, str::FromStr};
use windmill_parser::{Arg, MainArgSignature, Typ};

pub fn parse_bash_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_bash_file(&code)?;
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
        Err(anyhow!("Error parsing bash script".to_string()))
    }
}

pub fn parse_powershell_sig(code: &str) -> anyhow::Result<MainArgSignature> {
    let parsed = parse_powershell_file(&code)?;
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
        Err(anyhow!("Error parsing powershell script".to_string()))
    }
}

lazy_static::lazy_static! {
    static ref RE_BASH: Regex = Regex::new(r#"(?m)^(\w+)="\$(?:(\d+)|\{(\d+)\}|\{(\d+):-(.*)\})"(?:[\t ]*)?(?:#.*)?$"#).unwrap();

    pub static ref RE_POWERSHELL_PARAM: Regex = Regex::new(r#"(?m)param[\t ]*\(([^)]*)\)"#).unwrap();
    static ref RE_POWERSHELL_ARGS: Regex = Regex::new(r#"(?:\[(\w+)\])?\$(\w+)[\t ]*(?:=[\t ]*(?:(?:(?:"|')([^"\n\r\$]*)(?:"|'))|([\d.]+)))?"#).unwrap();
}

fn parse_bash_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let mut hm: HashMap<i32, (String, Option<String>)> = HashMap::new();
    for cap in RE_BASH.captures_iter(code) {
        hm.insert(
            cap.get(2)
                .or(cap.get(3))
                .or(cap.get(4))
                .and_then(|x| x.as_str().parse::<i32>().ok())
                .ok_or_else(|| anyhow!("Impossible to parse arg digit"))?,
            (
                cap[1].to_string(),
                cap.get(5).map(|x| x.as_str().to_string()),
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
                oidx: None,
            });
        } else {
            break;
        }
    }
    Ok(Some(args))
}

enum ParserState {
    Normal,
    InSingleQuote,
    InDoubleQuote,
}
fn split_pwsh_args(code: &str) -> Vec<&str> {
    let mut chars = code.char_indices().peekable();
    let mut state = ParserState::Normal;
    let mut splits = vec![];
    let mut last_idx = 0;
    while let Some((idx, char)) = chars.next() {
        match (&state, char) {
            (ParserState::Normal, '\'') => {
                state = ParserState::InSingleQuote;
            }
            (ParserState::Normal, '"') => {
                state = ParserState::InDoubleQuote;
            }
            (ParserState::InSingleQuote, '\'') => {
                state = ParserState::Normal;
            }
            (ParserState::InDoubleQuote, '"') => {
                state = ParserState::Normal;
            }
            (ParserState::Normal, ',') => {
                splits.push(&code[last_idx..idx]);
                last_idx = idx + 1; // skip the comma
            }
            _ => {}
        }
    }

    if last_idx < code.len() {
        splits.push(&code[last_idx..]);
    }

    splits
}

fn parse_powershell_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let param_wrapper = RE_POWERSHELL_PARAM.captures(code);
    let mut args = vec![];
    if let Some(param_wrapper) = param_wrapper {
        let param_wrapper = param_wrapper.get(1).unwrap().as_str();
        let params = split_pwsh_args(param_wrapper);
        for param in params {
            if let Some(cap) = RE_POWERSHELL_ARGS.captures(param) {
                let typ = cap.get(1).map(|x| x.as_str().to_string());
                let name = cap.get(2).unwrap().as_str().to_string();

                let mut parsed_typ = if let Some(typ) = typ {
                    match typ.as_str() {
                        "string" => Some(Typ::Str(None)),
                        "int" | "long" => Some(Typ::Int),
                        "decimal" | "double" | "single" => Some(Typ::Float),
                        "datetime" | "DateTime" => Some(Typ::Datetime),
                        _ => None,
                    }
                } else {
                    None
                };

                let default = if let Some(x) = cap.get(3) {
                    Some(json!(x.as_str().to_string()))
                } else if let Some(x) = cap.get(4) {
                    if parsed_typ.is_none() {
                        if x.as_str().parse::<i64>().is_ok() {
                            parsed_typ = Some(Typ::Int);
                        } else if x.as_str().parse::<f64>().is_ok() {
                            parsed_typ = Some(Typ::Float);
                        }
                    }
                    serde_json::Number::from_str(x.as_str())
                        .ok()
                        .map(serde_json::Value::Number)
                } else {
                    None
                };

                args.push(Arg {
                    name: name,
                    typ: parsed_typ.unwrap_or(Typ::Str(None)),
                    default: default.clone(),
                    otyp: None,
                    has_default: default.is_some(),
                    oidx: None,
                });
            }
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
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "image".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "digest".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("latest with spaces")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "text".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "non_required".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("")),
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

    #[test]
    fn test_parse_powershell_sig() -> anyhow::Result<()> {
        let code = r#"param($Msg, [string]$Msg2, $Dflt = "default value, with comma", [int]$Nb = 3 , $Nb2 = 5.0, $Nb3 = 5, $Wahoo = $env:WAHOO)"#;
        assert_eq!(
            parse_powershell_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "Msg".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "Msg2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "Dflt".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("default value, with comma")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "Nb".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(3)),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "Nb2".to_string(),
                        typ: Typ::Float,
                        default: Some(json!(5.0)),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "Nb3".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(5)),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "Wahoo".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
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
