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
