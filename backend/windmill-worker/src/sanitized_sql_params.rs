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

/// Escape a value for safe interpolation inside a SQL single-quoted string
/// literal by doubling single quotes (SQL-standard, accepted by every SQL
/// dialect we target: pg, mysql, mssql, bigquery, snowflake, oracle, duckdb).
///
/// Contextual variables are substituted via plain string replacement, so a
/// value containing a `'` could otherwise break out of the surrounding literal
/// and inject arbitrary SQL. Most contextual values are charset-constrained
/// (usernames, paths and ids cannot contain quotes), but emails are not: the
/// `usr.email` CHECK constraint follows RFC 5321, whose local part allows `'`,
/// `/`, `*` and `-` — enough to form a complete breakout payload such as
/// `x'or/**/1=1--@evil.com`. `%%WM_EMAIL%%` / `%%WM_END_USER_EMAIL%%` are the
/// reachable sinks, the latter from app end users. Escaping is a no-op for the
/// charset-constrained vars, so this is backwards compatible.
fn escape_contextual_value(value: &str) -> String {
    value.replace('\'', "''")
}

fn replace_contextual_variables(
    code: &mut String,
    contextual_variables: &HashMap<String, String>,
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
            *code = code.replace(&var_pattern, &escape_contextual_value(var_value));
        }
    }
}

pub fn sanitize_and_interpolate_unsafe_sql_args(
    code: &str,
    args: &Vec<Arg>,
    args_map: &HashMap<String, Value>,
    contextual_variables: &HashMap<String, String>,
) -> Result<(String, Vec<String>), error::Error> {
    let mut ret = code.to_string();
    let mut args_to_skip = vec![];

    replace_contextual_variables(&mut ret, contextual_variables);

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

    #[test]
    fn contextual_var_with_quote_cannot_break_out_of_literal() {
        // An email passing the RFC `usr.email` CHECK constraint can carry a
        // quote-breakout payload. It must stay confined to the string literal.
        let mut code =
            "SELECT 1 WHERE email = '%%WM_END_USER_EMAIL%%'".to_string();
        let mut ctx = HashMap::new();
        ctx.insert(
            "WM_END_USER_EMAIL".to_string(),
            "x'or/**/1=1--@evil.com".to_string(),
        );

        replace_contextual_variables(&mut code, &ctx);

        // The lone `'` is doubled, so it stays inside the literal instead of
        // terminating it early and starting an injected clause.
        assert_eq!(
            code,
            "SELECT 1 WHERE email = 'x''or/**/1=1--@evil.com'"
        );
    }

    #[test]
    fn quoteless_contextual_var_is_unchanged() {
        let mut code = "SELECT '%%WM_USERNAME%%', '%%WM_JOB_ID%%'".to_string();
        let mut ctx = HashMap::new();
        ctx.insert("WM_USERNAME".to_string(), "alice".to_string());
        ctx.insert(
            "WM_JOB_ID".to_string(),
            "0195e3b1-0000-0000-0000-000000000000".to_string(),
        );

        replace_contextual_variables(&mut code, &ctx);

        assert_eq!(
            code,
            "SELECT 'alice', '0195e3b1-0000-0000-0000-000000000000'"
        );
    }
}
