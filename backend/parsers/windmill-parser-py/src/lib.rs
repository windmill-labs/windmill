/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use itertools::Itertools;

use serde_json::json;
use windmill_parser::{json_to_typ, Arg, MainArgSignature, Typ};

use rustpython_parser::ast::{Constant, ExprKind, Located, StmtKind};
use rustpython_parser::parser::parse_program;

const DEF_MAIN: &str = "def main(";
const FUNCTION_CALL: &str = "<function call>";

fn filter_non_main(code: &str) -> String {
    let mut filtered_code = String::new();
    let mut code_iter = code.split("\n");
    let mut remaining: String = String::new();
    while let Some(line) = code_iter.next() {
        if line.starts_with(DEF_MAIN) {
            filtered_code += DEF_MAIN;
            remaining += line.strip_prefix(DEF_MAIN).unwrap();
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

    filtered_code.push_str(": return");
    return filtered_code;
}

pub fn parse_python_signature(code: &str) -> anyhow::Result<MainArgSignature> {
    let filtered_code = filter_non_main(code);
    if filtered_code.is_empty() {
        return Err(anyhow::anyhow!("No main function found".to_string(),));
    }
    let ast = parse_program(&filtered_code, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;
    let param = ast.into_iter().find_map(|x| match x {
        Located { node: StmtKind::FunctionDef { name, args, .. }, .. } if &name == "main" => {
            Some(*args)
        }
        _ => None,
    });
    if let Some(params) = param {
        //println!("{:?}", params);
        let def_arg_start = params.args.len() - params.defaults.len();
        Ok(MainArgSignature {
            star_args: params.vararg.is_some(),
            star_kwargs: params.kwarg.is_some(),
            args: params
                .args
                .into_iter()
                .enumerate()
                .map(|(i, x)| {
                    let x = x.node;
                    let default = if i >= def_arg_start {
                        to_value(&params.defaults[i - def_arg_start].node)
                    } else {
                        None
                    };

                    let mut typ = x.annotation.map_or(Typ::Unknown, |e| match *e {
                        Located { node: ExprKind::Name { id, .. }, .. } => match id.as_ref() {
                            "str" => Typ::Str(None),
                            "float" => Typ::Float,
                            "int" => Typ::Int,
                            "bool" => Typ::Bool,
                            "dict" => Typ::Object(vec![]),
                            "list" => Typ::List(Box::new(Typ::Str(None))),
                            "bytes" => Typ::Bytes,
                            "datetime" => Typ::Datetime,
                            "datetime.datetime" => Typ::Datetime,
                            "Sql" | "sql" => Typ::Sql,
                            _ => Typ::Resource(id),
                        },
                        _ => Typ::Unknown,
                    });

                    if typ == Typ::Unknown
                        && default.is_some()
                        && default != Some(json!(FUNCTION_CALL))
                    {
                        typ = json_to_typ(default.as_ref().unwrap());
                    }

                    Arg { otyp: None, name: x.arg, typ, has_default: default.is_some(), default }
                })
                .collect(),
        })
    } else {
        Err(anyhow::anyhow!(
            "main function was not findable".to_string(),
        ))
    }
}

fn to_value(et: &ExprKind) -> Option<serde_json::Value> {
    match et {
        ExprKind::Constant { value, .. } => Some(constant_to_value(value)),
        ExprKind::Dict { keys, values } => {
            let v = keys
                .into_iter()
                .zip(values)
                .map(|(k, v)| {
                    let key = to_value(&k.node)
                        .and_then(|x| match x {
                            serde_json::Value::String(s) => Some(s),
                            _ => None,
                        })
                        .unwrap_or_else(|| "no_key".to_string());
                    (key, to_value(&v.node))
                })
                .collect::<HashMap<String, _>>();
            Some(json!(v))
        }
        ExprKind::List { elts, .. } => {
            let v = elts
                .into_iter()
                .map(|x| to_value(&x.node))
                .collect::<Vec<_>>();
            Some(json!(v))
        }
        ExprKind::Call { .. } => Some(json!(FUNCTION_CALL)),
        _ => None,
    }
}

fn constant_to_value(c: &Constant) -> serde_json::Value {
    match c {
        Constant::None => json!(null),
        Constant::Bool(b) => json!(b),
        Constant::Str(s) => json!(s),
        Constant::Bytes(b) => json!(b),
        Constant::Int(i) => serde_json::from_str(&i.to_string()).unwrap_or(json!("invalid number")),
        Constant::Tuple(t) => json!(t.iter().map(constant_to_value).collect::<Vec<_>>()),
        Constant::Float(f) => json!(f),
        Constant::Complex { real, imag } => json!([real, imag]),
        Constant::Ellipsis => json!("..."),
    }
}

#[cfg(test)]
mod tests {

    use serde_json::json;

    use super::*;

    #[test]
    fn test_parse_python_sig() -> anyhow::Result<()> {
        let code = "

import os

def main(test1: str, name: datetime.datetime = datetime.now(), byte: bytes = bytes(1), f = \"wewe\", g = 21, h = [1,2], i = True):

	print(f\"Hello World and a warm welcome especially to {name}\")
	print(\"The env variable at `all/pretty_secret`: \", os.environ.get(\"ALL_PRETTY_SECRET\"))
	return {\"len\": len(name), \"splitted\": name.split() }

";
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Unknown,
                        default: Some(json!("<function call>")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "f".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("wewe")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "g".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(21)),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "h".to_string(),
                        typ: Typ::List(Box::new(Typ::Int)),
                        default: Some(json!([1, 2])),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "i".to_string(),
                        typ: Typ::Bool,
                        default: Some(json!(true)),
                        has_default: true
                    },
                ]
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_2() -> anyhow::Result<()> {
        let code = "

import os

postgresql = dict
def main(test1: str,
    name: datetime.datetime = datetime.now(),
    byte: bytes = bytes(1),
    resource: postgresql = \"$res:g/all/resource\"):

	print(f\"Hello World and a warm welcome especially to {name}\")
	print(\"The env variable at `all/pretty_secret`: \", os.environ.get(\"ALL_PRETTY_SECRET\"))
	return {\"len\": len(name), \"splitted\": name.split() }

";
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Unknown,
                        default: Some(json!("<function call>")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "resource".to_string(),
                        typ: Typ::Resource("postgresql".to_string()),
                        default: Some(json!("$res:g/all/resource")),
                        has_default: true
                    }
                ]
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_3() -> anyhow::Result<()> {
        let code = "

import os

def main(test1: str,
    name = \"test\",
    byte: bytes = bytes(1)): return

";
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("test")),
                        has_default: true
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true
                    }
                ]
            }
        );

        Ok(())
    }
}
