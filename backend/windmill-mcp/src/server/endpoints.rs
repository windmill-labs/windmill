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
    pub query_field_renames: Option<serde_json::Value>,
    pub body_field_renames: Option<serde_json::Value>,
}

/// True if this endpoint is safe to expose to a read-only token. Mirrors the
/// `read_only_hint` computed by `create_endpoint_annotations`: only `GET`.
pub fn is_endpoint_read_only(tool: &EndpointTool) -> bool {
    tool.method.as_ref() == "GET"
}

/// The body fields of an endpoint that cannot be called with an empty body, or
/// `None` when a bodyless call is legitimate.
///
/// `minProperties` on the body schema marks an operation whose OpenAPI declares
/// `requestBody: required: true` (see `generate_mcp_tools.py`). The API answers a
/// request carrying no JSON body with 415 before the handler runs, so a call
/// filling none of these fields can only fail.
pub fn non_empty_body_fields(tool: &EndpointTool) -> Option<Vec<&str>> {
    let schema = tool.body_schema.as_ref()?;
    if schema.get("minProperties").and_then(|m| m.as_u64())? == 0 {
        return None;
    }
    let props = schema.get("properties")?.as_object()?;
    Some(props.keys().map(|k| k.as_str()).collect())
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

    let mut description = format!("{}. {}", tool.description, tool.instructions);

    // A body that must not be empty, none of whose fields is individually required,
    // is a requirement no `required` array can express. Spell it out, or the schema
    // reads as if the path alone were a complete call.
    if let Some(fields) = non_empty_body_fields(tool) {
        if !fields
            .iter()
            .any(|f| combined_required.iter().any(|r| r == f))
        {
            description = format!(
                "{} At least one of these arguments must be provided: {}.",
                description.trim_end(),
                fields.join(", ")
            );
        }
    }

    // Create annotations based on HTTP method and endpoint characteristics
    let annotations = create_endpoint_annotations(tool);

    Tool::new(
        tool.name.clone(),
        description,
        Arc::new(combined_schema.as_object().unwrap().clone()),
    )
    .with_title(tool.name.to_string())
    .with_annotations(annotations)
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
                "description": "Target workspace id (from list_workspaces)."
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

    // Surface the requirement in the prose description too (the schema is
    // authoritative, but some models/clients lean on the text). Kept terse — this
    // repeats across every workspace-scoped tool in the list.
    if let Some(desc) = mcp_tool.description.take() {
        mcp_tool.description = Some(format!("{desc} Requires `workspace_id`.").into());
    } else {
        mcp_tool.description = Some("Requires `workspace_id`.".into());
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

    Tool::new(
        Cow::Borrowed("list_workspaces"),
        "List the Windmill workspaces this token can access. Use the returned workspace ids as the `workspace_id` argument of the other tools.",
        Arc::new(schema.as_object().unwrap().clone()),
    )
    .with_title("List accessible workspaces")
    .with_annotations(
        ToolAnnotations::with_title("List accessible workspaces")
            .read_only(true)
            .destructive(false)
            .idempotent(true)
            .open_world(false),
    )
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

    ToolAnnotations::with_title(format!("{} {}", method, tool.path))
        .read_only(read_only)
        .destructive(destructive)
        .idempotent(idempotent)
        .open_world(open_world)
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
        assert!(
            mcp.description
                .as_deref()
                .unwrap_or_default()
                .contains("workspace_id"),
            "description must mention the workspace_id requirement"
        );
    }

    #[test]
    fn multi_leaves_global_tool_unchanged() {
        let global = tool("searchDocs", "/docs/search");
        let plain = endpoint_tool_to_mcp_tool(&global);
        let mcp = endpoint_tool_to_mcp_tool_multi(&global);
        assert_eq!(
            mcp.description, plain.description,
            "global tool description must be unchanged"
        );
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

    /// updateVariable-shaped: the body must not be empty, but no single field of it
    /// is required, so `required` cannot carry the constraint and the prose must.
    fn update_tool(body_required: &[&str]) -> EndpointTool {
        let mut t = tool("updateVariable", "/w/{workspace}/variables/update/{path}");
        t.method = Cow::Borrowed("POST");
        t.path_params_schema = Some(serde_json::json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        }));
        t.body_schema = Some(serde_json::json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" },
                "is_secret": { "type": "boolean" }
            },
            "required": body_required,
            "minProperties": 1
        }));
        t
    }

    #[test]
    fn required_body_with_no_required_field_is_stated_in_the_description() {
        let desc = endpoint_tool_to_mcp_tool(&update_tool(&[]))
            .description
            .unwrap_or_default()
            .to_string();
        assert!(
            desc.contains("value, is_secret"),
            "the description must name the body fields, got: {desc}"
        );
    }

    #[test]
    fn required_body_with_a_required_field_keeps_its_description() {
        // `required` already forbids the empty body here, so the sentence would be noise.
        let desc = endpoint_tool_to_mcp_tool(&update_tool(&["value"]))
            .description
            .unwrap_or_default()
            .to_string();
        assert!(
            !desc.contains("At least one"),
            "no extra sentence is needed when a body field is required, got: {desc}"
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
