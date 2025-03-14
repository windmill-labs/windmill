use anyhow::anyhow;
use std::collections::HashMap;

use serde_json::value::RawValue;
use sqlx::types::Json;
use windmill_common::{error, utils::not_found_if_none};
use windmill_parser::Arg;
use windmill_parser_sql::{SANITIZED_DYN_IDENTIFIER_STR, SANITIZED_IDENTIFIER_STR};

/// Identifier must be a continuous ASCII alphanumeric word, not starting with
/// a number, that can contain underscores
fn sanitize_identifier(arg: &Arg, input: &str) -> Result<(), error::Error> {
    if input
        .chars()
        .next()
        .map(|c| c.is_ascii_alphabetic())
        .unwrap_or(false)
        && input.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Ok(())
    } else {
        Err(error::Error::BadRequest(format!("Argument `{}` contained forbidden characters. Received `{}` but should only contain alphanumerical characters and `_`.", arg.name, input)))
    }
}

pub fn sanitize_and_interpolate_unsafe_sql_args(
    code: &str,
    args: &Vec<Arg>,
    args_map: Option<&Json<HashMap<String, Box<RawValue>>>>,
) -> Result<(String, Vec<String>), error::Error> {
    let mut ret = code.to_string();
    let mut args_to_skip = vec![];

    if let Some(args_map) = args_map {
        for arg in args {
            if let Some(typ) = &arg.otyp {
                let pattern = format!("%%{}%%", arg.name);
                match typ.as_str() {
                    typ if typ.starts_with(SANITIZED_IDENTIFIER_STR) => {
                        let replace = &not_found_if_none(
                            args_map
                                .get(&arg.name)
                                .and_then(|rv| serde_json::from_str::<String>(rv.get()).ok()),
                            typ,
                            &arg.name,
                        )?;

                        sanitize_identifier(&arg, replace)?;
                        ret = ret.replace(&pattern, replace);
                        args_to_skip.push(arg.name.to_string());
                    }
                    typ if typ == SANITIZED_DYN_IDENTIFIER_STR => {
                        let replace: String = serde_json::from_str(
                            args_map.get(&arg.name).ok_or(anyhow!("asdasdd"))?.get(),
                        )?;

                        sanitize_identifier(&arg, &replace)?;
                        ret = ret.replace(&pattern, &replace);
                        args_to_skip.push(arg.name.to_string());
                    }
                    _ => continue,
                }
            }
        }
    }

    Ok((ret, args_to_skip))
}
