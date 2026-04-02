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
/// Ensures schemas conform to JSON Schema draft 2020-12 by:
/// - Converting `integer` type to `number` (some clients don't support integer)
/// - Removing invalid non-array `enum` values
/// - Stripping non-standard keywords (`originalType`, `format` with `resource-*` prefix)
/// - Fixing contradictory schemas (`type: "string"` with `properties` → `type: "object"`)
/// - Removing `default: null` when the type doesn't include `null`
/// - Adding `type: "object"` to empty schemas that have no type
pub fn make_schema_compatible(schema: &mut Value) {
    let Value::Object(obj) = schema else { return };

    // 1. Strip non-standard keywords that aren't part of JSON Schema
    obj.remove("originalType");

    // 2. Strip non-standard format values (resource-* is Windmill-internal)
    if obj
        .get("format")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s.starts_with("resource-"))
    {
        obj.remove("format");
    }

    // 3. Fix contradictory type: if `properties` is present, type must be "object"
    if obj.contains_key("properties") {
        match obj.get("type").and_then(|v| v.as_str()) {
            Some("object") => {}
            _ => {
                obj.insert("type".to_string(), Value::String("object".to_string()));
            }
        }
    }

    // 4. Convert integer to number
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

    // 5. Remove `default: null` when type doesn't include "null"
    if obj.get("default").is_some_and(|v| v.is_null()) {
        let type_includes_null = match obj.get("type") {
            Some(Value::String(s)) => s == "null",
            Some(Value::Array(arr)) => arr.iter().any(|v| v.as_str() == Some("null")),
            _ => false,
        };
        if !type_includes_null {
            obj.remove("default");
        }
    }

    // 6. Invalid enum values like `enum: null` are not valid draft 2020-12.
    if obj.get("enum").is_some_and(|enum_val| !enum_val.is_array()) {
        obj.remove("enum");
    }

    // 7. Ensure schemas with no type but with properties get type: "object"
    if !obj.contains_key("type") && !obj.is_empty() {
        obj.insert("type".to_string(), Value::String("object".to_string()));
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

#[cfg(test)]
mod tests {
    use super::make_schema_compatible;
    use serde_json::json;

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

    #[test]
    fn removes_invalid_nested_enum_values() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "body": {
                    "type": "object",
                    "properties": {
                        "expand": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": null
                            }
                        }
                    }
                }
            },
            "required": []
        });

        make_schema_compatible(&mut schema);

        assert_eq!(
            schema["properties"]["body"]["properties"]["expand"]["items"],
            json!({ "type": "string" })
        );
    }

    #[test]
    fn preserves_valid_enum_arrays() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["open", "closed"]
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(
            schema["properties"]["status"]["enum"],
            json!(["open", "closed"])
        );
    }

    #[test]
    fn strips_original_type() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "originalType": "string"
                },
                "data": {
                    "type": "string",
                    "originalType": "bytes"
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert!(schema["properties"]["name"].get("originalType").is_none());
        assert!(schema["properties"]["data"].get("originalType").is_none());
        assert_eq!(schema["properties"]["name"]["type"], json!("string"));
    }

    #[test]
    fn strips_resource_format() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "db": {
                    "type": "string",
                    "format": "resource-postgresql"
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert!(schema["properties"]["db"].get("format").is_none());
        assert_eq!(schema["properties"]["db"]["type"], json!("string"));
    }

    #[test]
    fn preserves_standard_format() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "email": {
                    "type": "string",
                    "format": "email"
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["properties"]["email"]["format"], json!("email"));
    }

    #[test]
    fn fixes_string_type_with_properties() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "string",
                    "format": "resource-record",
                    "properties": {}
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["properties"]["config"]["type"], json!("object"));
        assert!(schema["properties"]["config"].get("format").is_none());
    }

    #[test]
    fn removes_null_default_on_string_type() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "default": null
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert!(schema["properties"]["name"].get("default").is_none());
    }

    #[test]
    fn preserves_null_default_when_type_includes_null() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": ["string", "null"],
                    "default": null
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["properties"]["name"]["default"], json!(null));
    }

    #[test]
    fn preserves_non_null_default() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "default": "hello"
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["properties"]["name"]["default"], json!("hello"));
    }

    #[test]
    fn adds_type_object_to_empty_schema() {
        let mut schema = json!({});

        make_schema_compatible(&mut schema);

        // Empty schema stays empty (no keys = truly empty)
        assert_eq!(schema, json!({}));
    }

    #[test]
    fn adds_type_to_schema_with_properties_but_no_type() {
        let mut schema = json!({
            "properties": {
                "value": {}
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["type"], json!("object"));
    }
}
