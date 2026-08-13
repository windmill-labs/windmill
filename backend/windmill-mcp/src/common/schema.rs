//! Schema conversion utilities for MCP server
//!
//! Contains functions for converting Windmill schemas into MCP-compatible formats.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use super::types::{ResourceInfo, ResourceType, SchemaType};
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

/// If `node` is a schema describing a Windmill resource reference, return the
/// resource type name. Recognizes both on-disk shapes:
///   - Form A (top-level scalar): `{ type: "object", format: "resource-<name>" }`
///   - Form B (inside `items`):    `{ type: "resource", resourceType: "<name>" }`
fn resource_type_of_schema_node(node: &Value) -> Option<String> {
    let obj = node.as_object()?;
    if let Some(Value::String(fmt)) = obj.get("format") {
        if let Some(name) = fmt.strip_prefix("resource-") {
            return Some(name.to_string());
        }
    }
    if obj.get("type").and_then(Value::as_str) == Some("resource") {
        if let Some(Value::String(name)) = obj.get("resourceType") {
            return Some(name.clone());
        }
    }
    None
}

/// Recursively collect resource type names referenced at any depth in `node`.
fn collect_resource_types(node: &Value, out: &mut HashSet<String>) {
    if let Some(rt) = resource_type_of_schema_node(node) {
        out.insert(rt);
    }
    let Some(obj) = node.as_object() else { return };
    if let Some(Value::Object(props)) = obj.get("properties") {
        for v in props.values() {
            collect_resource_types(v, out);
        }
    }
    if let Some(items) = obj.get("items") {
        collect_resource_types(items, out);
    }
    if let Some(additional) = obj.get("additionalProperties") {
        if additional.is_object() {
            collect_resource_types(additional, out);
        }
    }
    for kw in ["allOf", "oneOf", "anyOf"] {
        if let Some(Value::Array(arr)) = obj.get(kw) {
            for s in arr {
                collect_resource_types(s, out);
            }
        }
    }
}

/// Extract resource type keys referenced anywhere in a schema (top-level
/// properties, nested objects, array items, additionalProperties, and
/// allOf/oneOf/anyOf subschemas). Recognizes both the `format: resource-<name>`
/// and `type: resource` + `resourceType` shapes.
pub fn extract_resource_types_from_schema(schema: &SchemaType) -> HashSet<String> {
    let mut resource_types = HashSet::new();
    for prop_value in schema.properties.values() {
        collect_resource_types(prop_value, &mut resource_types);
    }
    resource_types
}

/// Rewrite a single schema node that points to a Windmill resource: set
/// `type: "string"` and inject a description listing the available resources.
/// Mirrors the top-level behavior that used to live in
/// `transform_schema_for_resources`. No-op if the resource type isn't in the
/// pre-fetched cache.
fn apply_resource_enrichment(
    prop_map: &mut Map<String, Value>,
    resource_type_key: &str,
    resources_cache: &HashMap<String, Vec<ResourceInfo>>,
    resources_types: &[ResourceType],
) {
    let Some(resource_cache) = resources_cache.get(resource_type_key) else {
        return;
    };
    let resource_type = resources_types
        .iter()
        .find(|rt| rt.name == resource_type_key);
    let resources_count = resource_cache.len();
    let description = match resource_type {
        Some(rt) => format!(
            "This is a resource named `{}` with the following description: `{}`.\\nThe path of the resource should be used to specify the resource.\\n{}",
            rt.name,
            rt.description.as_deref().unwrap_or("No description"),
            if resources_count == 0 {
                "This resource does not have any available instances, you should create one from your windmill workspace."
            } else if resources_count > 1 {
                "This resource has multiple available instances, you should precisely select the one you want to use."
            } else {
                "There is 1 resource available."
            }
        ),
        None => "An object parameter.".to_string(),
    };
    prop_map.insert("type".to_string(), Value::String("string".to_string()));
    prop_map.insert("description".to_string(), Value::String(description));
    // Drop the Windmill-internal keys we just consumed so the node is clean
    // regardless of whether `make_schema_compatible` runs after us. (Its strip
    // only fires while `type == "resource"`, which is no longer true here.)
    prop_map.remove("resourceType");
    if prop_map
        .get("format")
        .and_then(Value::as_str)
        .is_some_and(|s| s.starts_with("resource-"))
    {
        prop_map.remove("format");
    }
    if resources_count > 0 {
        let resources_description = resource_cache
            .iter()
            .map(|resource| {
                format!(
                    "{}: $res:{}",
                    resource.description.as_deref().unwrap_or("No title"),
                    resource.path
                )
            })
            .collect::<Vec<String>>()
            .join("\\n");
        let prior_description = prop_map
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("No description")
            .to_string();
        prop_map.insert(
            "description".to_string(),
            Value::String(format!(
                "{}\\nHere are the available resources, in the format title:path. Title can be empty. Path should be used to specify the resource:\\n{}",
                prior_description, resources_description
            )),
        );
    }
}

