//! Endpoint tools for MCP server
//!
//! Contains the auto-generated endpoint tools and utilities for converting
//! them to MCP tools and handling HTTP calls to Windmill API endpoints.

use crate::db::ApiAuthed;
use rmcp::{model::Tool, ErrorData};
use std::sync::Arc;
use windmill_common::db::Authed;
use windmill_common::{auth::create_jwt_token, BASE_INTERNAL_URL};

// Import the auto-generated tools
use super::auto_generated_endpoints;
pub use auto_generated_endpoints::{all_tools, EndpointTool};

/// Get all available endpoint tools
pub fn all_endpoint_tools() -> Vec<EndpointTool> {
    all_tools()
}

/// Convert endpoint tools to MCP tools
pub fn endpoint_tools_to_mcp_tools(endpoint_tools: Vec<EndpointTool>) -> Vec<Tool> {
    endpoint_tools
        .into_iter()
        .map(|tool| endpoint_tool_to_mcp_tool(&tool))
        .collect()
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

    let combined_schema = serde_json::json!({
        "type": "object",
        "properties": combined_properties,
        "required": combined_required
    });

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
    }
}

/// Create appropriate annotations for endpoint tools based on HTTP method
fn create_endpoint_annotations(tool: &EndpointTool) -> rmcp::model::ToolAnnotations {
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

    rmcp::model::ToolAnnotations {
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
            combined_required.push(req.to_string());
        }
    }
}

/// Call an endpoint tool by making HTTP request to Windmill API
pub async fn call_endpoint_tool(
    tool: &EndpointTool,
    args: serde_json::Value,
    workspace_id: &str,
    api_authed: &ApiAuthed,
) -> Result<serde_json::Value, ErrorData> {
    let args_map = match &args {
        serde_json::Value::Object(map) => map,
        _ => {
            return Err(ErrorData::invalid_params(
                "Arguments must be an object",
                Some(tool.name.clone().into()),
            ))
        }
    };

    // Build URL with path substitutions
    let path_template =
        substitute_path_params(&tool.path, workspace_id, args_map, &tool.path_params_schema)?;
    let query_string = build_query_string(args_map, &tool.query_params_schema);
    let full_url = format!(
        "{}/api{}{}",
        BASE_INTERNAL_URL.as_str(),
        path_template,
        query_string
    );

    // Prepare request body
    let body_json = build_request_body(&tool.method, args_map, &tool.body_schema);

    // Create and execute request
    let response =
        create_http_request(&tool.method, &full_url, workspace_id, api_authed, body_json).await?;

    let status = response.status();
    let response_text = response.text().await.map_err(|e| {
        ErrorData::internal_error(format!("Failed to read response text: {}", e), None)
    })?;

    if status.is_success() {
        Ok(serde_json::from_str(&response_text)
            .unwrap_or_else(|_| serde_json::Value::String(response_text)))
    } else {
        Err(ErrorData::internal_error(
            format!(
                "HTTP {} {}: {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or(""),
                response_text
            ),
            None,
        ))
    }
}

/// Substitute path parameters in the URL template
fn substitute_path_params(
    path: &str,
    workspace_id: &str,
    args_map: &serde_json::Map<String, serde_json::Value>,
    path_schema: &Option<serde_json::Value>,
) -> Result<String, ErrorData> {
    let mut path_template = path.replace("{workspace}", workspace_id);

    if let Some(schema) = path_schema {
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (param_name, _) in props {
                let placeholder = format!("{{{}}}", param_name);
                match args_map.get(param_name) {
                    Some(param_value) => {
                        if let Some(str_val) = param_value.as_str() {
                            path_template = path_template.replace(&placeholder, str_val);
                        }
                    }
                    None => {
                        tracing::warn!("Missing required path parameter: {}", param_name);
                        return Err(ErrorData::invalid_params(
                            format!("Missing required path parameter: {}", param_name),
                            None,
                        ));
                    }
                }
            }
        }
    }

    Ok(path_template)
}

/// Build query string from arguments
fn build_query_string(
    args_map: &serde_json::Map<String, serde_json::Value>,
    query_schema: &Option<serde_json::Value>,
) -> String {
    let Some(schema) = query_schema else {
        return String::new();
    };
    let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
        return String::new();
    };

    let query_params: Vec<String> = props
        .keys()
        .filter_map(|param_name| {
            args_map
                .get(param_name)
                .filter(|v| !v.is_null())
                .map(|value| {
                    let value_str = value.to_string();
                    let str_val = value_str.trim_matches('"');
                    format!(
                        "{}={}",
                        urlencoding::encode(param_name),
                        urlencoding::encode(str_val)
                    )
                })
        })
        .collect();

    if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    }
}

/// Build request body from arguments
fn build_request_body(
    method: &str,
    args_map: &serde_json::Map<String, serde_json::Value>,
    body_schema: &Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    if method == "GET" {
        return None;
    }

    let schema = body_schema.as_ref()?;
    let props = schema.get("properties")?.as_object()?;

    let body_map: serde_json::Map<String, serde_json::Value> = props
        .keys()
        .filter_map(|param_name| {
            args_map
                .get(param_name)
                .map(|value| (param_name.clone(), value.clone()))
        })
        .collect();

    if body_map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(body_map))
    }
}

/// Create HTTP request with authentication
async fn create_http_request(
    method: &str,
    url: &str,
    workspace_id: &str,
    api_authed: &ApiAuthed,
    body_json: Option<serde_json::Value>,
) -> Result<reqwest::Response, ErrorData> {
    let client = &crate::HTTP_CLIENT;
    let mut request_builder = match method {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        _ => {
            return Err(ErrorData::invalid_params(
                format!("Unsupported HTTP method: {}", method),
                None,
            ))
        }
    };

    // Add authorization header
    let authed = Authed::from(api_authed.clone());
    let token = create_jwt_token(authed, workspace_id, 3600, None, None, None, None)
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    request_builder = request_builder.header("Authorization", format!("Bearer {}", token));

    // Add body if present
    if let Some(body) = body_json {
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .json(&body);
    }

    request_builder
        .send()
        .await
        .map_err(|e| ErrorData::internal_error(format!("Failed to execute request: {}", e), None))
}
