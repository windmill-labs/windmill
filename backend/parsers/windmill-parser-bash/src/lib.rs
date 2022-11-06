#![allow(non_snake_case)] // TODO: switch to parse_* function naming

use itertools::Itertools;

use std::fmt;
use windmill_common::error::to_anyhow;
use windmill_parser::{Arg, MainArgSignature, ObjectProperty, Typ};

pub fn parse_bash_sig(code: &str) -> windmill_common::error::Result<MainArgSignature> {
    let filtered_code = filter_non_main(code);
    let parsed = parse_file(&filtered_code)?;
    if let Some(x) = parsed {
        let args = x;
        Ok(MainArgSignature { star_args: false, star_kwargs: false, args })
    } else {
        Err(windmill_common::error::Error::BadRequest(
            "no main function found".to_string(),
        ))
    }
}

fn parse_file(filtered_code: &str) -> anyhow::Result<Option<Vec<Arg>>> {
    println!("parsing file {}", filtered_code);
    todo!()
}

fn filter_non_main(code: &str) -> String {
    const FUNC_MAIN: &str = "function main(";

    let mut filtered_code = String::new();
    let mut code_iter = code.split("\n");
    let mut remaining: String = String::new();
    while let Some(line) = code_iter.next() {
        if line.starts_with(FUNC_MAIN) {
            filtered_code += FUNC_MAIN;
            remaining += line.strip_prefix(FUNC_MAIN).unwrap();
            remaining += &code_iter.join("\n");
            break;
        }
    }
    if filtered_code.is_empty() {
        return String::new();
    }
    let mut chars = remaining.chars();
    let mut open_parens = 1;

    while let Some(c) = chars.next() {
        if c == '(' {
            open_parens += 1;
        } else if c == ')' {
            open_parens -= 1;
        }
        filtered_code.push(c);
        if open_parens == 0 {
            break;
        }
    }
    return filtered_code;
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_bash_sig() -> anyhow::Result<()> {
        let code = "
function main(
    $1 = "token"
) {
}
";
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_bash_sig(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    otyp: None,
                    name: "test1".to_string(),
                    typ: Typ::Str(None),
                    default: None,
                    has_default: false
                }]
            }
        );

        Ok(())
    }
}