/// Walk a schema and enrich every Windmill-resource reference (in either shape,
/// at any nesting depth) with `type: "string"` and a description listing
/// available resources. The non-standard keys (`format: resource-*`,
/// `resourceType`) consumed by the enrichment are stripped in place.
pub fn enrich_resource_schemas(
    node: &mut Value,
    resources_cache: &HashMap<String, Vec<ResourceInfo>>,
    resources_types: &[ResourceType],
) {
    if let Some(rt_key) = resource_type_of_schema_node(node) {
        if let Value::Object(obj) = node {
            apply_resource_enrichment(obj, &rt_key, resources_cache, resources_types);
        }
    }
    let Some(obj) = node.as_object_mut() else {
        return;
    };
    if let Some(Value::Object(props)) = obj.get_mut("properties") {
        for v in props.values_mut() {
            enrich_resource_schemas(v, resources_cache, resources_types);
        }
    }
    if let Some(items) = obj.get_mut("items") {
        enrich_resource_schemas(items, resources_cache, resources_types);
    }
    if let Some(additional) = obj.get_mut("additionalProperties") {
        if additional.is_object() {
            enrich_resource_schemas(additional, resources_cache, resources_types);
        }
    }
    for kw in ["allOf", "oneOf", "anyOf"] {
        if let Some(Value::Array(arr)) = obj.get_mut(kw) {
            for s in arr.iter_mut() {
                enrich_resource_schemas(s, resources_cache, resources_types);
            }
        }
    }
}

/// The seven type names permitted by the JSON Schema draft 2020-12 `type` keyword.
fn is_valid_json_schema_type(t: &str) -> bool {
    matches!(
        t,
        "null" | "boolean" | "object" | "array" | "number" | "string" | "integer"
    )
}

