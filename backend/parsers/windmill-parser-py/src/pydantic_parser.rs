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
    Constant, Expr, ExprAttribute, ExprCall, ExprConstant, ExprName, Stmt, StmtClassDef,
};
use std::cell::RefCell;
use windmill_parser::{ObjectType, Typ};

// ==================================================================
// Thread-Local Storage for AST Module
// ==================================================================

// Thread-local storage for AST module to enable Pydantic/dataclass detection.
// Uses a STACK to support recursive calls without race conditions.
thread_local! {
    static MODULE_STACK: RefCell<Vec<Vec<Stmt>>> = RefCell::new(Vec::new());
}

/// Push a module onto the stack
pub fn set_current_module(module: Vec<Stmt>) {
    MODULE_STACK.with(|stack| {
        stack.borrow_mut().push(module);
    });
}

/// Get a clone of the current (top of stack) module
pub fn get_current_module() -> Option<Vec<Stmt>> {
    MODULE_STACK.with(|stack| stack.borrow().last().cloned())
}

/// Pop the current module from the stack
pub fn clear_current_module() {
    MODULE_STACK.with(|stack| {
        stack.borrow_mut().pop();
    });
}

// ==================================================================
// RAII Guard for Thread-Local Cleanup
// ==================================================================

/// RAII guard to ensure thread-local module is always cleaned up
pub struct ModuleGuard;

impl ModuleGuard {
    pub fn new(module: Vec<Stmt>) -> Self {
        set_current_module(module);
        Self
    }
}

impl Drop for ModuleGuard {
    fn drop(&mut self) {
        clear_current_module();
    }
}

// ==================================================================
// Pydantic/Dataclass Detection
// ==================================================================

/// Maximum number of fields allowed in a Pydantic model or dataclass.
/// This prevents malicious code from defining models with thousands of fields.
const MAX_MODEL_FIELDS: usize = 200;

/// Detects if a class name refers to a Pydantic model or dataclass.
/// Returns ObjectType with parsed fields if detected, None otherwise.
pub fn detect_model_type(class_name: &str, module: &[Stmt]) -> Option<ObjectType> {
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

// ==================================================================
// Type Extraction
// ==================================================================

/// Extracts Windmill Typ from Python type annotation (RECURSIVE).
///
/// # Arguments
/// * `annotation` - The Python AST expression representing the type annotation
/// * `depth` - Current recursion depth (prevents infinite recursion)
fn extract_field_type(annotation: &Expr, depth: u8) -> Typ {
    const MAX_RECURSION_DEPTH: u8 = 10;

    // Prevent infinite recursion (max depth: 10)
    if depth >= MAX_RECURSION_DEPTH {
        eprintln!(
            "Warning: Type annotation recursion limit ({}) reached. Returning Unknown type.",
            MAX_RECURSION_DEPTH
        );
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
        Expr::BinOp(_) => {
            eprintln!(
                "Warning: Union types (e.g., str | int) are not yet supported. \
                 Field will be treated as Unknown type."
            );
            Typ::Unknown
        }

        // String annotations: "ForwardRef" (forward references)
        // MVP: Not supported, return Unknown
        Expr::Constant(ExprConstant {
            value: Constant::Str(s),
            ..
        }) => {
            eprintln!(
                "Warning: Forward references (e.g., '{}') are not yet supported. \
                 Field will be treated as Unknown type.",
                s
            );
            Typ::Unknown
        }

        // All other annotations
        _ => Typ::Unknown,
    }
}
