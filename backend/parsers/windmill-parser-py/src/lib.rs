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
        Constant, Expr, ExprAttribute, ExprCall, ExprConstant, ExprDict, ExprList, ExprName,
        Stmt, StmtClassDef, StmtFunctionDef, Suite,
    },
    Parse,
};

pub mod asset_parser;
pub use asset_parser::parse_assets;

const FUNCTION_CALL: &str = "<function call>";

// Thread-local storage for AST module to enable Pydantic/dataclass detection
// Uses a STACK to support recursive calls without race conditions
use std::cell::RefCell;

thread_local! {
    static MODULE_STACK: RefCell<Vec<Vec<Stmt>>> = RefCell::new(Vec::new());
}

/// Push a module onto the stack
fn set_current_module(module: Vec<Stmt>) {
    MODULE_STACK.with(|stack| {
        stack.borrow_mut().push(module);
    });
}

/// Get a clone of the current (top of stack) module
fn get_current_module() -> Option<Vec<Stmt>> {
    MODULE_STACK.with(|stack| {
        stack.borrow().last().cloned()
    })
}

/// Pop the current module from the stack
fn clear_current_module() {
    MODULE_STACK.with(|stack| {
        stack.borrow_mut().pop();
    });
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

/// RAII guard to ensure thread-local module is always cleaned up
struct ModuleGuard;

impl ModuleGuard {
    fn new(module: Vec<Stmt>) -> Self {
        set_current_module(module);
        Self
    }
}

impl Drop for ModuleGuard {
    fn drop(&mut self) {
        clear_current_module();
    }
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

    // Parse full code to get all class definitions for Pydantic/dataclass detection
    // Use RAII guard to ensure cleanup even on early return
    let _guard = if let Ok(full_ast) = Suite::parse(code, "main.py") {
        Some(ModuleGuard::new(full_ast))
    } else {
        None
    };

    let filtered_code = filter_non_main(code, &main_name);
    if filtered_code.is_empty() {
        return Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![],
            no_main_func: Some(true),
            has_preprocessor: Some(has_preprocessor),
        });
    }
    let ast = Suite::parse(&filtered_code, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let params = ast.into_iter().find_map(|x| match x {
        Stmt::FunctionDef(StmtFunctionDef { name, args, .. }) if &name == &main_name => Some(*args),
        _ => None,
    });

    if !skip_params && params.is_some() {
        let params = params.unwrap();
        //println!("{:?}", params);
        let def_arg_start = params.args.len() - params.defaults().count();
        Ok(MainArgSignature {
            star_args: params.vararg.is_some(),
            star_kwargs: params.kwarg.is_some(),
            args: params
                .args
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    let (mut typ, has_default) = x
                        .as_arg()
                        .annotation
                        .as_ref()
                        .map_or((Typ::Unknown, false), |e| parse_expr(e));

                    let default = if i >= def_arg_start {
                        params
                            .defaults()
                            .nth(i - def_arg_start)
                            .map(to_value)
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
                        otyp: None,
                        name: x.as_arg().arg.to_string(),
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
    // ModuleGuard automatically calls clear_current_module() when _guard goes out of scope
}

fn parse_expr(e: &Box<Expr>) -> (Typ, bool) {
    match e.as_ref() {
        Expr::Name(ExprName { id, .. }) => (parse_typ(id.as_ref()), false),
        Expr::Attribute(x) => {
            if x.value
                .as_name_expr()
                .is_some_and(|x| x.id.as_str() == "wmill")
            {
                (parse_typ(x.attr.as_str()), false)
            } else {
                (Typ::Unknown, false)
            }
        }
        Expr::BinOp(x) => {
            if matches!(
                x.right.as_ref(),
                Expr::Constant(ExprConstant { value: Constant::None, .. })
            ) {
                (parse_expr(&x.left).0, true)
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
                "List" | "list" => (Typ::List(Box::new(parse_expr(&x.slice).0)), false),
                "Optional" => (parse_expr(&x.slice).0, true),
                _ => (Typ::Unknown, false),
            },
            _ => (Typ::Unknown, false),
        },
        _ => (Typ::Unknown, false),
    }
}

fn parse_typ(id: &str) -> Typ {
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
        "Sql" | "sql" => Typ::Sql,
        x @ _ if x.starts_with("DynSelect_") => {
            Typ::DynSelect(x.strip_prefix("DynSelect_").unwrap().to_string())
        }
        x @ _ if x.starts_with("DynMultiselect_") => {
            Typ::DynMultiselect(x.strip_prefix("DynMultiselect_").unwrap().to_string())
        }
        _ => {
            // Check if it's a Pydantic model or dataclass
            if let Some(module) = get_current_module() {
                if let Some(object_type) = detect_model_type(id, &module) {
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

// ============================================================================
// Pydantic/Dataclass Detection Functions
// ============================================================================

/// Maximum number of fields allowed in a Pydantic model or dataclass
/// This prevents malicious code from defining models with thousands of fields
const MAX_MODEL_FIELDS: usize = 200;

/// Detects if a class name refers to a Pydantic model or dataclass
/// Returns ObjectType with parsed fields if detected, None otherwise
fn detect_model_type(class_name: &str, module: &[Stmt]) -> Option<ObjectType> {
    // Find class definition in module
    for stmt in module {
        if let Stmt::ClassDef(class_def) = stmt {
            if class_def.name.as_str() == class_name {
                // Check if it's a Pydantic model (inherits from BaseModel)
                if is_pydantic_base(&class_def.bases) {
                    return parse_pydantic_model(class_def);
                }

                // Check if it's a dataclass (has @dataclass decorator)
                if has_dataclass_decorator(&class_def.decorator_list) {
                    return parse_dataclass_fields(&class_def.body, class_def.name.as_str());
                }

                // Found class but it's neither Pydantic nor dataclass
                return None;
            }
        }
    }

    // Class not found in module
    None
}

/// Checks if a class inherits from BaseModel or pydantic.BaseModel
fn is_pydantic_base(bases: &[Expr]) -> bool {
    for base in bases {
        match base {
            // Match: class User(BaseModel)
            Expr::Name(ExprName { id, .. }) => {
                if id.as_str() == "BaseModel" {
                    return true;
                }
            }

            // Match: class User(pydantic.BaseModel)
            Expr::Attribute(ExprAttribute { attr, value, .. }) => {
                if attr.as_str() == "BaseModel" {
                    if let Expr::Name(ExprName { id, .. }) = value.as_ref() {
                        if id.as_str() == "pydantic" {
                            return true;
                        }
                    }
                }
            }

            _ => {}
        }
    }

    false
}

/// Checks if a class has @dataclass decorator
fn has_dataclass_decorator(decorators: &[Expr]) -> bool {
    for decorator in decorators {
        match decorator {
            // Match: @dataclass
            Expr::Name(ExprName { id, .. }) => {
                if id.as_str() == "dataclass" {
                    return true;
                }
            }

            // Match: @dataclasses.dataclass
            Expr::Attribute(ExprAttribute { attr, value, .. }) => {
                if attr.as_str() == "dataclass" {
                    if let Expr::Name(ExprName { id, .. }) = value.as_ref() {
                        if id.as_str() == "dataclasses" {
                            return true;
                        }
                    }
                }
            }

            // Match: @dataclass() or @dataclass(frozen=True)
            Expr::Call(ExprCall { func, .. }) => {
                if let Expr::Name(ExprName { id, .. }) = func.as_ref() {
                    if id.as_str() == "dataclass" {
                        return true;
                    }
                }

                // Also check for @dataclasses.dataclass(...)
                if let Expr::Attribute(ExprAttribute { attr, value, .. }) = func.as_ref() {
                    if attr.as_str() == "dataclass" {
                        if let Expr::Name(ExprName { id, .. }) = value.as_ref() {
                            if id.as_str() == "dataclasses" {
                                return true;
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    false
}

/// Parses Pydantic model fields from class body
fn parse_pydantic_model(class_def: &StmtClassDef) -> Option<ObjectType> {
    let mut properties = Vec::new();

    for stmt in &class_def.body {
        // Extract annotated assignments: field_name: field_type
        if let Stmt::AnnAssign(ann_assign) = stmt {
            if let Expr::Name(ExprName { id: field_name, .. }) = ann_assign.target.as_ref() {
                // Security: Enforce field count limit
                if properties.len() >= MAX_MODEL_FIELDS {
                    // Log warning and return truncated model
                    eprintln!(
                        "Warning: Model '{}' exceeds maximum field count ({}). Truncating to {} fields.",
                        class_def.name, MAX_MODEL_FIELDS, MAX_MODEL_FIELDS
                    );
                    break;
                }

                // Extract field type from annotation
                let field_type = extract_field_type(&ann_assign.annotation, 0);

                properties.push(windmill_parser::ObjectProperty {
                    key: field_name.to_string(),
                    typ: Box::new(field_type),
                });
            }
        }

        // Skip other statements (methods, class variables without annotations, etc.)
    }

    if properties.is_empty() {
        // Empty model - return object with no properties
        return Some(ObjectType {
            name: Some(class_def.name.to_string()),
            props: None,
        });
    }

    Some(ObjectType {
        name: Some(class_def.name.to_string()),
        props: Some(properties),
    })
}

/// Parses dataclass fields from class body (same as Pydantic)
fn parse_dataclass_fields(body: &[Stmt], class_name: &str) -> Option<ObjectType> {
    let mut properties = Vec::new();

    for stmt in body {
        // Extract annotated assignments: field_name: field_type
        if let Stmt::AnnAssign(ann_assign) = stmt {
            if let Expr::Name(ExprName { id: field_name, .. }) = ann_assign.target.as_ref() {
                // Security: Enforce field count limit
                if properties.len() >= MAX_MODEL_FIELDS {
                    // Log warning and return truncated model
                    eprintln!(
                        "Warning: Dataclass '{}' exceeds maximum field count ({}). Truncating to {} fields.",
                        class_name, MAX_MODEL_FIELDS, MAX_MODEL_FIELDS
                    );
                    break;
                }

                // Extract field type from annotation
                let field_type = extract_field_type(&ann_assign.annotation, 0);

                properties.push(windmill_parser::ObjectProperty {
                    key: field_name.to_string(),
                    typ: Box::new(field_type),
                });
            }
        }
    }

    if properties.is_empty() {
        // Empty dataclass - return object with no properties
        return Some(ObjectType {
            name: Some(class_name.to_string()),
            props: None,
        });
    }

    Some(ObjectType {
        name: Some(class_name.to_string()),
        props: Some(properties),
    })
}

/// Extracts Windmill Typ from Python type annotation (RECURSIVE)
/// depth: Current recursion depth (prevents infinite recursion)
fn extract_field_type(annotation: &Expr, depth: u8) -> Typ {
    // Prevent infinite recursion (max depth: 10)
    if depth >= 10 {
        return Typ::Unknown;
    }

    match annotation {
        // Simple types: str, int, bool, float, bytes
        Expr::Name(ExprName { id, .. }) => match id.as_str() {
            "str" => Typ::Str(None),
            "int" => Typ::Int,
            "float" => Typ::Float,
            "bool" => Typ::Bool,
            "bytes" => Typ::Bytes,
            "datetime" => Typ::Datetime,
            // Custom class - check if it's a model
            custom_type => {
                // Try to detect if this is a nested Pydantic model
                if let Some(module) = get_current_module() {
                    if let Some(object_type) = detect_model_type(custom_type, &module) {
                        return Typ::Object(object_type);
                    }
                }
                // Unknown type - return Unknown instead of Resource
                // (Resource is for runtime-resolvable resources)
                Typ::Unknown
            }
        },

        // Generic types: List[T], Optional[T], Dict[K, V]
        Expr::Subscript(subscript) => {
            if let Expr::Name(ExprName { id, .. }) = subscript.value.as_ref() {
                match id.as_str() {
                    // List[T]
                    "List" | "list" => {
                        let inner_type = extract_field_type(&subscript.slice, depth + 1);
                        Typ::List(Box::new(inner_type))
                    }

                    // Optional[T] - unwrap to T
                    "Optional" => extract_field_type(&subscript.slice, depth + 1),

                    // Dict[K, V] - return generic Object
                    "Dict" | "dict" => Typ::Object(ObjectType::new(None, Some(vec![]))),

                    // Set[T], Tuple[T], etc. - not supported in MVP
                    _ => Typ::Unknown,
                }
            } else {
                Typ::Unknown
            }
        }

        // Union types: str | int (Python 3.10+) or Union[str, int]
        // MVP: Not supported, return Unknown
        Expr::BinOp(_) => Typ::Unknown,

        // String annotations: "ForwardRef" (forward references)
        // MVP: Not supported, return Unknown
        Expr::Constant(ExprConstant {
            value: Constant::Str(_),
            ..
        }) => Typ::Unknown,

        // All other annotations
        _ => Typ::Unknown,
    }
}

// ============================================================================
// End of Pydantic/Dataclass Detection Functions
// ============================================================================

fn to_value<R>(et: &Expr<R>) -> Option<serde_json::Value> {
    match et {
        Expr::Constant(ExprConstant { value, .. }) => Some(constant_to_value(value)),
        Expr::Dict(ExprDict { keys, values, .. }) => {
            let v = keys
                .into_iter()
                .zip(values)
                .map(|(k, v)| {
                    let key = k
                        .as_ref()
                        .map(to_value)
                        .flatten()
                        .and_then(|x| match x {
                            serde_json::Value::String(s) => Some(s),
                            _ => None,
                        })
                        .unwrap_or_else(|| "no_key".to_string());
                    (key, to_value(&v))
                })
                .collect::<HashMap<String, _>>();
            Some(json!(v))
        }
        Expr::List(ExprList { elts, .. }) => {
            let v = elts.into_iter().map(|x| to_value(&x)).collect::<Vec<_>>();
            Some(json!(v))
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
                        typ: Typ::Unknown,
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
                        typ: Typ::Unknown,
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
    fn test_pydantic_basic_model() -> anyhow::Result<()> {
        let code = "
from pydantic import BaseModel

class User(BaseModel):
    name: str
    age: int
    email: str

def main(user: User):
    return f'Hello {user.name}'
";
        let result = parse_python_signature(code, None, false)?;

        // Check that user parameter is detected as Object type
        assert_eq!(result.args.len(), 1);
        assert_eq!(result.args[0].name, "user");

        // Verify it's an Object type with correct model name
        match &result.args[0].typ {
            Typ::Object(obj) => {
                assert_eq!(obj.name, Some("User".to_string()));
                assert!(obj.props.is_some());

                let props = obj.props.as_ref().unwrap();
                assert_eq!(props.len(), 3);

                // Verify field names and types
                assert_eq!(props[0].key, "name");
                assert_eq!(*props[0].typ, Typ::Str(None));

                assert_eq!(props[1].key, "age");
                assert_eq!(*props[1].typ, Typ::Int);

                assert_eq!(props[2].key, "email");
                assert_eq!(*props[2].typ, Typ::Str(None));
            }
            _ => panic!("Expected Typ::Object for Pydantic model"),
        }

        Ok(())
    }

    #[test]
    fn test_python_dataclass() -> anyhow::Result<()> {
        let code = "
from dataclasses import dataclass

@dataclass
class Config:
    host: str
    port: int
    debug: bool

def main(config: Config):
    return config.host
";
        let result = parse_python_signature(code, None, false)?;

        // Check that config parameter is detected as Object type
        assert_eq!(result.args.len(), 1);
        assert_eq!(result.args[0].name, "config");

        // Verify it's an Object type with correct class name
        match &result.args[0].typ {
            Typ::Object(obj) => {
                assert_eq!(obj.name, Some("Config".to_string()));
                assert!(obj.props.is_some());

                let props = obj.props.as_ref().unwrap();
                assert_eq!(props.len(), 3);

                // Verify field names and types
                assert_eq!(props[0].key, "host");
                assert_eq!(*props[0].typ, Typ::Str(None));

                assert_eq!(props[1].key, "port");
                assert_eq!(*props[1].typ, Typ::Int);

                assert_eq!(props[2].key, "debug");
                assert_eq!(*props[2].typ, Typ::Bool);
            }
            _ => panic!("Expected Typ::Object for dataclass"),
        }

        Ok(())
    }

    #[test]
    fn test_pydantic_nested_model() -> anyhow::Result<()> {
        let code = "
from pydantic import BaseModel

class Address(BaseModel):
    street: str
    city: str

class Person(BaseModel):
    name: str
    address: Address

def main(person: Person):
    return person.name
";
        let result = parse_python_signature(code, None, false)?;

        // Check that person parameter is detected as Object type
        assert_eq!(result.args.len(), 1);
        assert_eq!(result.args[0].name, "person");

        // Verify it's an Object type with nested model
        match &result.args[0].typ {
            Typ::Object(obj) => {
                assert_eq!(obj.name, Some("Person".to_string()));
                assert!(obj.props.is_some());

                let props = obj.props.as_ref().unwrap();
                assert_eq!(props.len(), 2);

                // Verify name field
                assert_eq!(props[0].key, "name");
                assert_eq!(*props[0].typ, Typ::Str(None));

                // Verify address field is a nested Object
                assert_eq!(props[1].key, "address");
                match props[1].typ.as_ref() {
                    Typ::Object(nested_obj) => {
                        assert_eq!(nested_obj.name, Some("Address".to_string()));
                        assert!(nested_obj.props.is_some());

                        let nested_props = nested_obj.props.as_ref().unwrap();
                        assert_eq!(nested_props.len(), 2);
                        assert_eq!(nested_props[0].key, "street");
                        assert_eq!(nested_props[1].key, "city");
                    }
                    _ => panic!("Expected nested Typ::Object for Address"),
                }
            }
            _ => panic!("Expected Typ::Object for Person model"),
        }

        Ok(())
    }
}
