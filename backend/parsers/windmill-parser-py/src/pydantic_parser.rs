/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Pydantic BaseModel and Python dataclass detection and parsing.
//!
//! This module provides functionality to detect and parse Pydantic models and Python
//! dataclasses from Python AST, enabling automatic UI generation for complex data structures.

use rustpython_parser::ast::{
    Constant, Expr, ExprAttribute, ExprCall, ExprConstant, ExprName, ExprTuple, Stmt, StmtAnnAssign,
};
use std::collections::HashSet;
use windmill_parser::{ObjectProperty, ObjectType, Typ};

// ==================================================================
// Constants
// ==================================================================

/// Maximum number of fields allowed in a Pydantic model or dataclass.
/// This prevents malicious code from defining models with thousands of fields.
const MAX_MODEL_FIELDS: usize = 200;

/// Maximum recursion depth for nested types.
const MAX_RECURSION_DEPTH: u8 = 10;

// ==================================================================
// Pydantic/Dataclass Detection
// ==================================================================

/// Detects if a class name refers to a Pydantic model or dataclass.
/// Returns ObjectType with parsed fields if detected, None otherwise.
///
/// # Arguments
/// * `class_name` - The name of the class to look up
/// * `module` - The AST statements to search in
pub fn detect_model_type(class_name: &str, module: &[Stmt]) -> Option<ObjectType> {
    let mut visited = HashSet::new();
    detect_model_type_impl(class_name, module, &mut visited)
}

