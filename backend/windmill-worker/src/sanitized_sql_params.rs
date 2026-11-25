use anyhow::anyhow;
use regex::Regex;
use std::collections::{HashMap, HashSet};

use serde_json::Value;
use windmill_common::error;
use windmill_parser::Arg;
use windmill_parser_sql::{SANITIZED_ENUM_STR, SANITIZED_RAW_STRING_STR};

lazy_static::lazy_static! {
    static ref RE_SQL_CONTEXTUAL_VAR: Regex = Regex::new(r"%%WM_[A-Z_]+%%").unwrap();
}

/// Identifier must be a continuous ASCII alphanumeric word, not starting with
/// a number, that can contain underscores
fn sanitize_identifier(arg: &Arg, input: &str) -> Result<(), error::Error> {
    if input.is_empty() {
        return Err(error::Error::BadRequest(format!(
            "Interpolated argument `{}` cannot be empty",
            arg.name
        )));
    }
    if input
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic())
        .unwrap_or(false)
        && input.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Ok(())
    } else {
        Err(error::Error::BadRequest(format!("Interpolated argument `{}` contained forbidden characters. Received `{}` but should only contain alphanumerical characters and `_`.", arg.name, input)))
    }
}

/// Contextual variables are pasted as raw text, usually inside a string literal, a quoted
/// identifier or a comment, and some values come from the job's inputs (an email, the
/// `_FLOW_PATH` argument). A value must not end any of those, in any dialect.
fn sanitize_contextual_variable(name: &str, value: &str) -> Result<(), error::Error> {
    if value
        .chars()
        .any(|c| matches!(c, '\'' | '"' | '`' | ']' | '\\' | '$') || c.is_control())
        || value.contains("/*")
        || value.contains("*/")
    {
        Err(error::Error::BadRequest(format!(
            "Contextual variable `{name}` contains characters that cannot be safely interpolated \
             in a SQL query (quotes, backticks, `]`, `\\`, `$`, comment delimiters or control \
             characters)"
        )))
    } else {
        Ok(())
    }
}

fn replace_contextual_variables(
    code: &mut String,
    contextual_variables: &HashMap<String, String>,
) -> Result<(), error::Error> {
    let vars = RE_SQL_CONTEXTUAL_VAR
        .find_iter(&code)
        .map(|m| m.as_str().to_string())
        .collect::<HashSet<_>>();

    for var_pattern in vars {
        let var_name = var_pattern
            .strip_prefix("%%")
            .unwrap()
            .strip_suffix("%%")
            .unwrap();
        let var_value = contextual_variables.get(var_name);
        if let Some(var_value) = var_value {
            sanitize_contextual_variable(var_name, var_value)?;
            *code = code.replace(&var_pattern, var_value);
        }
    }
    Ok(())
}

pub fn sanitize_and_interpolate_unsafe_sql_args(
    code: &str,
    args: &Vec<Arg>,
    args_map: &HashMap<String, Value>,
    contextual_variables: &HashMap<String, String>,
) -> Result<(String, Vec<String>), error::Error> {
    let mut ret = code.to_string();
    let mut args_to_skip = vec![];

    replace_contextual_variables(&mut ret, contextual_variables)?;

    for arg in args {
        if let Some(typ) = &arg.otyp {
            let pattern = format!("%%{}%%", arg.name);
            match typ.as_str() {
                SANITIZED_ENUM_STR => {
                    let replace =
                        args_map
                            .get(&arg.name)
                            .and_then(|rv| rv.as_str())
                            .ok_or(anyhow!(
                                "Sanitized enum `{}` needs to receive a string",
                                arg.name
                            ))?;
                    let windmill_parser::Typ::Str(Some(variants)) = &arg.typ else {
                        return Err(error::Error::ArgumentErr(format!(
                            "Wrong type of argument for sanitized enum `{}`",
                            arg.name
                        )));
                    };
                    if variants.iter().all(|v| v != replace) {
                        return Err(error::Error::ArgumentErr(format!(
                            "Sanitized enum argument `{}` expected one of `[{}]` but received `{}`",
                            arg.name,
                            variants
                                .iter()
                                .map(|s| format!("{s}"))
                                .collect::<Vec<String>>()
                                .join(","),
                            replace,
                        )));
                    }

                    sanitize_identifier(&arg, replace)?;
                    ret = ret.replace(&pattern, replace);
                    args_to_skip.push(arg.name.to_string());
                }
                SANITIZED_RAW_STRING_STR => {
                    let replace =
                        args_map
                            .get(&arg.name)
                            .and_then(|rv| rv.as_str())
                            .ok_or(anyhow!(
                                "Sanitized raw string `{}` needs to receive a string",
                                arg.name
                            ))?;
                    let windmill_parser::Typ::Str(_) = &arg.typ else {
                        return Err(error::Error::ArgumentErr(format!(
                            "Wrong type of argument for sanitized raw string `{}`",
                            arg.name
                        )));
                    };
                    sanitize_identifier(&arg, replace)?;
                    ret = ret.replace(&pattern, &replace);
                    args_to_skip.push(arg.name.to_string());
                }
                _ => continue,
            }
        }
    }

    Ok((ret, args_to_skip))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interpolate(code: &str, var: &str, value: &str) -> Result<String, error::Error> {
        let vars = HashMap::from([(var.to_string(), value.to_string())]);
        sanitize_and_interpolate_unsafe_sql_args(code, &vec![], &HashMap::new(), &vars)
            .map(|(code, _)| code)
    }

    #[test]
    fn contextual_variables_with_quoting_characters_are_refused() {
        assert_eq!(
            interpolate(
                "SELECT '%%WM_EMAIL%%'",
                "WM_EMAIL",
                "first.last+tag@windmill.dev"
            )
            .unwrap(),
            "SELECT 'first.last+tag@windmill.dev'"
        );
        for value in [
            "o'brien@example.com",
            "f/a/b', 'c",
            "x\\",
            "say \"hi\"",
            "a`b",
            "a]b",
            "a$$b",
            "a\nb",
            "a*/b",
            "a/*b",
        ] {
            let err =
                interpolate("SELECT '%%WM_FLOW_PATH%%'", "WM_FLOW_PATH", value).expect_err(value);
            assert!(matches!(err, error::Error::BadRequest(_)), "{value}");
        }
    }
}
