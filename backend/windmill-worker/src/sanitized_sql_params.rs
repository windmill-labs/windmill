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

/// How a dialect escapes characters inside a string literal. Contextual variables are
/// substituted as raw text, and `WM_END_USER_EMAIL` carries an app end user's email into a
/// query that runs with the author's credentials: an email may contain `'` (and `\` or `"`
/// in a quoted local part), so every value is escaped for any string literal it lands in.
#[derive(Clone, Copy)]
pub enum SqlStringEscaping {
    /// `'` → `''`; `\` is ordinary and `"` quotes identifiers (PostgreSQL, MSSQL, Oracle,
    /// DuckDB).
    Standard,
    /// `\` → `\\`, `'` → `\'` and `"` → `\"`, exact in both `'...'` and `"..."` literals
    /// (MySQL, Snowflake, BigQuery; BigQuery does not accept `''`).
    #[cfg_attr(
        not(any(feature = "mysql", feature = "snowflake", feature = "bigquery")),
        allow(dead_code)
    )]
    Backslash,
    /// `'` → `''` and `"` → `""`, for MySQL under `NO_BACKSLASH_ESCAPES`, where `\` is
    /// ordinary and `\'` would close the literal. A quote of the other style than the
    /// surrounding literal comes out doubled.
    #[cfg_attr(not(feature = "mysql"), allow(dead_code))]
    DoubledQuotes,
}

impl SqlStringEscaping {
    fn escape(self, value: &str) -> String {
        match self {
            SqlStringEscaping::Standard => value.replace('\'', "''"),
            SqlStringEscaping::Backslash => value
                .replace('\\', "\\\\")
                .replace('\'', "\\'")
                .replace('"', "\\\""),
            SqlStringEscaping::DoubledQuotes => value.replace('\'', "''").replace('"', "\"\""),
        }
    }
}

#[cfg(feature = "mysql")]
pub fn has_contextual_variables(code: &str) -> bool {
    RE_SQL_CONTEXTUAL_VAR.is_match(code)
}

fn replace_contextual_variables(
    code: &mut String,
    contextual_variables: &HashMap<String, String>,
    escaping: SqlStringEscaping,
) -> () {
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
            *code = code.replace(&var_pattern, &escaping.escape(var_value));
        }
    }
}

pub fn sanitize_and_interpolate_unsafe_sql_args(
    code: &str,
    args: &Vec<Arg>,
    args_map: &HashMap<String, Value>,
    contextual_variables: &HashMap<String, String>,
    escaping: SqlStringEscaping,
) -> Result<(String, Vec<String>), error::Error> {
    let mut ret = code.to_string();
    let mut args_to_skip = vec![];

    replace_contextual_variables(&mut ret, contextual_variables, escaping);

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

    fn interpolate(code: &str, email: &str, escaping: SqlStringEscaping) -> String {
        let vars = HashMap::from([("WM_END_USER_EMAIL".to_string(), email.to_string())]);
        sanitize_and_interpolate_unsafe_sql_args(code, &vec![], &HashMap::new(), &vars, escaping)
            .unwrap()
            .0
    }

    #[test]
    fn contextual_variables_stay_inside_their_string_literal() {
        let code = "SELECT 1 WHERE email = '%%WM_END_USER_EMAIL%%'";
        let quote = r"x'/**/OR/**/'1'='1'--@e.com";
        let backslash = r#""x\'/**/OR/**/1=1#"@e.com"#;

        assert_eq!(
            interpolate(code, quote, SqlStringEscaping::Standard),
            r"SELECT 1 WHERE email = 'x''/**/OR/**/''1''=''1''--@e.com'"
        );
        assert_eq!(
            interpolate(code, backslash, SqlStringEscaping::Backslash),
            r#"SELECT 1 WHERE email = '\"x\\\'/**/OR/**/1=1#\"@e.com'"#
        );
        assert_eq!(
            interpolate(code, backslash, SqlStringEscaping::DoubledQuotes),
            r#"SELECT 1 WHERE email = '""x\''/**/OR/**/1=1#""@e.com'"#
        );
        for escaping in [
            SqlStringEscaping::Standard,
            SqlStringEscaping::Backslash,
            SqlStringEscaping::DoubledQuotes,
        ] {
            assert_eq!(
                interpolate(code, "a.b+c@e.com", escaping),
                "SELECT 1 WHERE email = 'a.b+c@e.com'"
            );
        }
    }
}