/// Internal implementation with visited tracking
fn detect_model_type_impl(
    class_name: &str,
    module: &[Stmt],
    visited: &mut HashSet<String>,
) -> Option<ObjectType> {
    // Cycle detection: if we're already parsing this class, return a placeholder
    if visited.contains(class_name) {
        return Some(ObjectType {
            name: Some(class_name.to_string()),
            props: None, // Placeholder for self-referential types
        });
    }

    // Find class definition in module
    for stmt in module {
        if let Stmt::ClassDef(class_def) = stmt {
            if class_def.name.as_str() == class_name {
                // Mark as being visited
                visited.insert(class_name.to_string());

                let result = if is_pydantic_base(&class_def.bases) {
                    // Pydantic BaseModel
                    parse_model_fields(&class_def.body, class_def.name.as_str(), module, visited)
                } else if has_dataclass_decorator(&class_def.decorator_list)
                    || has_pydantic_dataclass_decorator(&class_def.decorator_list)
                {
                    // Standard dataclass or Pydantic dataclass
                    parse_model_fields(&class_def.body, class_def.name.as_str(), module, visited)
                } else {
                    // Found class but it's neither Pydantic nor dataclass
                    None
                };

                // Remove from visited set after processing
                visited.remove(class_name);

                return result;
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
            Expr::Name(ExprName { id, .. }) if id.as_str() == "BaseModel" => {
                return true;
            }

            // Match: class User(pydantic.BaseModel)
            Expr::Attribute(ExprAttribute { attr, value, .. }) if attr.as_str() == "BaseModel" => {
                if let Expr::Name(ExprName { id, .. }) = value.as_ref() {
                    if id.as_str() == "pydantic" {
                        return true;
                    }
                }
            }

            _ => {}
        }
    }

    false
}

/// Checks if a class has @dataclass decorator (standard library)
fn has_dataclass_decorator(decorators: &[Expr]) -> bool {
    for decorator in decorators {
        match decorator {
            // Match: @dataclass
            Expr::Name(ExprName { id, .. }) if id.as_str() == "dataclass" => {
                return true;
            }

            // Match: @dataclasses.dataclass
            Expr::Attribute(ExprAttribute { attr, value, .. }) if attr.as_str() == "dataclass" => {
                if let Expr::Name(ExprName { id, .. }) = value.as_ref() {
                    if id.as_str() == "dataclasses" {
                        return true;
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

/// Checks if a class has @pydantic.dataclasses.dataclass decorator (Pydantic v2)
fn has_pydantic_dataclass_decorator(decorators: &[Expr]) -> bool {
    for decorator in decorators {
        match decorator {
            // Match: @pydantic.dataclasses.dataclass
            Expr::Attribute(ExprAttribute { attr, value, .. }) if attr.as_str() == "dataclass" => {
                if let Expr::Attribute(ExprAttribute {
                    attr: inner_attr, value: inner_value, ..
                }) = value.as_ref()
                {
                    if inner_attr.as_str() == "dataclasses" {
                        if let Expr::Name(ExprName { id, .. }) = inner_value.as_ref() {
                            if id.as_str() == "pydantic" {
                                return true;
                            }
                        }
                    }
                }
            }

            // Match: @pydantic.dataclasses.dataclass(...)
            Expr::Call(ExprCall { func, .. }) => {
                if let Expr::Attribute(ExprAttribute { attr, value, .. }) = func.as_ref() {
                    if attr.as_str() == "dataclass" {
                        if let Expr::Attribute(ExprAttribute {
                            attr: inner_attr,
                            value: inner_value,
                            ..
                        }) = value.as_ref()
                        {
                            if inner_attr.as_str() == "dataclasses" {
                                if let Expr::Name(ExprName { id, .. }) = inner_value.as_ref() {
                                    if id.as_str() == "pydantic" {
                                        return true;
                                    }
                                }
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

// ==================================================================
// Field Parsing (Unified for Pydantic and Dataclass)
// ==================================================================

/// Parses model fields from class body (works for both Pydantic and dataclass)
fn parse_model_fields(
    body: &[Stmt],
    class_name: &str,
    module: &[Stmt],
    visited: &mut HashSet<String>,
) -> Option<ObjectType> {
    let mut properties = Vec::new();

    for stmt in body {
        // Extract annotated assignments: field_name: field_type
        if let Stmt::AnnAssign(ann_assign) = stmt {
            if let Some(prop) = parse_annotated_field(ann_assign, module, visited) {
                if properties.len() >= MAX_MODEL_FIELDS {
                    eprintln!(
                        "Model {model} exceeds maximum field count {limit}, truncating",
                        model = class_name,
                        limit = MAX_MODEL_FIELDS.to_string(),
                    );
                    break;
                }
                properties.push(prop);
            }
        }
    }

    if properties.is_empty() {
        // Empty model - return object with no properties
        return Some(ObjectType { name: Some(class_name.to_string()), props: None });
    }

    Some(ObjectType { name: Some(class_name.to_string()), props: Some(properties) })
}

/// Parses a single annotated field assignment
fn parse_annotated_field(
    ann_assign: &StmtAnnAssign,
    module: &[Stmt],
    visited: &mut HashSet<String>,
) -> Option<ObjectProperty> {
    if let Expr::Name(ExprName { id: field_name, .. }) = ann_assign.target.as_ref() {
        let field_type = extract_field_type(&ann_assign.annotation, 0, module, visited);
        Some(ObjectProperty { key: field_name.to_string(), typ: Box::new(field_type) })
    } else {
        None
    }
}

// ==================================================================
// Type Extraction
// ==================================================================

/// Extracts Windmill Typ from Python type annotation (RECURSIVE).
///
/// # Arguments
/// * `annotation` - The Python AST expression representing the type annotation
/// * `depth` - Current recursion depth (prevents infinite recursion)
/// * `module` - The AST statements for nested model lookup
/// * `visited` - Set of class names currently being parsed (for cycle detection)
fn extract_field_type(
    annotation: &Expr,
    depth: u8,
    module: &[Stmt],
    visited: &mut HashSet<String>,
) -> Typ {
    // Prevent infinite recursion
    if depth >= MAX_RECURSION_DEPTH {
        eprintln!(
            "Type annotation recursion limit {limit} reached, returning Unknown type",
            limit = MAX_RECURSION_DEPTH,
        );
        return Typ::Unknown;
    }

    match annotation {
        // Simple types: str, int, bool, float, bytes, Any
        Expr::Name(ExprName { id, .. }) => match id.as_str() {
            "str" => Typ::Str(None),
            "int" => Typ::Int,
            "float" => Typ::Float,
            "bool" => Typ::Bool,
            "bytes" => Typ::Bytes,
            "datetime" => Typ::Datetime,
            "date" => Typ::Date,
            "Any" => Typ::Unknown, // typing.Any maps to Unknown
            // Custom class - check if it's a model
            custom_type => {
                if let Some(object_type) = detect_model_type_impl(custom_type, module, visited) {
                    Typ::Object(object_type)
                } else {
                    // Unknown type - return Unknown instead of Resource
                    Typ::Unknown
                }
            }
        },

        // Generic types: List[T], Optional[T], Dict[K, V], Annotated[T, ...]
        Expr::Subscript(subscript) => {
            if let Expr::Name(ExprName { id, .. }) = subscript.value.as_ref() {
                match id.as_str() {
                    // List[T]
                    "List" | "list" => {
                        let inner_type =
                            extract_field_type(&subscript.slice, depth + 1, module, visited);
                        Typ::List(Box::new(inner_type))
                    }

                    // Optional[T] - unwrap to T
                    "Optional" => extract_field_type(&subscript.slice, depth + 1, module, visited),

                    // Dict[K, V] - return generic Object
                    "Dict" | "dict" => Typ::Object(ObjectType::new(None, Some(vec![]))),

                    // Annotated[T, ...] - extract the first type argument (Pydantic v2)
                    "Annotated" => match subscript.slice.as_ref() {
                        Expr::Tuple(ExprTuple { elts, .. }) if !elts.is_empty() => {
                            extract_field_type(&elts[0], depth + 1, module, visited)
                        }
                        _ => Typ::Unknown,
                    },

                    // Set[T], Tuple[T], etc. - not supported
                    _ => Typ::Unknown,
                }
            } else {
                Typ::Unknown
            }
        }

        // Union types: str | int (Python 3.10+) or Union[str, int]
        // Not fully supported - return Unknown with warning
        Expr::BinOp(_) => {
            eprintln!("Union types (e.g., str | int) are not yet supported, treating as Unknown");
            Typ::Unknown
        }

        // String annotations: "ForwardRef" (forward references)
        // Not fully supported - return Unknown with warning
        Expr::Constant(ExprConstant { value: Constant::Str(s), .. }) => {
            eprintln!(
                "Forward references like \"{forward_ref}\" are not yet supported, treating as Unknown",
                forward_ref = s,
            );
            Typ::Unknown
        }

        // All other annotations
        _ => Typ::Unknown,
    }
}
