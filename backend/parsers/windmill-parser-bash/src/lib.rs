#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use anyhow::anyhow;

#[cfg(not(target_arch = "wasm32"))]
use regex::Regex;
#[cfg(target_arch = "wasm32")]
use regex_lite::Regex;

use serde_json::json;

use std::{collections::HashMap, str::FromStr};
use windmill_parser::{Arg, MainArgSignature, ObjectType, Typ};

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
    static ref RE_BASH: Regex = Regex::new(r#"(?m)^(\w+)="\$(?:(\d+)|\{(\d+)\}|\{(\d+):-(.*)\})"(?:[\t ]*)?(?:#.*)?\r?$"#).unwrap();

    static ref RE_POWERSHELL_ARGS: Regex = Regex::new(r#"(?:\[([\w\[\]]+)\])?\$(\w+)[\t ]*(?:=[\t ]*(?:(?:(?:"|')([^"\n\r\$]*)(?:"|'))|([\d.]+)))?\r?"#).unwrap();
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

/// Extract a PowerShell param() block, handling nested parentheses.
///
/// # Arguments
/// * `code` - The PowerShell code to extract from
/// * `include_keyword` - If true, returns the full block including "param(...)"
///                       If false, returns only the contents between the parentheses
///
/// # Returns
/// The extracted param block or contents, or None if not found.
pub fn extract_powershell_param_block(code: &str, include_keyword: bool) -> Option<&str> {
    // Find "param" keyword (case-insensitive)
    let lower_code = code.to_lowercase();
    let param_start = lower_code.find("param")?;

    // Skip whitespace and tabs after "param"
    let mut chars = code[param_start + 5..].char_indices();
    let mut paren_offset = param_start + 5;

    // Skip whitespace to find opening paren
    while let Some((idx, ch)) = chars.next() {
        if ch == '(' {
            paren_offset += idx;
            break;
        } else if !ch.is_whitespace() && ch != '\t' {
            // Found non-whitespace, non-paren character - not a valid param block
            return None;
        }
    }

    // Now parse from the opening parenthesis
    let remaining = &code[paren_offset..];
    let mut chars = remaining.char_indices();

    // Skip the opening '('
    if let Some((_, ch)) = chars.next() {
        if ch != '(' {
            return None;
        }
    } else {
        return None;
    }

    let mut depth = 1;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;
    let content_start = paren_offset + 1; // Start after the opening '('

    for (idx, ch) in chars {
        if escape_next {
            escape_next = false;
            continue;
        }

        match ch {
            '`' if in_single_quote || in_double_quote => {
                // PowerShell escape character
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            '(' if !in_single_quote && !in_double_quote => {
                depth += 1;
            }
            ')' if !in_single_quote && !in_double_quote => {
                depth -= 1;
                if depth == 0 {
                    // Found the matching closing parenthesis
                    // idx is the position of ')' relative to paren_offset
                    if include_keyword {
                        // Return full block including "param" keyword and closing paren
                        return Some(&code[param_start..paren_offset + idx + 1]);
                    } else {
                        // Return only contents between parentheses
                        return Some(&code[content_start..paren_offset + idx]);
                    }
                }
            }
            _ => {}
        }
    }

    None
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

fn parse_powershell_single_typ(typ: &str) -> Typ {
    match typ.to_lowercase().as_str() {
        "string" => Typ::Str(None),
        "int" | "long" => Typ::Int,
        "decimal" | "double" | "single" => Typ::Float,
        "datetime" => Typ::Datetime,
        "bool" => Typ::Bool,
        "pscustomobject" => Typ::Object(ObjectType::new(None, None)),
        _ => Typ::Str(None),
    }
}

fn parse_powershell_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let param_wrapper = extract_powershell_param_block(code, false);
    let mut args = vec![];
    if let Some(param_wrapper) = param_wrapper {
        let params = split_pwsh_args(param_wrapper);
        for param in params {
            if let Some(cap) = RE_POWERSHELL_ARGS.captures(param) {
                let typ = cap.get(1).map(|x| x.as_str().to_string());
                let name = cap.get(2).unwrap().as_str().to_string();

                let mut parsed_typ = if let Some(typ) = typ {
                    if typ.as_str().ends_with("[]") {
                        Some(Typ::List(Box::new(parse_powershell_single_typ(
                            typ.as_str().strip_suffix("[]").unwrap(),
                        ))))
                    } else {
                        Some(parse_powershell_single_typ(typ.as_str()))
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
        let code = r#"param($Msg, [string]$Msg2, $Dflt = "default value, with comma", [int]$Nb = 3 , $Nb2 = 5.0, $Nb3 = 5, $Wahoo = $env:WAHOO, [PSCustomObject]$Obj, [string[]]$Arr)"#;
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
                    },
                    Arg {
                        otyp: None,
                        name: "Obj".to_string(),
                        typ: Typ::Object(ObjectType::new(None, None)),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "Arr".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
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

    #[test]
    fn test_parse_powershell_sig_with_nested_parens() -> anyhow::Result<()> {
        // Test with nested parentheses in default values
        let code = r#"param($Msg, [string]$Func = (Get-Date), $Complex = (Get-Item("file.txt")), $Nested = ((1 + 2) * 3))"#;
        let result = parse_powershell_sig(code)?;

        // Should parse 4 parameters without errors
        assert_eq!(result.args.len(), 4);
        assert_eq!(result.args[0].name, "Msg");
        assert_eq!(result.args[1].name, "Func");
        assert_eq!(result.args[2].name, "Complex");
        assert_eq!(result.args[3].name, "Nested");

        Ok(())
    }

    #[test]
    fn test_parse_powershell_sig_with_quoted_parens() -> anyhow::Result<()> {
        // Test with parentheses inside quotes
        let code = r#"param($Msg = "test (with parens)", $Msg2 = 'also (with parens)')"#;
        let result = parse_powershell_sig(code)?;

        // Should parse 2 parameters
        assert_eq!(result.args.len(), 2);
        assert_eq!(result.args[0].name, "Msg");
        assert_eq!(result.args[0].default, Some(json!("test (with parens)")));
        assert_eq!(result.args[1].name, "Msg2");
        assert_eq!(result.args[1].default, Some(json!("also (with parens)")));

        Ok(())
    }

    #[test]
    fn test_extract_powershell_param_block() {
        // Test basic param block - contents only
        let code = "param($x, $y)";
        assert_eq!(extract_powershell_param_block(code, false), Some("$x, $y"));

        // Test basic param block - full block
        assert_eq!(extract_powershell_param_block(code, true), Some("param($x, $y)"));

        // Test with nested parentheses - contents only
        let code = "param($x = Get-Func(1, 2), $y = (1 + 2))";
        assert_eq!(extract_powershell_param_block(code, false), Some("$x = Get-Func(1, 2), $y = (1 + 2)"));

        // Test with nested parentheses - full block
        assert_eq!(extract_powershell_param_block(code, true), Some("param($x = Get-Func(1, 2), $y = (1 + 2))"));

        // Test with whitespace - contents only
        let code = "param  \t  ($x, $y)";
        assert_eq!(extract_powershell_param_block(code, false), Some("$x, $y"));

        // Test with whitespace - full block
        assert_eq!(extract_powershell_param_block(code, true), Some("param  \t  ($x, $y)"));

        // Test with parentheses in quotes - contents only
        let code = r#"param($x = "test (parens)")"#;
        assert_eq!(extract_powershell_param_block(code, false), Some(r#"$x = "test (parens)""#));

        // Test with parentheses in quotes - full block
        assert_eq!(extract_powershell_param_block(code, true), Some(r#"param($x = "test (parens)")"#));

        // Test with code after param block - full block
        let code = "param($x)\nWrite-Host $x";
        assert_eq!(extract_powershell_param_block(code, true), Some("param($x)"));
    }

    #[test]
    fn test_parse_bash_sig_with_crlf() -> anyhow::Result<()> {
        // Test with CRLF line endings (Windows-style)
        let code = "\r\ntoken=\"$1\"\r\nimage=\"$2\"\r\ndigest=\"${3:-latest with spaces}\"\r\ntext=\"$4\" # with comment\r\nnon_required=\"${5:-}\"\r\n\r\n\r\n";
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
}
