//! Endpoint tools for MCP server
//!
//! Contains the EndpointTool structure and utilities for converting
//! them to MCP tools.

use rmcp::model::{Tool, ToolAnnotations};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

use crate::common::schema::make_schema_compatible;

/// Represents an auto-generated endpoint tool from OpenAPI specification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EndpointTool {
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub instructions: Cow<'static, str>,
    pub path: Cow<'static, str>,
    pub method: Cow<'static, str>,
    pub path_params_schema: Option<serde_json::Value>,
    pub query_params_schema: Option<serde_json::Value>,
    pub body_schema: Option<serde_json::Value>,
    pub path_field_renames: Option<serde_json::Value>,
    pub query_field_renames: Option<serde_json::Value>,
    pub body_field_renames: Option<serde_json::Value>,
}

/// Convert a single endpoint tool to MCP tool
pub fn endpoint_tool_to_mcp_tool(tool: &EndpointTool) -> Tool {
    let mut combined_properties = serde_json::Map::new();
    let mut combined_required = Vec::new();

    // Combine all parameter schemas
    let schemas = [
        &tool.path_params_schema,
        &tool.query_params_schema,
        &tool.body_schema,
    ];

    for schema in schemas.iter().filter_map(|s| s.as_ref()) {
        merge_schema_into(&mut combined_properties, &mut combined_required, schema);
    }

    let mut combined_schema = serde_json::json!({
        "type": "object",
        "properties": combined_properties,
        "required": combined_required
    });
    make_schema_compatible(&mut combined_schema);

    let description = format!("{}. {}", tool.description, tool.instructions);

    // Create annotations based on HTTP method and endpoint characteristics
    let annotations = create_endpoint_annotations(tool);

    Tool {
        name: tool.name.clone(),
        description: Some(description.into()),
        input_schema: Arc::new(combined_schema.as_object().unwrap().clone()),
        title: Some(tool.name.to_string()),
        output_schema: None,
        icons: None,
        annotations: Some(annotations),
        meta: None,
        execution: None,
    }
}

/// Create appropriate annotations for endpoint tools based on HTTP method
fn create_endpoint_annotations(tool: &EndpointTool) -> ToolAnnotations {
    let method = tool.method.as_ref();

    // Determine characteristics based on HTTP method
    let (read_only, destructive, idempotent, open_world) = match method {
        "GET" => (true, false, true, true), // Read-only, safe, idempotent
        "POST" => (false, true, false, true), // Can modify, potentially destructive, not idempotent
        "PUT" => (false, false, true, true), // Can modify, typically idempotent updates
        "DELETE" => (false, true, true, true), // Destructive but idempotent
        "PATCH" => (false, false, false, true), // Partial updates, not guaranteed idempotent
        _ => (false, true, false, true),    // Default: assume can modify and be destructive
    };

    ToolAnnotations {
        title: Some(format!("{} {}", method, tool.path)),
        read_only_hint: Some(read_only),
        destructive_hint: Some(destructive),
        idempotent_hint: Some(idempotent),
        open_world_hint: Some(open_world),
    }
}

/// Merge schema into combined properties and required fields
fn merge_schema_into(
    combined_properties: &mut serde_json::Map<String, serde_json::Value>,
    combined_required: &mut Vec<String>,
    schema: &serde_json::Value,
) {
    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        for (key, value) in props {
            combined_properties.insert(key.clone(), value.clone());
        }
    }

    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        for req in required.iter().filter_map(|r| r.as_str()) {
            if !combined_required.contains(&req.to_string()) {
                combined_required.push(req.to_string());
            }
        }
    }
}
