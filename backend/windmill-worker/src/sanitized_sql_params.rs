use anyhow::anyhow;
use std::collections::HashMap;

use serde_json::Value;
use windmill_common::error;
use windmill_parser::Arg;
use windmill_parser_sql::{SANITIZED_ENUM_STR, SANITIZED_RAW_STRING_STR};

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

pub fn sanitize_and_interpolate_unsafe_sql_args(
    code: &str,
    args: &Vec<Arg>,
    args_map: &HashMap<String, Value>,
) -> Result<(String, Vec<String>), error::Error> {
    let mut ret = code.to_string();
    let mut args_to_skip = vec![];

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
