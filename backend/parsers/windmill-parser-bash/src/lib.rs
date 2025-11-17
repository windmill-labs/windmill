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

/// Extract a PowerShell param() block with its preceding attributes, handling nested parentheses.
///
/// # Arguments
/// * `code` - The PowerShell code to extract from
/// * `include_attributes` - If true, includes attributes like [CmdletBinding()] before param
///
/// # Returns
/// A tuple of (param_block_with_attributes, remaining_code) or None if not found.
/// - `param_block_with_attributes`: The param block including attributes/comments if include_attributes is true
/// - `remaining_code`: The rest of the code after the param block
///
/// This function uses the existing extract_powershell_param_block validation, which already
/// ensures that only comments, whitespace, and attributes appear before param. So we can
/// simply return everything from the beginning to the end of the param block.
pub fn extract_powershell_param_block_with_attributes(code: &str, include_attributes: bool) -> Option<(&str, &str)> {
    // First, use the existing function to validate and find the param block
    let param_block = extract_powershell_param_block(code, true)?;

    // Find where the param block ends in the original code
    let param_end_pos = code.find(param_block)? + param_block.len();

    // If include_attributes is true, we start from the beginning (position 0)
    // since we know everything before param is valid (comments/whitespace/attributes)
    // Otherwise, start from where param begins
    if include_attributes {
        Some((&code[..param_end_pos], &code[param_end_pos..]))
    } else {
        Some((param_block, &code[param_end_pos..]))
    }
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
    // Scan through the code looking for "param" while validating everything before it
    let mut chars = code.chars().enumerate().peekable();
    let mut in_block_comment = false;
    let mut in_attribute_bracket = false;
    let mut bracket_depth = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut param_start = None;

    while let Some((idx, ch)) = chars.next() {
        if in_block_comment {
            // Check for end of block comment: #>
            if ch == '#' && chars.peek().map(|(_, c)| c) == Some(&'>') {
                chars.next(); // consume '>'
                in_block_comment = false;
            }
        } else if in_attribute_bracket {
            // Handle quotes inside attributes to avoid counting brackets/parens inside strings
            if in_single_quote {
                if ch == '\'' {
                    in_single_quote = false;
                }
            } else if in_double_quote {
                if ch == '"' {
                    in_double_quote = false;
                }
            } else {
                // Track bracket depth to handle nested brackets/parens in attributes
                match ch {
                    '\'' => in_single_quote = true,
                    '"' => in_double_quote = true,
                    '[' => bracket_depth += 1,
                    ']' => {
                        bracket_depth -= 1;
                        if bracket_depth == 0 {
                            in_attribute_bracket = false;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            match ch {
                // Start of block comment: <#
                '<' if chars.peek().map(|(_, c)| c) == Some(&'#') => {
                    chars.next(); // consume '#'
                    in_block_comment = true;
                }
                // Single-line comment: consume until end of line
                '#' => {
                    for (_, next_ch) in chars.by_ref() {
                        if next_ch == '\n' || next_ch == '\r' {
                            break;
                        }
                    }
                }
                // Start of attribute bracket (e.g., [CmdletBinding()])
                '[' => {
                    in_attribute_bracket = true;
                    bracket_depth = 1;
                }
                // Check if we've found "param" (case-insensitive)
                'p' | 'P' => {
                    // Check if this is the start of "param" keyword
                    let remaining = &code[idx..];
                    if remaining.len() >= 5 {
                        let next_four = &remaining[1..5];
                        if next_four.eq_ignore_ascii_case("aram") {
                            // Check word boundaries
                            let before_ok = idx == 0 || {
                                let before = code.as_bytes()[idx - 1];
                                !before.is_ascii_alphanumeric() && before != b'_'
                            };
                            let after_ok = idx + 5 >= code.len() || {
                                let after = code.as_bytes()[idx + 5];
                                !after.is_ascii_alphanumeric() && after != b'_'
                            };

                            if before_ok && after_ok {
                                param_start = Some(idx);
                                break;
                            }
                        }
                    }
                }
                // Whitespace is allowed
                c if c.is_whitespace() => {}
                // Any other character means there's code before param
                _ => return None,
            }
        }
    }

    // If we're still in a block comment or unclosed attribute bracket, it's invalid
    if in_block_comment || in_attribute_bracket {
        return None;
    }

    let param_start = param_start?;

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
            '`' if in_double_quote => {
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

fn parse_powershell_single_typ(typ: &str) -> Typ {
    match typ.to_lowercase().as_str() {
        "string" => Typ::Str(None),
        "int" | "long" => Typ::Int,
        "decimal" | "double" | "single" => Typ::Float,
        "datetime" => Typ::Datetime,
        "bool" | "switch" => Typ::Bool,
        "pscustomobject" => Typ::Object(ObjectType::new(None, None)),
        _ => Typ::Str(None),
    }
}

/// Parse ValidateSet attribute to extract enum values
/// Example: ValidateSet('Red', 'Green', 'Blue') -> Some(vec!["Red", "Green", "Blue"])
fn parse_validate_set(bracket_content: &str) -> Option<Vec<String>> {
    // Find the opening parenthesis
    let start = bracket_content.find('(')?;
    let end = bracket_content.rfind(')')?;

    if start >= end {
        return None;
    }

    let values_str = &bracket_content[start + 1..end];
    let mut values = Vec::new();
    let mut current_value = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in values_str.chars() {
        if escape_next {
            current_value.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '`' if in_double_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                if in_single_quote {
                    // End of single-quoted string
                    values.push(current_value.clone());
                    current_value.clear();
                    in_single_quote = false;
                } else {
                    // Start of single-quoted string
                    in_single_quote = true;
                }
            }
            '"' if !in_single_quote => {
                if in_double_quote {
                    // End of double-quoted string
                    values.push(current_value.clone());
                    current_value.clear();
                    in_double_quote = false;
                } else {
                    // Start of double-quoted string
                    in_double_quote = true;
                }
            }
            ',' if !in_single_quote && !in_double_quote => {
                // Skip commas outside quotes
                continue;
            }
            c if in_single_quote || in_double_quote => {
                current_value.push(c);
            }
            c if !c.is_whitespace() => {
                // Handle unquoted values (though PowerShell typically requires quotes)
                current_value.push(c);
            }
            _ => {}
        }
    }

    // Handle any remaining unquoted value
    if !current_value.is_empty() {
        values.push(current_value.trim().to_string());
    }

    if values.is_empty() {
        None
    } else {
        Some(values)
    }
}

/// Single-pass PowerShell parameter parser.
/// Parses the content of a param() block and extracts all parameter information.
///
/// This function processes PowerShell parameter declarations in a single pass, handling:
/// - Parameter attributes: [Parameter(Mandatory)], [Parameter(Mandatory=$true)], [ValidateSet(...)], etc.
/// - Type annotations: [string], [int[]], [PSCustomObject], etc.
/// - Variable names: $Name, $Value, etc.
/// - Default values: = 'text', = 25, = $env:VAR, etc.
/// - Mandatory detection: Parameters with Mandatory attribute are marked as required
fn parse_powershell_parameters(content: &str) -> anyhow::Result<Vec<Arg>> {
    #[derive(Debug, PartialEq)]
    enum State {
        Normal,
        InSingleQuote,
        InDoubleQuote,
        InBracket,
    }

    let mut args = Vec::new();
    let mut chars = content.char_indices().peekable();
    let mut state = State::Normal;
    let mut bracket_depth: i32 = 0;
    let mut paren_depth: i32 = 0;

    // Current parameter being built
    let mut type_annotation: Option<String> = None;
    let mut var_name: Option<String> = None;
    let mut default_value: Option<String> = None;
    let mut is_mandatory = false;
    let mut validate_set: Option<Vec<String>> = None;

    // Track position for extracting text
    let mut last_bracket_start = None;
    let mut found_dollar = false;

    while let Some((idx, ch)) = chars.next() {
        match state {
            State::InSingleQuote => {
                if ch == '\'' {
                    state = State::Normal;
                }
            }
            State::InDoubleQuote => {
                if ch == '"' {
                    // Check for escape character
                    if idx > 0 && content.chars().nth(idx - 1) != Some('`') {
                        state = State::Normal;
                    }
                }
            }
            State::InBracket => {
                match ch {
                    '[' => bracket_depth += 1,
                    ']' => {
                        bracket_depth -= 1;
                        if bracket_depth == 0 {
                            // Extract the bracket content
                            if let Some(start) = last_bracket_start {
                                let bracket_content = &content[start + 1..idx];

                                // Check if this is a Parameter attribute with Mandatory (case-insensitive)
                                let lower = bracket_content.to_lowercase();
                                if lower.starts_with("parameter(") || lower.starts_with("parameter ") {
                                    // Check for Mandatory (case-insensitive)
                                    if lower.contains("mandatory") {
                                        // Check if it's explicitly set to false
                                        if !lower.contains("mandatory=$false") && !lower.contains("mandatory = $false") {
                                            is_mandatory = true;
                                        }
                                    }
                                }

                                // Check if this is a ValidateSet attribute
                                if lower.starts_with("validateset(") {
                                    // Extract values from ValidateSet('val1', 'val2', ...)
                                    if let Some(values) = parse_validate_set(bracket_content) {
                                        validate_set = Some(values);
                                    }
                                }

                                // Check if this looks like a type (simple word, possibly with [])
                                let is_type = !bracket_content.contains('(')
                                    && !bracket_content.contains('=')
                                    && (bracket_content.chars().next().unwrap_or(' ').is_alphabetic()
                                        || bracket_content.starts_with('['));

                                if is_type && !found_dollar {
                                    type_annotation = Some(bracket_content.to_string());
                                }
                            }
                            state = State::Normal;
                            last_bracket_start = None;
                        }
                    }
                    '(' => paren_depth += 1,
                    ')' => paren_depth = paren_depth.saturating_sub(1),
                    _ => {}
                }
            }
            State::Normal => {
                match ch {
                    '\'' => state = State::InSingleQuote,
                    '"' => state = State::InDoubleQuote,
                    '[' => {
                        state = State::InBracket;
                        bracket_depth = 1;
                        last_bracket_start = Some(idx);
                    }
                    '$' => {
                        found_dollar = true;
                        // Extract variable name
                        let name_start = idx + 1;
                        let mut name_end = name_start;
                        while let Some(&(_, next_ch)) = chars.peek() {
                            if next_ch.is_alphanumeric() || next_ch == '_' {
                                name_end += 1;
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        var_name = Some(content[name_start..name_end].to_string());
                    }
                    '=' if found_dollar => {
                        // Extract default value
                        // Skip whitespace after =
                        while let Some(&(_, next_ch)) = chars.peek() {
                            if next_ch.is_whitespace() {
                                chars.next();
                            } else {
                                break;
                            }
                        }

                        let default_start = chars.peek().map(|(i, _)| *i).unwrap_or(content.len());
                        let mut default_end = default_start;
                        let mut in_string = false;
                        let mut string_char = ' ';

                        while let Some((i, ch)) = chars.peek().copied() {
                            if in_string {
                                if ch == string_char && content.chars().nth(i.saturating_sub(1)) != Some('`') {
                                    in_string = false;
                                    default_end = i + 1;
                                    chars.next();
                                } else {
                                    default_end = i + 1;
                                    chars.next();
                                }
                            } else if ch == '\'' || ch == '"' {
                                in_string = true;
                                string_char = ch;
                                default_end = i + 1;
                                chars.next();
                            } else if ch == ',' {
                                break;
                            } else if ch.is_whitespace() && chars.clone().skip(1).next().map(|(_, c)| c) == Some(',') {
                                break;
                            } else {
                                default_end = i + 1;
                                chars.next();
                            }
                        }

                        default_value = Some(content[default_start..default_end].trim().to_string());
                    }
                    ',' => {
                        // End of parameter, finalize it
                        if let Some(name) = var_name.take() {
                            args.push(finalize_parameter(name, type_annotation.take(), default_value.take(), is_mandatory, validate_set.take())?);
                        }

                        // Reset for next parameter
                        type_annotation = None;
                        var_name = None;
                        default_value = None;
                        is_mandatory = false;
                        validate_set = None;
                        found_dollar = false;
                    }
                    _ => {}
                }
            }
        }
    }

    // Finalize last parameter
    if let Some(name) = var_name {
        args.push(finalize_parameter(name, type_annotation, default_value, is_mandatory, validate_set)?);
    }

    Ok(args)
}

fn finalize_parameter(
    name: String,
    type_annotation: Option<String>,
    default_value: Option<String>,
    is_mandatory: bool,
    validate_set: Option<Vec<String>>,
) -> anyhow::Result<Arg> {
    // Store the original PowerShell type for use in the executor
    let otyp = type_annotation.clone();

    // If ValidateSet is present, use it to create an enum type
    let mut parsed_typ = if let Some(ref enum_values) = validate_set {
        Some(Typ::Str(Some(enum_values.clone())))
    } else if let Some(typ) = type_annotation {
        if typ.ends_with("[]") {
            Some(Typ::List(Box::new(parse_powershell_single_typ(
                typ.strip_suffix("[]").unwrap(),
            ))))
        } else {
            Some(parse_powershell_single_typ(&typ))
        }
    } else {
        None
    };

    let default = if let Some(default_str) = default_value {
        // Try to parse as string (quoted)
        if (default_str.starts_with('"') && default_str.ends_with('"'))
            || (default_str.starts_with('\'') && default_str.ends_with('\''))
        {
            Some(json!(default_str[1..default_str.len() - 1].to_string()))
        } else {
            // Try to parse as number
            if parsed_typ.is_none() {
                if default_str.parse::<i64>().is_ok() {
                    parsed_typ = Some(Typ::Int);
                } else if default_str.parse::<f64>().is_ok() {
                    parsed_typ = Some(Typ::Float);
                }
            }
            serde_json::Number::from_str(&default_str)
                .ok()
                .map(serde_json::Value::Number)
        }
    } else {
        None
    };

    // has_default semantics:
    // - true: parameter is optional (has a default value OR is not mandatory)
    // - false: parameter is required (marked as Mandatory AND no default value)
    // Simplified: A parameter is optional unless it's mandatory without a default
    let has_default = default.is_some() || !is_mandatory;

    Ok(Arg {
        name,
        typ: parsed_typ.unwrap_or(Typ::Str(None)),
        default: default.clone(),
        otyp,
        has_default,
        oidx: None,
    })
}

fn parse_powershell_file(code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    let param_wrapper = extract_powershell_param_block(code, false);
    if let Some(param_wrapper) = param_wrapper {
        Ok(Some(parse_powershell_parameters(param_wrapper)?))
    } else {
        Ok(Some(vec![]))
    }
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
        let code = r#"param($Msg, [string]$Msg2, $Dflt = "default value, with comma", [int]$Nb = 3 , $Nb2 = 5.0, $Nb3 = 5, $Wahoo = $env:WAHOO, [PSCustomObject]$Obj, [string[]]$Arr, [Parameter(Mandatory)][ValidateSet('Green', 'Blue', 'Red')][string]$Message)"#;
        assert_eq!(
            parse_powershell_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None, // No type annotation
                        name: "Msg".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true, // Optional (not mandatory)
                        oidx: None
                    },
                    Arg {
                        otyp: Some("string".to_string()), // [string]
                        name: "Msg2".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true, // Optional (not mandatory)
                        oidx: None
                    },
                    Arg {
                        otyp: None, // No type annotation
                        name: "Dflt".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("default value, with comma")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: Some("int".to_string()), // [int]
                        name: "Nb".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(3)),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None, // Type inferred from default value
                        name: "Nb2".to_string(),
                        typ: Typ::Float,
                        default: Some(json!(5.0)),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None, // Type inferred from default value
                        name: "Nb3".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(5)),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None, // No type annotation
                        name: "Wahoo".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true, // Optional (not mandatory)
                        oidx: None
                    },
                    Arg {
                        otyp: Some("PSCustomObject".to_string()), // [PSCustomObject]
                        name: "Obj".to_string(),
                        typ: Typ::Object(ObjectType::new(None, None)),
                        default: None,
                        has_default: true, // Optional (not mandatory)
                        oidx: None
                    },
                    Arg {
                        otyp: Some("string[]".to_string()), // [string[]]
                        name: "Arr".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: None,
                        has_default: true, // Optional (not mandatory)
                        oidx: None
                    },
                    Arg {
                        otyp: Some("string".to_string()), // [string] (last type bracket with Mandatory and ValidateSet)
                        name: "Message".to_string(),
                        typ: Typ::Str(Some(vec!["Green".to_string(), "Blue".to_string(), "Red".to_string()])), // ValidateSet enum
                        default: None,
                        has_default: false, // Required (Mandatory attribute)
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
    fn test_extract_powershell_param_block() {
        // Basic cases
        assert_eq!(
            extract_powershell_param_block("param($Name, $Age)", true),
            Some("param($Name, $Age)")
        );
        assert_eq!(
            extract_powershell_param_block("param($Name, $Age)", false),
            Some("$Name, $Age")
        );

        // Case insensitive and whitespace
        assert_eq!(
            extract_powershell_param_block("PARAM  ($Value)", false),
            Some("$Value")
        );

        // Nested parentheses
        assert_eq!(
            extract_powershell_param_block("param([ValidateScript({$_ -gt 0})]$Count)", false),
            Some("[ValidateScript({$_ -gt 0})]$Count")
        );

        // Strings with parentheses
        assert_eq!(
            extract_powershell_param_block("param([string]$Path = 'C:\\file(1).txt')", false),
            Some("[string]$Path = 'C:\\file(1).txt'")
        );
        assert_eq!(
            extract_powershell_param_block(r#"param($Msg = "Hello (world)")"#, false),
            Some(r#"$Msg = "Hello (world)""#)
        );
        assert_eq!(
            extract_powershell_param_block(
                r#"param([Parameter(Mandatory)][ValidateSet('Green', 'Blue', 'Red')][string]$Message)"#,
                false
            ),
            Some(r#"[Parameter(Mandatory)][ValidateSet('Green', 'Blue', 'Red')][string]$Message"#)
        );

        // Escaped quotes
        assert_eq!(
            extract_powershell_param_block("param($Text = 'don''t')", false),
            Some("$Text = 'don''t'")
        );
        assert_eq!(
            extract_powershell_param_block(r#"param($Text = "He said `"Hi`"")"#, false),
            Some(r#"$Text = "He said `"Hi`"""#)
        );

        // Multiline
        let multiline = "param(\n    [string]$Name,\n    [int]$Age\n)";
        assert!(extract_powershell_param_block(multiline, false).is_some());

        // Invalid cases
        assert_eq!(extract_powershell_param_block("$x = 5", false), None);
        assert_eq!(extract_powershell_param_block("param", false), None);
        assert_eq!(extract_powershell_param_block("param($x", false), None);

        // Valid: param at beginning with single-line comments before
        assert_eq!(
            extract_powershell_param_block("# This is a comment\nparam($Name)", false),
            Some("$Name")
        );
        assert_eq!(
            extract_powershell_param_block("# Comment 1\n# Comment 2\n\nparam($Name)", false),
            Some("$Name")
        );

        // Valid: param at beginning with block comment before
        assert_eq!(
            extract_powershell_param_block("<# Block comment #>\nparam($Name)", false),
            Some("$Name")
        );
        assert_eq!(
            extract_powershell_param_block(
                "<#\n  Multi-line\n  block comment\n#>\nparam($Name)",
                false
            ),
            Some("$Name")
        );

        // Valid: mixed comments and whitespace
        assert_eq!(
            extract_powershell_param_block(
                "# Line comment\n<# Block comment #>\n\nparam($Name)",
                false
            ),
            Some("$Name")
        );

        // Invalid: code before param
        assert_eq!(
            extract_powershell_param_block("$x = 5\nparam($Name)", false),
            None
        );
        assert_eq!(
            extract_powershell_param_block("Write-Host 'test'\nparam($Name)", false),
            None
        );

        // Invalid: unclosed block comment
        assert_eq!(
            extract_powershell_param_block("<# Unclosed comment\nparam($Name)", false),
            None
        );

        // Invalid: unclosed block comment
        assert_eq!(
            extract_powershell_param_block("function test-x{   param($Name)\n}", false),
            None
        );

        // Valid: [CmdletBinding()] before param
        assert_eq!(
            extract_powershell_param_block("[CmdletBinding()]\nparam($Name)", false),
            Some("$Name")
        );
        assert_eq!(
            extract_powershell_param_block("[CmdletBinding()]\nparam($Name, $Age)", true),
            Some("param($Name, $Age)")
        );

        // Valid: [CmdletBinding()] with options before param
        assert_eq!(
            extract_powershell_param_block(
                "[CmdletBinding(SupportsShouldProcess=$true)]\nparam($Path)",
                false
            ),
            Some("$Path")
        );

        // Valid: Multiple attributes before param
        assert_eq!(
            extract_powershell_param_block(
                "[CmdletBinding()]\n[OutputType([string])]\nparam($Value)",
                false
            ),
            Some("$Value")
        );

        // Valid: CmdletBinding with comments
        assert_eq!(
            extract_powershell_param_block(
                "# My function\n[CmdletBinding()]\nparam($Name)",
                false
            ),
            Some("$Name")
        );

        // Valid: CmdletBinding with whitespace variations
        assert_eq!(
            extract_powershell_param_block(
                "[CmdletBinding()]  \n  param($Name)",
                false
            ),
            Some("$Name")
        );

        // Invalid: Unclosed attribute bracket
        assert_eq!(
            extract_powershell_param_block("[CmdletBinding(\nparam($Name)", false),
            None
        );

        // Valid: CmdletBinding with DefaultParameterSetName
        assert_eq!(
            extract_powershell_param_block(
                "[CmdletBinding(DefaultParameterSetName='ByName')]\nparam($Name, $Id)",
                false
            ),
            Some("$Name, $Id")
        );

        // Valid: CmdletBinding with complex parameters
        assert_eq!(
            extract_powershell_param_block(
                "[CmdletBinding(DefaultParameterSetName='ByName', SupportsShouldProcess=$true)]\nparam($Path)",
                false
            ),
            Some("$Path")
        );

        // Valid: Multiple attributes with parameters
        assert_eq!(
            extract_powershell_param_block(
                "[CmdletBinding(DefaultParameterSetName='Set1')]\n[OutputType([string])]\nparam($Value)",
                false
            ),
            Some("$Value")
        );
    }

    #[test]
    fn test_parse_powershell_sig_with_parameter_attributes() -> anyhow::Result<()> {
        // Test with [Parameter(Mandatory=$true)] attribute
        let code = r#"[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)]
    [string]$Name,
    [Parameter(Mandatory=$false)]
    [int]$Age = 25
)"#;
        let result = parse_powershell_sig(code)?;
        assert_eq!(result.args.len(), 2);
        assert_eq!(result.args[0].name, "Name");
        assert_eq!(result.args[0].typ, Typ::Str(None));
        assert_eq!(result.args[0].has_default, false);
        assert_eq!(result.args[1].name, "Age");
        assert_eq!(result.args[1].typ, Typ::Int);
        assert_eq!(result.args[1].has_default, true);
        assert_eq!(result.args[1].default, Some(json!(25)));

        // Test with complex attributes including ValidateSet
        let code2 = r#"param(
    [Parameter(Mandatory=$true, Position=0)]
    [ValidateSet('Red', 'Green', 'Blue')]
    [string]$Color,
    [Parameter(ValueFromPipeline=$true)]
    [string[]]$Items
)"#;
        let result2 = parse_powershell_sig(code2)?;
        assert_eq!(result2.args.len(), 2);
        assert_eq!(result2.args[0].name, "Color");
        assert_eq!(
            result2.args[0].typ,
            Typ::Str(Some(vec![
                "Red".to_string(),
                "Green".to_string(),
                "Blue".to_string()
            ]))
        );
        assert_eq!(result2.args[1].name, "Items");
        assert_eq!(result2.args[1].typ, Typ::List(Box::new(Typ::Str(None))));

        Ok(())
    }

    #[test]
    fn test_powershell_single_pass_parser() -> anyhow::Result<()> {
        // Test the single-pass parser with a complex real-world example
        let code = r#"[CmdletBinding()]
param(
    [Parameter(Mandatory=$true, Position=0, HelpMessage="Enter the server name")]
    [ValidateNotNullOrEmpty()]
    [string]$ServerName,

    [Parameter(Mandatory=$false)]
    [ValidateRange(1, 65535)]
    [int]$Port = 8080,

    [Parameter(ValueFromPipeline=$true)]
    [string[]]$LogFiles,

    [ValidateSet('Debug', 'Info', 'Warning', 'Error')]
    [string]$LogLevel = 'Info',

    [PSCustomObject]$Config
)"#;
        let result = parse_powershell_sig(code)?;

        assert_eq!(result.args.len(), 5);

        // ServerName: mandatory string with no default
        assert_eq!(result.args[0].name, "ServerName");
        assert_eq!(result.args[0].typ, Typ::Str(None));
        assert_eq!(result.args[0].has_default, false);

        // Port: optional int with default
        assert_eq!(result.args[1].name, "Port");
        assert_eq!(result.args[1].typ, Typ::Int);
        assert_eq!(result.args[1].default, Some(json!(8080)));
        assert_eq!(result.args[1].has_default, true);

        // LogFiles: string array (no mandatory, so optional)
        assert_eq!(result.args[2].name, "LogFiles");
        assert_eq!(result.args[2].typ, Typ::List(Box::new(Typ::Str(None))));
        assert_eq!(result.args[2].has_default, true); // Optional (not mandatory)

        // LogLevel: string with default and ValidateSet (creates enum type)
        assert_eq!(result.args[3].name, "LogLevel");
        assert_eq!(
            result.args[3].typ,
            Typ::Str(Some(vec![
                "Debug".to_string(),
                "Info".to_string(),
                "Warning".to_string(),
                "Error".to_string()
            ]))
        );
        assert_eq!(result.args[3].default, Some(json!("Info")));
        assert_eq!(result.args[3].has_default, true);

        // Config: PSCustomObject (no mandatory, so optional)
        assert_eq!(result.args[4].name, "Config");
        assert_eq!(result.args[4].typ, Typ::Object(ObjectType::new(None, None)));
        assert_eq!(result.args[4].has_default, true); // Optional (not mandatory)

        Ok(())
    }

    #[test]
    fn test_powershell_mandatory_attribute() -> anyhow::Result<()> {
        // Test various forms of the Mandatory attribute
        let code = r#"param(
    [Parameter(Mandatory)]
    [string]$RequiredNoEquals,

    [Parameter(Mandatory=$true)]
    [string]$RequiredWithTrue,

    [Parameter(Mandatory = $true)]
    [string]$RequiredWithSpaces,

    [Parameter(Mandatory=$false)]
    [string]$NotRequired,

    [Parameter(Position=0)]
    [string]$NoMandatory,

    [string]$PlainRequired = "default",

    [Parameter(Mandatory=$true)]
    [int]$RequiredInt
)"#;
        let result = parse_powershell_sig(code)?;

        assert_eq!(result.args.len(), 7);

        // RequiredNoEquals: mandatory without =$true
        assert_eq!(result.args[0].name, "RequiredNoEquals");
        assert_eq!(result.args[0].has_default, false); // Required (mandatory, no default)

        // RequiredWithTrue: mandatory with =$true
        assert_eq!(result.args[1].name, "RequiredWithTrue");
        assert_eq!(result.args[1].has_default, false); // Required

        // RequiredWithSpaces: mandatory with spaces
        assert_eq!(result.args[2].name, "RequiredWithSpaces");
        assert_eq!(result.args[2].has_default, false); // Required

        // NotRequired: explicitly Mandatory=$false
        assert_eq!(result.args[3].name, "NotRequired");
        assert_eq!(result.args[3].has_default, true); // Optional (not mandatory)

        // NoMandatory: no Mandatory attribute
        assert_eq!(result.args[4].name, "NoMandatory");
        assert_eq!(result.args[4].has_default, true); // Optional (not mandatory)

        // PlainRequired: has default value (always optional)
        assert_eq!(result.args[5].name, "PlainRequired");
        assert_eq!(result.args[5].has_default, true); // Optional (has default)
        assert_eq!(result.args[5].default, Some(json!("default")));

        // RequiredInt: mandatory int
        assert_eq!(result.args[6].name, "RequiredInt");
        assert_eq!(result.args[6].typ, Typ::Int);
        assert_eq!(result.args[6].has_default, false); // Required

        Ok(())
    }

    #[test]
    fn test_extract_powershell_param_block_with_attributes() {
        // Test without attributes
        let code = "param($Name, $Age)";
        let result = extract_powershell_param_block_with_attributes(code, true);
        assert_eq!(result, Some(("param($Name, $Age)", "")));

        // Test with simple CmdletBinding
        let code2 = "[CmdletBinding()]\nparam($Name)";
        let result2 = extract_powershell_param_block_with_attributes(code2, true);
        assert_eq!(result2, Some(("[CmdletBinding()]\nparam($Name)", "")));

        // Test with CmdletBinding with parameters
        let code3 = "[CmdletBinding(DefaultParameterSetName='ByName')]\nparam($Name, $Id)";
        let result3 = extract_powershell_param_block_with_attributes(code3, true);
        assert_eq!(result3, Some(("[CmdletBinding(DefaultParameterSetName='ByName')]\nparam($Name, $Id)", "")));

        // Test with multiple attributes
        let code4 = "[CmdletBinding()]\n[OutputType([string])]\nparam($Value)";
        let result4 = extract_powershell_param_block_with_attributes(code4, true);
        assert_eq!(result4, Some(("[CmdletBinding()]\n[OutputType([string])]\nparam($Value)", "")));

        // Test with comment before attributes
        let code5 = "# My function\n[CmdletBinding()]\nparam($Name)";
        let result5 = extract_powershell_param_block_with_attributes(code5, true);
        assert_eq!(result5, Some(("# My function\n[CmdletBinding()]\nparam($Name)", "")));

        // Test with include_attributes = false (should only get param block, not attributes)
        let code6 = "[CmdletBinding()]\nparam($Name)";
        let result6 = extract_powershell_param_block_with_attributes(code6, false);
        assert_eq!(result6, Some(("param($Name)", "")));

        // Test with code after param
        let code7 = "[CmdletBinding()]\nparam($Name)\nWrite-Host 'Hello'";
        let result7 = extract_powershell_param_block_with_attributes(code7, true);
        assert_eq!(result7, Some(("[CmdletBinding()]\nparam($Name)", "\nWrite-Host 'Hello'")));

        // Test with code after param (without attributes)
        let code8 = "[CmdletBinding()]\nparam($Name)\nWrite-Host 'Hello'";
        let result8 = extract_powershell_param_block_with_attributes(code8, false);
        assert_eq!(result8, Some(("param($Name)", "\nWrite-Host 'Hello'")));
    }

    #[test]
    fn test_powershell_sig_with_cmdletbinding_paramsetname() -> anyhow::Result<()> {
        // Test with [CmdletBinding(DefaultParameterSetName='ByName')]
        let code = r#"[CmdletBinding(DefaultParameterSetName='ByName')]
param(
    [Parameter(Mandatory=$true, ParameterSetName='ByName')]
    [string]$Name,

    [Parameter(Mandatory=$true, ParameterSetName='ById')]
    [int]$Id,

    [string]$Description = "default description"
)"#;
        let result = parse_powershell_sig(code)?;

        assert_eq!(result.args.len(), 3);

        // Name: mandatory string
        assert_eq!(result.args[0].name, "Name");
        assert_eq!(result.args[0].typ, Typ::Str(None));
        assert_eq!(result.args[0].has_default, false);

        // Id: mandatory int
        assert_eq!(result.args[1].name, "Id");
        assert_eq!(result.args[1].typ, Typ::Int);
        assert_eq!(result.args[1].has_default, false);

        // Description: optional with default
        assert_eq!(result.args[2].name, "Description");
        assert_eq!(result.args[2].typ, Typ::Str(None));
        assert_eq!(result.args[2].default, Some(json!("default description")));
        assert_eq!(result.args[2].has_default, true);

        Ok(())
    }

    #[test]
    fn test_powershell_case_insensitive_parameter() -> anyhow::Result<()> {
        // Test that [parameter(...)] is case-insensitive
        let code = r#"param(
    [parameter(Mandatory)]
    [string]$LowerCase,

    [PARAMETER(MANDATORY=$TRUE)]
    [string]$UpperCase,

    [Parameter(mandatory=$true)]
    [string]$MixedCase
)"#;
        let result = parse_powershell_sig(code)?;

        assert_eq!(result.args.len(), 3);

        // All should be detected as mandatory
        assert_eq!(result.args[0].name, "LowerCase");
        assert_eq!(result.args[0].has_default, false);

        assert_eq!(result.args[1].name, "UpperCase");
        assert_eq!(result.args[1].has_default, false);

        assert_eq!(result.args[2].name, "MixedCase");
        assert_eq!(result.args[2].has_default, false);

        Ok(())
    }

    #[test]
    fn test_powershell_validateset_enum() -> anyhow::Result<()> {
        // Test with ValidateSet creating an enum type
        let code = r#"param(
    [ValidateSet('Red', 'Green', 'Blue')]
    [string]$Color,

    [Parameter(Mandatory=$true)]
    [ValidateSet("Small", "Medium", "Large")]
    [string]$Size
)"#;
        let result = parse_powershell_sig(code)?;

        assert_eq!(result.args.len(), 2);

        // Color: optional with ValidateSet (enum)
        assert_eq!(result.args[0].name, "Color");
        assert_eq!(
            result.args[0].typ,
            Typ::Str(Some(vec![
                "Red".to_string(),
                "Green".to_string(),
                "Blue".to_string()
            ]))
        );
        assert_eq!(result.args[0].has_default, true); // Optional (not mandatory)

        // Size: mandatory with ValidateSet (enum)
        assert_eq!(result.args[1].name, "Size");
        assert_eq!(
            result.args[1].typ,
            Typ::Str(Some(vec![
                "Small".to_string(),
                "Medium".to_string(),
                "Large".to_string()
            ]))
        );
        assert_eq!(result.args[1].has_default, false); // Required (mandatory)

        Ok(())
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
