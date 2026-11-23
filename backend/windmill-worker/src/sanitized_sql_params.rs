use anyhow::anyhow;
use regex::Regex;
use std::collections::HashMap;

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

/// How a dialect escapes a value substituted into a quote-delimited string literal.
/// Contextual variables are substituted as raw text, and `WM_END_USER_EMAIL` carries an app
/// end user's email into a query that runs with the author's credentials: an email may
/// contain `'` (and `\` or `"` in a quoted local part).
///
/// Each mode must hold in every quote-delimited literal form and session mode the dialect
/// has, since a script can switch modes itself. Where some literal processes backslash
/// escapes, `\` is doubled too, or a value's `\'` would become `\''` and close the literal.
/// The price is a doubled `\` or quote in the rare value containing one, in a literal that
/// did not need it. Dollar-quoted, Oracle `q'[...]'` and raw literals cannot be escaped into.
#[derive(Clone, Copy)]
pub enum SqlStringEscaping {
    /// `'` → `''` (Oracle).
    #[cfg_attr(not(feature = "oracledb"), allow(dead_code))]
    Quote,
    /// `'` → `''`, `"` → `""`: `"..."` is a string under `QUOTED_IDENTIFIER OFF` (MSSQL).
    #[cfg_attr(not(all(feature = "enterprise", feature = "mssql")), allow(dead_code))]
    BothQuotes,
    /// `\` → `\\`, `'` → `''` (PostgreSQL and DuckDB `E'...'`, Snowflake).
    QuoteAndBackslash,
    /// `\` → `\\`, `'` → `''`, `"` → `""`: `"..."` is a string, and only doubling stays
    /// confined under `NO_BACKSLASH_ESCAPES` (MySQL).
    #[cfg_attr(not(feature = "mysql"), allow(dead_code))]
    MySql,
    /// `\` → `\\`, quotes as `\x27`/`\x22`: `''` is not accepted, and a value with no quote
    /// characters left cannot desync `parse_sql_blocks` or close a raw string (BigQuery).
    #[cfg_attr(not(feature = "bigquery"), allow(dead_code))]
    BigQuery,
}

impl SqlStringEscaping {
    fn escape(self, value: &str) -> String {
        match self {
            SqlStringEscaping::Quote => value.replace('\'', "''"),
            SqlStringEscaping::BothQuotes => value.replace('\'', "''").replace('"', "\"\""),
            SqlStringEscaping::QuoteAndBackslash => value.replace('\\', "\\\\").replace('\'', "''"),
            SqlStringEscaping::MySql => value
                .replace('\\', "\\\\")
                .replace('\'', "''")
                .replace('"', "\"\""),
            SqlStringEscaping::BigQuery => value
                .replace('\\', "\\\\")
                .replace('\'', "\\x27")
                .replace('"', "\\x22"),
        }
    }

    /// Whether some literal of the dialect reads `\` as an escape while the escaped value
    /// can still start with a quote or `\`.
    fn backslash_escapes_quotes(self) -> bool {
        matches!(
            self,
            SqlStringEscaping::QuoteAndBackslash | SqlStringEscaping::MySql
        )
    }
}

/// A single pass, so a value containing `%%WM_*%%` is never itself expanded.
fn replace_contextual_variables(
    code: &mut String,
    contextual_variables: &HashMap<String, String>,
    escaping: SqlStringEscaping,
) -> Result<(), error::Error> {
    let mut unsafe_position = None;
    let replaced = RE_SQL_CONTEXTUAL_VAR
        .replace_all(code, |caps: &regex::Captures| {
            let m = caps.get(0).unwrap();
            let pattern = m.as_str();
            let Some(value) = contextual_variables.get(&pattern[2..pattern.len() - 2]) else {
                return pattern.to_string();
            };
            let escaped = escaping.escape(value);
            // An odd run of `\` before the placeholder escapes the value's first character,
            // which shifts the pairing of the doubled quotes or backslashes after it.
            let preceding_backslashes = code[..m.start()]
                .chars()
                .rev()
                .take_while(|c| *c == '\\')
                .count();
            if escaping.backslash_escapes_quotes()
                && preceding_backslashes % 2 == 1
                && escaped.starts_with(['\'', '"', '\\'])
            {
                unsafe_position = Some(pattern.to_string());
            }
            escaped
        })
        .into_owned();
    if let Some(pattern) = unsafe_position {
        return Err(error::Error::ExecutionErr(format!(
            "Contextual variable `{pattern}` directly follows a backslash, which would escape \
             the first character of its value. Move the backslash away from the variable."
        )));
    }
    *code = replaced;
    Ok(())
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

    replace_contextual_variables(&mut ret, contextual_variables, escaping)?;

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
            interpolate(code, quote, SqlStringEscaping::Quote),
            r"SELECT 1 WHERE email = 'x''/**/OR/**/''1''=''1''--@e.com'"
        );
        // `\` doubled too, so `\'` cannot become `\''` in an `E'...'` literal
        assert_eq!(
            interpolate(code, backslash, SqlStringEscaping::QuoteAndBackslash),
            r#"SELECT 1 WHERE email = '"x\\''/**/OR/**/1=1#"@e.com'"#
        );
        assert_eq!(
            interpolate(code, backslash, SqlStringEscaping::MySql),
            r#"SELECT 1 WHERE email = '""x\\''/**/OR/**/1=1#""@e.com'"#
        );
        assert_eq!(
            interpolate(code, backslash, SqlStringEscaping::BothQuotes),
            r#"SELECT 1 WHERE email = '""x\''/**/OR/**/1=1#""@e.com'"#
        );
        assert_eq!(
            interpolate(code, backslash, SqlStringEscaping::BigQuery),
            r#"SELECT 1 WHERE email = '\x22x\\\x27/**/OR/**/1=1#\x22@e.com'"#
        );
        // a value is never expanded itself: `%` is valid in an email's local part
        let vars = HashMap::from([
            ("WM_EMAIL".to_string(), "%%WM_TOKEN%%@e.com".to_string()),
            ("WM_TOKEN".to_string(), "secret".to_string()),
        ]);
        let (out, _) = sanitize_and_interpolate_unsafe_sql_args(
            "SELECT '%%WM_EMAIL%%', '%%WM_TOKEN%%'",
            &vec![],
            &HashMap::new(),
            &vars,
            SqlStringEscaping::Quote,
        )
        .unwrap();
        assert_eq!(out, "SELECT '%%WM_TOKEN%%@e.com', 'secret'");
        // an author's `\` before the placeholder would escape the value's first quote
        let odd_backslash = "SELECT 1 WHERE email = E'\\%%WM_END_USER_EMAIL%%'";
        let payload = "'/**/OR/**/1=1--@e.com";
        for escaping in [
            SqlStringEscaping::QuoteAndBackslash,
            SqlStringEscaping::MySql,
        ] {
            let vars = HashMap::from([("WM_END_USER_EMAIL".to_string(), payload.to_string())]);
            assert!(sanitize_and_interpolate_unsafe_sql_args(
                odd_backslash,
                &vec![],
                &HashMap::new(),
                &vars,
                escaping,
            )
            .is_err());
        }
        for escaping in [
            SqlStringEscaping::Quote,
            SqlStringEscaping::BothQuotes,
            SqlStringEscaping::QuoteAndBackslash,
            SqlStringEscaping::MySql,
            SqlStringEscaping::BigQuery,
        ] {
            assert_eq!(
                interpolate(code, "a.b+c@e.com", escaping),
                "SELECT 1 WHERE email = 'a.b+c@e.com'"
            );
        }
    }
}