/// Transform a JSON schema for maximum MCP client compatibility.
///
/// Ensures schemas conform to JSON Schema draft 2020-12 by:
/// - Converting `integer` type to `number` (some clients don't support integer)
/// - Removing invalid non-array `enum` values
/// - Stripping non-standard keywords (`originalType`, `format` with `resource-*` prefix)
/// - Rewriting the Windmill pseudo-type `type: "resource"` to `type: "string"`
/// - Fixing contradictory schemas (`type: "string"` with `properties` → `type: "object"`,
///   or with `items` → `type: "array"`)
/// - Dropping invalid `type` values (e.g. the empty string `""` Windmill emits for
///   untyped fields) so the node validates as "any type"
/// - Removing `default: null` when the type doesn't include `null`
/// - Adding `type: "object"` to property-bearing schemas that have no type
pub fn make_schema_compatible(schema: &mut Value) {
    let Value::Object(obj) = schema else { return };

    // 1. Strip non-standard keywords that aren't part of JSON Schema. The
    // Windmill-internal `resourceType` is dropped unconditionally so it can't
    // leak through even on enrichment cache-miss paths.
    obj.remove("originalType");
    obj.remove("resourceType");

    // 2. Strip non-standard format values (resource-* is Windmill-internal)
    if obj
        .get("format")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s.starts_with("resource-"))
    {
        obj.remove("format");
    }

    // 2b. Rewrite Windmill pseudo-type `resource` to `string`. The parser emits
    // this shape (with a sibling `resourceType` key) for `list[ResourceType]`
    // params; "resource" is not in the JSON Schema 2020-12 type enum and is
    // rejected by strict validators (e.g. Anthropic's tool registration).
    if obj.get("type").and_then(|v| v.as_str()) == Some("resource") {
        obj.insert("type".to_string(), Value::String("string".to_string()));
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

    // 3b. Fix contradictory type: if `items` is present (and the node isn't an
    // object), type must be "array". Windmill serializes a `list[...]` field as
    // `type: "string"` with an `items` subschema; "string" + items is nonsense
    // and leaves the element schema unreachable to the client.
    if obj.contains_key("items") && !obj.contains_key("properties") {
        match obj.get("type").and_then(|v| v.as_str()) {
            Some("array") => {}
            _ => {
                obj.insert("type".to_string(), Value::String("array".to_string()));
            }
        }
    }

    // 3c. Drop invalid `type` values. Windmill emits `type: ""` for fields
    // declared without an explicit type; the empty string (and any other name
    // outside the draft 2020-12 type enum) makes a strict validator reject the
    // whole schema (e.g. Anthropic's tool registration). Removing it leaves the
    // node untyped, which accepts any value -- the meaning of an untyped field.
    match obj.get("type") {
        None => {}
        Some(Value::String(s)) => {
            if !is_valid_json_schema_type(s) {
                obj.remove("type");
            }
        }
        Some(Value::Array(arr)) => {
            let filtered: Vec<Value> = arr
                .iter()
                .filter(|v| v.as_str().is_some_and(is_valid_json_schema_type))
                .cloned()
                .collect();
            match filtered.len() {
                0 => {
                    obj.remove("type");
                }
                1 => {
                    obj.insert("type".to_string(), filtered.into_iter().next().unwrap());
                }
                _ => {
                    obj.insert("type".to_string(), Value::Array(filtered));
                }
            }
        }
        // `type` as null/number/bool/object is not a valid keyword value at all.
        Some(_) => {
            obj.remove("type");
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
    use super::*;
    use crate::common::types::{ResourceInfo, ResourceType};
    use serde_json::json;
    use std::collections::HashMap;

    fn raw_schema(raw: &str) -> Option<Schema> {
        Some(serde_json::from_str::<Schema>(raw).unwrap())
    }

    #[test]
    fn flow_schema_without_required_keeps_properties() {
        // Real flow input schema shape (see backend/tests/worker.rs): it omits
        // `required` and carries an `order` key instead. This shape must keep its
        // properties, not fall back to an empty schema, so MCP flow tools still
        // advertise their inputs.
        let flow = raw_schema(
            r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"world":{"type":"string"}},"type":"object","order":["world"]}"#,
        );
        let schema_type = convert_schema_to_schema_type(flow);
        assert_eq!(schema_type.r#type, "object");
        assert!(schema_type.properties.contains_key("world"));
        assert!(schema_type.required.is_empty());
    }

    #[test]
    fn script_schema_with_required_keeps_properties() {
        // Script schemas always include `required`; this must remain unaffected.
        let script = raw_schema(
            r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"world":{"type":"string"}},"required":["world"],"type":"object"}"#,
        );
        let schema_type = convert_schema_to_schema_type(script);
        assert!(schema_type.properties.contains_key("world"));
        assert_eq!(schema_type.required, vec!["world".to_string()]);
    }

    fn aws_resources() -> (HashMap<String, Vec<ResourceInfo>>, Vec<ResourceType>) {
        let mut cache = HashMap::new();
        cache.insert(
            "c_aws_account".to_string(),
            vec![ResourceInfo {
                path: "f/platform/aws_dev".to_string(),
                description: Some("Dev account".to_string()),
                resource_type: "c_aws_account".to_string(),
            }],
        );
        let types = vec![ResourceType {
            name: "c_aws_account".to_string(),
            description: Some("AWS account".to_string()),
        }];
        (cache, types)
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
    fn rewrites_resource_pseudo_type_to_string() {
        let mut schema = json!({
            "type": "resource",
            "resourceType": "c_aws_account"
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["type"], json!("string"));
        assert!(schema.get("resourceType").is_none());
    }

    #[test]
    fn rewrites_resource_pseudo_type_inside_array_items() {
        // Phocas repro: list[ResourceType] parameter. Anthropic rejected this
        // with "tools.<N>.custom.input_schema: JSON schema is invalid" because
        // "resource" is not in the draft 2020-12 type enum.
        let mut schema = json!({
            "type": "object",
            "properties": {
                "accounts": {
                    "type": "array",
                    "items": {
                        "type": "resource",
                        "resourceType": "c_aws_account"
                    }
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(
            schema["properties"]["accounts"]["items"]["type"],
            json!("string")
        );
        assert!(schema["properties"]["accounts"]["items"]
            .get("resourceType")
            .is_none());
    }

    #[test]
    fn rewrites_resource_pseudo_type_inside_nested_properties() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "db": {
                            "type": "resource",
                            "resourceType": "postgresql"
                        }
                    }
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(
            schema["properties"]["config"]["properties"]["db"]["type"],
            json!("string")
        );
        assert!(schema["properties"]["config"]["properties"]["db"]
            .get("resourceType")
            .is_none());
    }

    #[test]
    fn strips_nested_resource_format_inside_array_items() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "dbs": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "format": "resource-postgresql"
                    }
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert!(schema["properties"]["dbs"]["items"].get("format").is_none());
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

    #[test]
    fn extract_resource_types_top_level_form_a() {
        let schema: SchemaType = serde_json::from_value(json!({
            "type": "object",
            "properties": {
                "db": { "type": "object", "format": "resource-postgresql" }
            },
            "required": []
        }))
        .unwrap();

        let types = extract_resource_types_from_schema(&schema);
        assert!(types.contains("postgresql"));
        assert_eq!(types.len(), 1);
    }

    #[test]
    fn extract_resource_types_inside_array_items_form_b() {
        let schema: SchemaType = serde_json::from_value(json!({
            "type": "object",
            "properties": {
                "accounts": {
                    "type": "array",
                    "items": { "type": "resource", "resourceType": "c_aws_account" }
                }
            },
            "required": []
        }))
        .unwrap();

        let types = extract_resource_types_from_schema(&schema);
        assert!(types.contains("c_aws_account"));
    }

    #[test]
    fn extract_resource_types_inside_nested_properties_and_one_of() {
        let schema: SchemaType = serde_json::from_value(json!({
            "type": "object",
            "properties": {
                "config": {
                    "type": "object",
                    "properties": {
                        "db": { "type": "resource", "resourceType": "postgresql" }
                    }
                },
                "either": {
                    "oneOf": [
                        { "type": "object", "format": "resource-mysql" },
                        { "type": "string" }
                    ]
                }
            },
            "required": []
        }))
        .unwrap();

        let types = extract_resource_types_from_schema(&schema);
        assert!(types.contains("postgresql"));
        assert!(types.contains("mysql"));
    }

    #[test]
    fn enrich_top_level_form_a_resource() {
        let (cache, types) = aws_resources();
        let mut node = json!({
            "type": "object",
            "format": "resource-c_aws_account"
        });

        enrich_resource_schemas(&mut node, &cache, &types);

        assert_eq!(node["type"], json!("string"));
        assert!(node.get("format").is_none());
        let desc = node["description"].as_str().unwrap();
        assert!(desc.contains("c_aws_account"));
        assert!(desc.contains("$res:f/platform/aws_dev"));
    }

    #[test]
    fn enrich_inside_array_items_form_b() {
        // Phocas repro: items use the parser-style `type: resource` form.
        let (cache, types) = aws_resources();
        let mut node = json!({
            "type": "array",
            "items": { "type": "resource", "resourceType": "c_aws_account" }
        });

        enrich_resource_schemas(&mut node, &cache, &types);

        // The items schema should be rewritten to string with a description
        // listing the available resources, and the Windmill-internal
        // resourceType key should be stripped.
        assert_eq!(node["items"]["type"], json!("string"));
        assert!(node["items"].get("resourceType").is_none());
        let desc = node["items"]["description"].as_str().unwrap();
        assert!(desc.contains("$res:f/platform/aws_dev"));
    }

    #[test]
    fn enrich_is_noop_when_resource_type_not_in_cache() {
        let mut node = json!({
            "type": "object",
            "format": "resource-unknown_type"
        });
        let before = node.clone();

        enrich_resource_schemas(&mut node, &HashMap::new(), &[]);

        assert_eq!(node, before);
    }

    #[test]
    fn enrich_deeply_nested_resource() {
        let (cache, types) = aws_resources();
        let mut node = json!({
            "type": "object",
            "properties": {
                "outer": {
                    "type": "object",
                    "properties": {
                        "inner": {
                            "type": "array",
                            "items": {
                                "type": "resource",
                                "resourceType": "c_aws_account"
                            }
                        }
                    }
                }
            }
        });

        enrich_resource_schemas(&mut node, &cache, &types);

        assert_eq!(
            node["properties"]["outer"]["properties"]["inner"]["items"]["type"],
            json!("string")
        );
    }

    /// Recursively assert no `type` keyword anywhere in the schema carries an
    /// empty string or a name outside the draft 2020-12 type enum -- the exact
    /// shape Anthropic rejects with "input_schema: JSON Schema is invalid".
    fn assert_all_types_valid(node: &Value) {
        if let Some(obj) = node.as_object() {
            match obj.get("type") {
                Some(Value::String(s)) => {
                    assert!(is_valid_json_schema_type(s), "invalid type string: {s:?}")
                }
                Some(Value::Array(arr)) => {
                    for t in arr {
                        let s = t.as_str().expect("type array entries must be strings");
                        assert!(is_valid_json_schema_type(s), "invalid type entry: {s:?}");
                    }
                }
                _ => {}
            }
            for v in obj.values() {
                assert_all_types_valid(v);
            }
        } else if let Some(arr) = node.as_array() {
            for v in arr {
                assert_all_types_valid(v);
            }
        }
    }

    #[test]
    fn drops_empty_type_string() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "value": { "type": "", "description": "" }
            }
        });

        make_schema_compatible(&mut schema);

        // Empty type is removed entirely (untyped == accepts any value); it is
        // NOT re-typed to object, so a scalar value still validates.
        assert!(schema["properties"]["value"].get("type").is_none());
        assert_all_types_valid(&schema);
    }

    #[test]
    fn infers_array_type_from_items() {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "services": {
                    "type": "string",
                    "description": "An object parameter.",
                    "items": { "type": "object" }
                }
            }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["properties"]["services"]["type"], json!("array"));
        assert_all_types_valid(&schema);
    }

    #[test]
    fn items_does_not_override_object_with_properties() {
        // A node carrying both `properties` and a stray `items` is an object,
        // not an array -- the object signal wins.
        let mut schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "items": { "type": "string" }
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["type"], json!("object"));
    }

    #[test]
    fn filters_invalid_type_array_entries() {
        let mut schema = json!({ "type": ["string", ""] });

        make_schema_compatible(&mut schema);

        // Sole surviving entry collapses to a bare string.
        assert_eq!(schema["type"], json!("string"));
    }

    #[test]
    fn drops_type_array_when_all_entries_invalid() {
        let mut schema = json!({ "type": ["", "bogus"], "description": "x" });

        make_schema_compatible(&mut schema);

        assert!(schema.get("type").is_none());
    }

    #[test]
    fn customer_services_field_repro() {
        // Repro of the reported failure: a `list[object]` script param that
        // Windmill serialized as `type: "string"` + `items`, whose element
        // object carried an untyped `value` field (`type: ""`). Anthropic
        // rejected the whole tool list with
        // "tools.<N>.custom.input_schema: JSON Schema is invalid".
        let mut schema = json!({
            "type": "object",
            "properties": {
                "services": {
                    "items": {
                        "type": "object",
                        "properties": {
                            "serviceTypeId": { "description": "", "type": "string" },
                            "value": { "description": "", "type": "" }
                        }
                    },
                    "description": "An object parameter.",
                    "type": "string"
                }
            },
            "required": ["services"]
        });

        make_schema_compatible(&mut schema);

        assert_eq!(schema["properties"]["services"]["type"], json!("array"));
        assert!(
            schema["properties"]["services"]["items"]["properties"]["value"]
                .get("type")
                .is_none()
        );
        assert_all_types_valid(&schema);
    }
}
