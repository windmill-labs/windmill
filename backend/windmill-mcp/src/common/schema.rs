//! Schema conversion utilities for MCP server
//!
//! Contains functions for converting Windmill schemas into MCP-compatible formats.

use std::collections::HashSet;

use serde_json::Value;

use super::types::SchemaType;
use windmill_common::scripts::Schema;

/// Convert a Windmill Schema to a SchemaType
pub fn convert_schema_to_schema_type(schema: Option<Schema>) -> SchemaType {
    let schema_obj = if let Some(ref s) = schema {
        match serde_json::from_str::<SchemaType>(s.0.get()) {
            Ok(val) => val,
            Err(_) => SchemaType::default(),
        }
    } else {
        SchemaType::default()
    };
    schema_obj
}

/// Extract resource type keys from a schema
///
/// Scans the schema properties for fields with format "resource-{type}"
/// and returns a set of all unique resource type names found.
pub fn extract_resource_types_from_schema(schema: &SchemaType) -> HashSet<String> {
    let mut resource_types = HashSet::new();
    for (_key, prop_value) in schema.properties.iter() {
        if let Value::Object(prop_map) = prop_value {
            if let Some(Value::String(format_str)) = prop_map.get("format") {
                if let Some(rt) = format_str.strip_prefix("resource-") {
                    resource_types.insert(rt.to_string());
                }
            }
        }
    }
    resource_types
}

/// Transform a JSON schema for maximum MCP client compatibility.
///
/// Some MCP clients (e.g., n8n) have limited JSON Schema support:
/// - `integer` type is not supported (convert to `number`)
pub fn make_schema_compatible(schema: &mut Value) {
    let Value::Object(obj) = schema else { return };

    // 1. Convert integer to number
    if let Some(type_val) = obj.get_mut("type") {
        match type_val {
            Value::String(s) if s == "integer" => *s = "number".to_string(),
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    if let Value::String(s) = item {
                        if s == "integer" {
                            *s = "number".to_string();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Recursively process nested schemas
    if let Some(Value::Object(props)) = obj.get_mut("properties") {
        for value in props.values_mut() {
            make_schema_compatible(value);
        }
    }

    if let Some(items) = obj.get_mut("items") {
        make_schema_compatible(items);
    }

    if let Some(additional) = obj.get_mut("additionalProperties") {
        if additional.is_object() {
            make_schema_compatible(additional);
        }
    }

    if let Some(Value::Array(all_of)) = obj.get_mut("allOf") {
        for s in all_of.iter_mut() {
            make_schema_compatible(s);
        }
    }

    if let Some(Value::Array(one_of)) = obj.get_mut("oneOf") {
        for s in one_of.iter_mut() {
            make_schema_compatible(s);
        }
    }

    if let Some(Value::Array(any_of)) = obj.get_mut("anyOf") {
        for s in any_of.iter_mut() {
            make_schema_compatible(s);
        }
    }
}
