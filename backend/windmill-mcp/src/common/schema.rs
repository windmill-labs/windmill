//! Schema conversion utilities for MCP server
//!
//! Contains functions for converting Windmill schemas into MCP-compatible formats.

use std::collections::HashSet;

use serde_json::Map;
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
    make_schema_compatible_inner(schema, true);
}

fn make_schema_compatible_inner(schema: &mut Value, is_root: bool) {
    let Value::Object(obj) = schema else { return };

    let is_object = obj
        .get("type")
        .map(|type_val| matches!(type_val, Value::String(s) if s == "object"))
        .unwrap_or(false);

    if is_object {
        let properties = obj
            .entry("properties")
            .or_insert_with(|| Value::Object(Map::new()));

        let has_named_properties = match properties {
            Value::Object(props) => {
                let has_named_properties = !props.is_empty();
                for value in props.values_mut() {
                    make_schema_compatible_inner(value, false);
                }
                has_named_properties
            }
            _ => {
                *properties = Value::Object(Map::new());
                false
            }
        };

        match obj.get_mut("required") {
            Some(Value::Array(_)) => {}
            Some(required) => *required = Value::Array(vec![]),
            None => {
                obj.insert("required".to_string(), Value::Array(vec![]));
            }
        }

        match obj.get_mut("additionalProperties") {
            Some(Value::Object(additional)) => {
                let mut additional_value = Value::Object(additional.clone());
                make_schema_compatible_inner(&mut additional_value, false);
                *additional = additional_value.as_object().cloned().unwrap_or_default();
            }
            Some(Value::Bool(_)) => {}
            Some(additional) => {
                *additional = Value::Bool(!is_root && !has_named_properties);
            }
            None => {
                // Root tool input schemas should be closed by default, while nested
                // opaque objects stay open unless a schema says otherwise.
                obj.insert(
                    "additionalProperties".to_string(),
                    Value::Bool(!is_root && !has_named_properties),
                );
            }
        }
    }

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
    if let Some(items) = obj.get_mut("items") {
        make_schema_compatible_inner(items, false);
    }

    if let Some(Value::Array(all_of)) = obj.get_mut("allOf") {
        for s in all_of.iter_mut() {
            make_schema_compatible_inner(s, false);
        }
    }

    if let Some(Value::Array(one_of)) = obj.get_mut("oneOf") {
        for s in one_of.iter_mut() {
            make_schema_compatible_inner(s, false);
        }
    }

    if let Some(Value::Array(any_of)) = obj.get_mut("anyOf") {
        for s in any_of.iter_mut() {
            make_schema_compatible_inner(s, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::make_schema_compatible;
    use serde_json::json;

    #[test]
    fn closes_root_object_schemas() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(schema["required"], json!(["path"]));
    }

    #[test]
    fn keeps_nested_opaque_objects_open() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "value": { "type": "object" }
            },
            "required": []
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["additionalProperties"], json!(false));
        assert_eq!(schema["properties"]["value"]["properties"], json!({}));
        assert_eq!(schema["properties"]["value"]["required"], json!([]));
        assert_eq!(
            schema["properties"]["value"]["additionalProperties"],
            json!(true)
        );
    }

    #[test]
    fn closes_root_no_arg_schemas() {
        let mut schema = json!({
            "type": "object"
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["properties"], json!({}));
        assert_eq!(schema["required"], json!([]));
        assert_eq!(schema["additionalProperties"], json!(false));
    }

    #[test]
    fn preserves_explicit_additional_properties() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "args": {
                    "type": "object",
                    "additionalProperties": true
                }
            },
            "required": []
        });

        make_schema_compatible(&mut schema);

        assert_eq!(
            schema["properties"]["args"]["additionalProperties"],
            json!(true)
        );
    }

    #[test]
    fn converts_nested_integer_types() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "count": { "type": "integer" }
            },
            "required": []
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["properties"]["count"]["type"], json!("number"));
    }
}
