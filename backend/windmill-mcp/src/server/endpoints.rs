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

/// True if this endpoint is safe to expose to a read-only token. Mirrors the
/// `read_only_hint` computed by `create_endpoint_annotations`: only `GET`.
pub fn is_endpoint_read_only(tool: &EndpointTool) -> bool {
    tool.method.as_ref() == "GET"
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

/// Convert an endpoint tool to an MCP tool for multi-workspace mode.
///
/// Endpoints whose path is workspace-scoped (`/w/{workspace}/...`) gain a
/// required `workspace_id` argument — in multi-workspace mode there is no
/// ambient workspace, so the caller must name the target workspace explicitly.
/// Global endpoints (e.g. docs search) are returned unchanged.
pub fn endpoint_tool_to_mcp_tool_multi(tool: &EndpointTool) -> Tool {
    let mut mcp_tool = endpoint_tool_to_mcp_tool(tool);

    if !tool.path.contains("{workspace}") {
        return mcp_tool;
    }

    let mut schema = (*mcp_tool.input_schema).clone();

    if let Some(props) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
        props.insert(
            "workspace_id".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The workspace to run this tool against. Call list_workspaces to see the workspaces you can access."
            }),
        );
    }

    match schema.get_mut("required").and_then(|r| r.as_array_mut()) {
        Some(req) => {
            if !req.iter().any(|v| v.as_str() == Some("workspace_id")) {
                req.insert(0, serde_json::Value::String("workspace_id".to_string()));
            }
        }
        None => {
            schema.insert("required".to_string(), serde_json::json!(["workspace_id"]));
        }
    }

    mcp_tool.input_schema = Arc::new(schema);
    mcp_tool
}

/// Build the synthetic `list_workspaces` tool exposed only in multi-workspace
/// mode. It takes no arguments and returns the workspaces the token can access.
pub fn list_workspaces_tool() -> Tool {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    });

    Tool {
        name: Cow::Borrowed("list_workspaces"),
        description: Some(
            "List the Windmill workspaces this token can access. Use the returned workspace ids as the `workspace_id` argument of the other tools."
                .into(),
        ),
        input_schema: Arc::new(schema.as_object().unwrap().clone()),
        title: Some("List accessible workspaces".to_string()),
        output_schema: None,
        icons: None,
        annotations: Some(ToolAnnotations {
            title: Some("List accessible workspaces".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        }),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &'static str, path: &'static str) -> EndpointTool {
        EndpointTool {
            name: Cow::Borrowed(name),
            description: Cow::Borrowed("desc"),
            instructions: Cow::Borrowed(""),
            path: Cow::Borrowed(path),
            method: Cow::Borrowed("GET"),
            path_params_schema: None,
            query_params_schema: Some(serde_json::json!({
                "type": "object",
                "properties": { "starred_only": { "type": "boolean" } },
                "required": []
            })),
            body_schema: None,
            path_field_renames: None,
            query_field_renames: None,
            body_field_renames: None,
        }
    }

    #[test]
    fn multi_injects_required_workspace_id_for_workspaced_tool() {
        let mcp =
            endpoint_tool_to_mcp_tool_multi(&tool("listScripts", "/w/{workspace}/scripts/list"));
        let props = mcp
            .input_schema
            .get("properties")
            .unwrap()
            .as_object()
            .unwrap();
        assert!(
            props.contains_key("workspace_id"),
            "workspace_id must be added as a property"
        );
        // pre-existing param is preserved
        assert!(props.contains_key("starred_only"));
        let required = mcp
            .input_schema
            .get("required")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(
            required.iter().any(|v| v.as_str() == Some("workspace_id")),
            "workspace_id must be required"
        );
    }

    #[test]
    fn multi_leaves_global_tool_unchanged() {
        let mcp = endpoint_tool_to_mcp_tool_multi(&tool("searchDocs", "/docs/search"));
        let props = mcp
            .input_schema
            .get("properties")
            .unwrap()
            .as_object()
            .unwrap();
        assert!(
            !props.contains_key("workspace_id"),
            "global tools (no {{workspace}} in path) must not gain a workspace_id arg"
        );
        let required = mcp
            .input_schema
            .get("required")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(required.iter().all(|v| v.as_str() != Some("workspace_id")));
    }

    #[test]
    fn multi_does_not_duplicate_workspace_id() {
        // Even if run twice, workspace_id stays a single required entry.
        let once = endpoint_tool_to_mcp_tool_multi(&tool("listFlows", "/w/{workspace}/flows/list"));
        let required = once
            .input_schema
            .get("required")
            .unwrap()
            .as_array()
            .unwrap();
        let count = required
            .iter()
            .filter(|v| v.as_str() == Some("workspace_id"))
            .count();
        assert_eq!(
            count, 1,
            "workspace_id must appear exactly once in required"
        );
    }

    #[test]
    fn list_workspaces_tool_has_no_params() {
        let t = list_workspaces_tool();
        assert_eq!(t.name.as_ref(), "list_workspaces");
        let required = t.input_schema.get("required").unwrap().as_array().unwrap();
        assert!(required.is_empty());
    }
}
