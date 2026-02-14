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
use windmill_parser::{json_to_typ, Arg, MainArgSignature, ObjectType, Typ};

use rustpython_parser::{
    ast::{
        Constant, Expr, ExprAttribute, ExprConstant, ExprDict, ExprList, ExprName, Stmt,
        StmtAssign, StmtClassDef, StmtFunctionDef, Suite,
    },
    Parse,
};

pub mod asset_parser;
pub mod pydantic_parser;

pub use asset_parser::parse_assets;

const FUNCTION_CALL: &str = "<function call>";

/// Cheap string-based check to see if code might contain Pydantic models or dataclasses.
/// Returns true if we should do full AST parsing for type detection, false otherwise.
/// This avoids expensive parsing for the common case where scripts don't use these features.
fn should_parse_for_models(code: &str) -> bool {
    code.contains("BaseModel")
        || code.contains("from pydantic")
        || code.contains("import pydantic")
        || code.contains("@dataclass")
        || code.contains("from dataclasses")
        || code.contains("import dataclasses")
}

fn filter_non_main(code: &str, main_name: &str) -> String {
    let def_main = format!("def {}(", main_name);
    let mut filtered_code = String::new();
    let mut code_iter = code.split("\n");
    let mut remaining: String = String::new();
    while let Some(line) = code_iter.next() {
        if line.starts_with(&def_main) {
            filtered_code += &def_main;
            remaining += line.strip_prefix(&def_main).unwrap();
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

/// Data extracted from parsing the Python code
struct CodeMetadata {
    enums: HashMap<String, EnumInfo>,
    descriptions: HashMap<String, String>,
}

/// Information about an Enum class
struct EnumInfo {
    values: Vec<String>,
    members: HashMap<String, String>,
}

fn has_enum_keyword(code: &str) -> bool {
    code.contains("Enum")
}

/// Extract only class and function definitions from code (prepass filtering)
fn filter_relevant_statements(code: &str) -> String {
    let mut result = Vec::new();
    let mut lines = code.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("class ") || trimmed.starts_with("def ") {
            result.push(line);
            let base_indent = line.len() - trimmed.len();

            while let Some(&next_line) = lines.peek() {
                let next_trimmed = next_line.trim_start();
                let next_indent = next_line.len() - next_trimmed.len();

                if next_trimmed.is_empty() || next_indent > base_indent {
                    result.push(lines.next().unwrap());
                } else {
                    break;
                }
            }
        }
    }

    result.join("\n")
}

/// Extract Enum definitions and docstring descriptions lazily.
/// Only parses AST if relevant keywords are present.
fn extract_code_metadata(code: &str, main_name: &str) -> CodeMetadata {
    let mut enums = HashMap::new();
    let mut descriptions = HashMap::new();

    let has_enum = has_enum_keyword(code);
    let has_docstring = code.contains("Args:");

    if !has_enum && !has_docstring {
        return CodeMetadata { enums, descriptions };
    }

    let filtered_code = filter_relevant_statements(code);

    let ast = match Suite::parse(&filtered_code, "main.py") {
        Ok(ast) => ast,
        Err(_) => return CodeMetadata { enums, descriptions },
    };

    for stmt in ast {
        match stmt {
            Stmt::ClassDef(StmtClassDef { name, body, bases, .. }) if has_enum => {
                let is_enum = bases.iter().any(|base| {
                    matches!(base, Expr::Name(ExprName { id, .. })
                        if id == "Enum" || id == "IntEnum" || id == "StrEnum"
                        || id == "Flag" || id == "IntFlag")
                });

                if is_enum {
                    let mut values = Vec::new();
                    let mut members = HashMap::new();

                    for item in body {
                        if let Stmt::Assign(StmtAssign { targets, value, .. }) = item {
                            if let Some(Expr::Name(ExprName { id: target_name, .. })) =
                                targets.first()
                            {
                                if !target_name.starts_with('_') {
                                    if let Expr::Constant(ExprConstant {
                                        value: Constant::Str(val),
                                        ..
                                    }) = value.as_ref()
                                    {
                                        values.push(val.to_string());
                                        members.insert(target_name.to_string(), val.to_string());
                                    }
                                }
                            }
                        }
                    }

                    if !values.is_empty() {
                        enums.insert(name.to_string(), EnumInfo { values, members });
                    }
                }
            }
            Stmt::FunctionDef(StmtFunctionDef { name: func_name, body, .. }) if has_docstring => {
                if &func_name == main_name {
                    if let Some(Stmt::Expr(expr_stmt)) = body.first() {
                        if let Expr::Constant(ExprConstant {
                            value: Constant::Str(docstring),
                            ..
                        }) = expr_stmt.value.as_ref()
                        {
                            descriptions = parse_docstring_args(docstring);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    CodeMetadata { enums, descriptions }
}

/// Parse docstring Args: section (format: "param_name (type): Description")
fn parse_docstring_args(docstring: &str) -> HashMap<String, String> {
    let mut descriptions = HashMap::new();
    let mut in_args_section = false;
    let mut base_indent: Option<usize> = None;

    for line in docstring.lines() {
        let trimmed = line.trim();

        if trimmed == "Args:" {
            in_args_section = true;
            base_indent = None;
            continue;
        }

        if in_args_section {
            if trimmed.is_empty() {
                continue;
            }

            let indent = line.len() - line.trim_start().len();

            if base_indent.is_none() && !trimmed.is_empty() {
                base_indent = Some(indent);
            }

            if let Some(base) = base_indent {
                if indent < base && trimmed.ends_with(':') {
                    break;
                }
            }

            if let Some(colon_pos) = trimmed.find(':') {
                let before_colon = &trimmed[..colon_pos];
                let description = trimmed[colon_pos + 1..].trim();

                if let Some(paren_pos) = before_colon.find('(') {
                    let param_name = before_colon[..paren_pos].trim();
                    descriptions.insert(param_name.to_string(), description.to_string());
                } else {
                    descriptions.insert(before_colon.trim().to_string(), description.to_string());
                }
            }
        }
    }

    descriptions
}

/// skip_params is a micro optimization for when we just want to find the main
/// function without parsing all the params.
pub fn parse_python_signature(
    code: &str,
    override_main: Option<String>,
    skip_params: bool,
) -> anyhow::Result<MainArgSignature> {
    let main_name = override_main.unwrap_or("main".to_string());

    let has_preprocessor = !filter_non_main(code, "preprocessor").is_empty();

    // Optimization: Parse code only once
    // - If models detected: parse full code, extract main from it, keep AST for type detection
    // - If no models: parse only the filtered main function
    let (params, module) = if should_parse_for_models(code) {
        // Parse full code once for both Pydantic detection and signature extraction
        let ast = Suite::parse(code, "main.py")
            .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

        // Extract main function from full AST
        let params = ast.iter().find_map(|x| match x {
            Stmt::FunctionDef(StmtFunctionDef { name, args, .. }) if name == &main_name => {
                Some(args.as_ref().clone())
            }
            _ => None,
        });

        // Keep AST for Pydantic/dataclass detection
        (params, Some(ast))
    } else {
        // No models detected - parse only the filtered main function
        let filtered_code = filter_non_main(code, &main_name);
        if filtered_code.is_empty() {
            (None, None)
        } else {
            let ast = Suite::parse(&filtered_code, "main.py")
                .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

            let params = ast.into_iter().find_map(|x| match x {
                Stmt::FunctionDef(StmtFunctionDef { name, args, .. }) if &name == &main_name => {
                    Some(*args)
                }
                _ => None,
            });

            (params, None)
        }
    };

    // Check if main function was found
    if params.is_none() {
        return Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![],
            no_main_func: Some(true),
            has_preprocessor: Some(has_preprocessor),
        });
    }

    if !skip_params && params.is_some() {
        let params = params.unwrap();
        let def_arg_start = params.args.len() - params.defaults().count();

        // Two-pass approach for lazy metadata extraction:
        // Pass 1: Parse types without enum info to determine if metadata is needed
        // Pass 2: Re-parse unknown types with metadata only if necessary
        // This ensures zero overhead for scripts without enums/docstrings

        let empty_enums = HashMap::new();
        let module_ref = module.as_ref().map(|m| m.as_slice());
        let args_first_pass: Vec<_> = params
            .args
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let arg_name = x.as_arg().arg.to_string();
                let (typ, has_default) = x
                    .as_arg()
                    .annotation
                    .as_ref()
                    .map_or((Typ::Unknown, false), |e| {
                        parse_expr(e, &empty_enums, module_ref)
                    });
                (i, arg_name, typ, has_default)
            })
            .collect();

        // Determine if we need to extract metadata from the code
        let has_potential_enums = args_first_pass
            .iter()
            .any(|(_, _, typ, _)| matches!(typ, Typ::Resource(_)));

        let metadata = if has_potential_enums || code.contains("Args:") {
            extract_code_metadata(code, &main_name)
        } else {
            CodeMetadata { enums: HashMap::new(), descriptions: HashMap::new() }
        };

        // Build final args, re-parsing Resource types as enums if metadata was extracted
        Ok(MainArgSignature {
            star_args: params.vararg.is_some(),
            star_kwargs: params.kwarg.is_some(),
            args: args_first_pass
                .into_iter()
                .map(|(i, arg_name, mut typ, mut has_default)| {
                    if matches!(typ, Typ::Resource(_)) && !metadata.enums.is_empty() {
                        if let Some(annotation) = params.args[i].as_arg().annotation.as_ref() {
                            (typ, has_default) =
                                parse_expr(annotation, &metadata.enums, module_ref);
                        }
                    }

                    let default = if i >= def_arg_start {
                        params
                            .defaults()
                            .nth(i - def_arg_start)
                            .map(|expr| to_value(expr, &metadata.enums))
                            .flatten()
                    } else {
                        None
                    };

                    let should_get_type_from_default = match &typ {
                        Typ::Unknown => true,
                        // if the type is a list of unknowns, we should get the type from the default
                        Typ::List(inner) => matches!(inner.as_ref(), Typ::Unknown),
                        _ => false,
                    };

                    if should_get_type_from_default
                        && default.is_some()
                        && default != Some(json!(FUNCTION_CALL))
                    {
                        typ = json_to_typ(default.as_ref().unwrap(), false);
                    }

                    // if the type is still a list of unknowns after checking the default, we set it to a list of strings to not break past behavior
                    match typ {
                        Typ::List(inner) if matches!(inner.as_ref(), Typ::Unknown) => {
                            typ = Typ::List(Box::new(Typ::Str(None)));
                        }
                        _ => {}
                    }

                    Arg {
                        otyp: metadata.descriptions.get(&arg_name).map(|d| d.to_string()),
                        name: arg_name,
                        typ,
                        has_default: has_default || default.is_some(),
                        default,
                        oidx: None,
                    }
                })
                .collect(),
            no_main_func: Some(false),
            has_preprocessor: Some(has_preprocessor),
        })
    } else {
        Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![],
            no_main_func: Some(params.is_none()),
            has_preprocessor: Some(has_preprocessor),
        })
    }
}

fn parse_expr(
    e: &Box<Expr>,
    enums: &HashMap<String, EnumInfo>,
    module: Option<&[Stmt]>,
) -> (Typ, bool) {
    match e.as_ref() {
        Expr::Name(ExprName { id, .. }) => (parse_typ(id.as_ref(), enums, module), false),
        Expr::Attribute(x) => {
            if let Some(name) = x.value.as_name_expr() {
                match name.id.as_str() {
                    "wmill" => (parse_typ(x.attr.as_str(), enums, module), false),
                    "datetime" => {
                        let full_name = format!("datetime.{}", x.attr.as_str());
                        (parse_typ(&full_name, enums, module), false)
                    }
                    _ => (Typ::Unknown, false),
                }
            } else {
                (Typ::Unknown, false)
            }
        }
        Expr::BinOp(x) => {
            if matches!(
                x.right.as_ref(),
                Expr::Constant(ExprConstant { value: Constant::None, .. })
            ) {
                (parse_expr(&x.left, enums, module).0, true)
            } else {
                (Typ::Unknown, false)
            }
        }
        Expr::Subscript(x) => match x.value.as_ref() {
            Expr::Name(ExprName { id, .. }) => match id.as_str() {
                "Literal" => {
                    let values = match x.slice.as_ref() {
                        Expr::Tuple(elts) => {
                            let v: Vec<String> = elts
                                .elts
                                .iter()
                                .map(|x| match x {
                                    Expr::Constant(c) => c.value.as_str().map(|x| x.to_string()),
                                    _ => None,
                                })
                                .filter_map(|x| x)
                                .collect();
                            if v.is_empty() {
                                None
                            } else {
                                Some(v)
                            }
                        }
                        _ => None,
                    };
                    (Typ::Str(values), false)
                }
                "List" | "list" => (
                    Typ::List(Box::new(parse_expr(&x.slice, enums, module).0)),
                    false,
                ),
                "Optional" => (parse_expr(&x.slice, enums, module).0, true),
                _ => (Typ::Unknown, false),
            },
            _ => (Typ::Unknown, false),
        },
        _ => (Typ::Unknown, false),
    }
}

fn parse_typ(id: &str, enums: &HashMap<String, EnumInfo>, module: Option<&[Stmt]>) -> Typ {
    if let Some(enum_info) = enums.get(id) {
        return Typ::Str(Some(enum_info.values.clone()));
    }

    match id {
        "str" => Typ::Str(None),
        "float" => Typ::Float,
        "int" => Typ::Int,
        "bool" => Typ::Bool,
        "dict" => Typ::Object(ObjectType::new(None, Some(vec![]))),
        "list" => Typ::List(Box::new(Typ::Unknown)),
        "bytes" => Typ::Bytes,
        "datetime" => Typ::Datetime,
        "datetime.datetime" => Typ::Datetime,
        "date" => Typ::Date,
        "datetime.date" => Typ::Date,
        "Sql" | "sql" => Typ::Sql,
        x @ _ if x.starts_with("DynSelect_") => {
            Typ::DynSelect(x.strip_prefix("DynSelect_").unwrap().to_string())
        }
        x @ _ if x.starts_with("DynMultiselect_") => {
            Typ::DynMultiselect(x.strip_prefix("DynMultiselect_").unwrap().to_string())
        }
        _ => {
            // Check if it's a Pydantic model or dataclass
            if let Some(module) = module {
                if let Some(object_type) = pydantic_parser::detect_model_type(id, module) {
                    return Typ::Object(object_type);
                }
            }

            // Fallback to Resource if not a model
            Typ::Resource(map_resource_name(id))
        }
    }
}

fn map_resource_name(x: &str) -> String {
    match x {
        "S3Object" => "s3_object".to_string(),
        _ => x.to_string(),
    }
}

fn to_value<R>(et: &Expr<R>, enums: &HashMap<String, EnumInfo>) -> Option<serde_json::Value> {
    match et {
        Expr::Constant(ExprConstant { value, .. }) => Some(constant_to_value(value)),
        Expr::Dict(ExprDict { keys, values, .. }) => {
            let v = keys
                .into_iter()
                .zip(values)
                .map(|(k, v)| {
                    let key = k
                        .as_ref()
                        .map(|e| to_value(e, enums))
                        .flatten()
                        .and_then(|x| match x {
                            serde_json::Value::String(s) => Some(s),
                            _ => None,
                        })
                        .unwrap_or_else(|| "no_key".to_string());
                    (key, to_value(&v, enums))
                })
                .collect::<HashMap<String, _>>();
            Some(json!(v))
        }
        Expr::List(ExprList { elts, .. }) => {
            let v = elts
                .into_iter()
                .map(|x| to_value(&x, enums))
                .collect::<Vec<_>>();
            Some(json!(v))
        }
        Expr::Attribute(ExprAttribute { value, attr, .. }) => {
            // Handle Enum.MEMBER: returns enum value ("red") not member name ("RED")
            if let Expr::Name(ExprName { id: enum_name, .. }) = value.as_ref() {
                if let Some(enum_info) = enums.get(enum_name.as_str()) {
                    if let Some(enum_value) = enum_info.members.get(attr.as_str()) {
                        return Some(json!(enum_value));
                    }
                }
                Some(json!(attr.as_str()))
            } else {
                None
            }
        }
        Expr::Call { .. } => Some(json!(FUNCTION_CALL)),
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
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Datetime,
                        default: Some(json!("<function call>")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "f".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("wewe")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "g".to_string(),
                        typ: Typ::Int,
                        default: Some(json!(21)),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "h".to_string(),
                        typ: Typ::List(Box::new(Typ::Int)),
                        default: Some(json!([1, 2])),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "i".to_string(),
                        typ: Typ::Bool,
                        default: Some(json!(true)),
                        has_default: true,
                        oidx: None
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false)
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
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Datetime,
                        default: Some(json!("<function call>")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "resource".to_string(),
                        typ: Typ::Resource("postgresql".to_string()),
                        default: Some(json!("$res:g/all/resource")),
                        has_default: true,
                        oidx: None
                    }
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false)
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_3() -> anyhow::Result<()> {
        let code = "

import os

def main(test1: str,
    s3o: wmill.S3Object,
    name = \"test\",
    byte: bytes = bytes(1)): return

";
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "s3o".to_string(),
                        typ: Typ::Resource("s3_object".to_string()),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "name".to_string(),
                        typ: Typ::Str(None),
                        default: Some(json!("test")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "byte".to_string(),
                        typ: Typ::Bytes,
                        default: Some(json!("<function call>")),
                        has_default: true,
                        oidx: None
                    }
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false)
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_4() -> anyhow::Result<()> {
        let code = r#"

import os

def main(test1: Literal["foo", "bar"], test2: List[Literal["foo", "bar"]]): return

"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "test1".to_string(),
                        typ: Typ::Str(Some(vec!["foo".to_string(), "bar".to_string()])),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "test2".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(Some(vec![
                            "foo".to_string(),
                            "bar".to_string()
                        ])))),
                        default: None,
                        has_default: false,
                        oidx: None
                    }
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false)
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_5() -> anyhow::Result<()> {
        let code = r#"

import os

def main(test1: DynSelect_foo): return

"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    otyp: None,
                    name: "test1".to_string(),
                    typ: Typ::DynSelect("foo".to_string()),
                    default: None,
                    has_default: false,
                    oidx: None
                }],
                no_main_func: Some(false),
                has_preprocessor: Some(false)
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_6() -> anyhow::Result<()> {
        let code = r#"

import os

def hello(): return

"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![],
                no_main_func: Some(true),
                has_preprocessor: Some(false)
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_7() -> anyhow::Result<()> {
        let code = r#"

import os

def preprocessor(): return

def main(): return



"#;
        //println!("{}", serde_json::to_string()?);
        assert_eq!(
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![],
                no_main_func: Some(false),
                has_preprocessor: Some(true)
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_8() -> anyhow::Result<()> {
        let code = r#"
from typing import List
def main(a: list, e: List[int], b: list = [1,2,3,4], c = [1,2,3,4], d = ["a", "b"]): return
"#;
        println!(
            "{}",
            serde_json::to_string(&parse_python_signature(code, None, false)?)?
        );
        assert_eq!(
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "a".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "e".to_string(),
                        typ: Typ::List(Box::new(Typ::Int)),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "b".to_string(),
                        typ: Typ::List(Box::new(Typ::Int)),
                        default: Some(json!([1, 2, 3, 4])),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "c".to_string(),
                        typ: Typ::List(Box::new(Typ::Int)),
                        default: Some(json!([1, 2, 3, 4])),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "d".to_string(),
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: Some(json!(["a", "b"])),
                        has_default: true,
                        oidx: None
                    }
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false)
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_9() -> anyhow::Result<()> {
        let code = r#"
from typing import Optional
def main(a: str, b: Optional[str], c: str | None): return
"#;
        println!(
            "{}",
            serde_json::to_string(&parse_python_signature(code, None, false)?)?
        );
        assert_eq!(
            parse_python_signature(code, None, false)?,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        otyp: None,
                        name: "a".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "b".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        otyp: None,
                        name: "c".to_string(),
                        typ: Typ::Str(None),
                        default: None,
                        has_default: true,
                        oidx: None
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false)
            }
        );

        Ok(())
    }

    #[test]
    fn test_parse_python_sig_enum() -> anyhow::Result<()> {
        let code = r#"
from enum import Enum

class Color(str, Enum):
    RED = 'red'
    GREEN = 'green'
    BLUE = 'blue'

def main(color: Color = Color.RED):
    """
    Test enum parsing

    Args:
        color (Color): Color selection from Color enum
    """
    return {"color": color}
"#;
        let result = parse_python_signature(code, None, false)?;
        assert_eq!(result.args.len(), 1);
        assert_eq!(result.args[0].name, "color");
        assert_eq!(
            result.args[0].typ,
            Typ::Str(Some(vec![
                "red".to_string(),
                "green".to_string(),
                "blue".to_string()
            ]))
        );
        assert_eq!(result.args[0].default, Some(json!("red")));
        assert_eq!(
            result.args[0].otyp,
            Some("Color selection from Color enum".to_string())
        );
        Ok(())
    }
}
