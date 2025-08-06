use rmcp::{model::Tool, Error};
use std::sync::Arc;
use windmill_common::auth::create_jwt_token;
use windmill_common::db::Authed;
use windmill_common::BASE_URL;
use crate::db::ApiAuthed;
use crate::mcp_tools::EndpointTool;

pub fn endpoint_tools_to_mcp_tools(endpoint_tools: Vec<EndpointTool>) -> Vec<Tool> {
    endpoint_tools.into_iter().map(|tool| endpoint_tool_to_mcp_tool(&tool)).collect()
}

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
    
    Tool {
        name: tool.name.clone(),
        description: Some(description.into()),
        input_schema: Arc::new(combined_schema.as_object().unwrap().clone()),
        annotations: Some(rmcp::model::ToolAnnotations {
            title: Some(format!("{} {}", 
                match tool.method {
                    http::Method::GET => "GET",
                    http::Method::POST => "POST", 
                    http::Method::PUT => "PUT",
                    http::Method::DELETE => "DELETE",
                    http::Method::PATCH => "PATCH",
                    _ => "UNKNOWN"
                }, 
                tool.path
            )),
            read_only_hint: None,
            destructive_hint: None,
            idempotent_hint: None,
            open_world_hint: None,
        }),
    }
}

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

pub async fn call_endpoint_tool(
    tool: &EndpointTool,
    args: serde_json::Value,
    workspace_id: &str,
    api_authed: &ApiAuthed,
) -> Result<serde_json::Value, Error> {
    let args_map = match &args {
        serde_json::Value::Object(map) => map,
        _ => return Err(Error::invalid_params("Arguments must be an object", Some(tool.name.clone().into()))),
    };

    // Build URL with path substitutions
    let path_template = substitute_path_params(&tool.path, workspace_id, args_map, &tool.path_params_schema);
    let query_string = build_query_string(args_map, &tool.query_params_schema);
    let full_url = format!("{}/api{}{}", BASE_URL.read().await, path_template, query_string);

    // Prepare request body
    let body_json = build_request_body(&tool.method, args_map, &tool.body_schema);

    // Create and execute request
    let response = create_http_request(&tool.method, &full_url, workspace_id, api_authed, body_json).await?;
    
    let status = response.status();
    let response_text = response.text().await.map_err(|e| {
        Error::internal_error(format!("Failed to read response text: {}", e), None)
    })?;

    if status.is_success() {
        Ok(serde_json::from_str(&response_text).unwrap_or_else(|_| serde_json::Value::String(response_text)))
    } else {
        Err(Error::internal_error(
            format!("HTTP {} {}: {}", status.as_u16(), status.canonical_reason().unwrap_or(""), response_text),
            None
        ))
    }
}

fn substitute_path_params(
    path: &str,
    workspace_id: &str,
    args_map: &serde_json::Map<String, serde_json::Value>,
    path_schema: &Option<serde_json::Value>,
) -> String {
    let mut path_template = path.replace("{workspace}", workspace_id);
    
    if let Some(schema) = path_schema {
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (param_name, _) in props {
                let placeholder = format!("{{{}}}", param_name);
                if let Some(param_value) = args_map.get(param_name) {
                    if let Some(str_val) = param_value.as_str() {
                        path_template = path_template.replace(&placeholder, str_val);
                    }
                }
            }
        }
    }
    
    path_template
}

fn build_query_string(
    args_map: &serde_json::Map<String, serde_json::Value>,
    query_schema: &Option<serde_json::Value>,
) -> String {
    let Some(schema) = query_schema else { return String::new() };
    let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else { return String::new() };
    
    let query_params: Vec<String> = props
        .keys()
        .filter_map(|param_name| {
            args_map.get(param_name)
                .filter(|v| !v.is_null())
                .map(|value| {
                    let value_str = value.to_string();
                    let str_val = value_str.trim_matches('"');
                    format!("{}={}", 
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

fn build_request_body(
    method: &http::Method,
    args_map: &serde_json::Map<String, serde_json::Value>,
    body_schema: &Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    if method == &http::Method::GET {
        return None;
    }
    
    let schema = body_schema.as_ref()?;
    let props = schema.get("properties")?.as_object()?;
    
    let body_map: serde_json::Map<String, serde_json::Value> = props
        .keys()
        .filter_map(|param_name| {
            args_map.get(param_name)
                .map(|value| (param_name.clone(), value.clone()))
        })
        .collect();
    
    if body_map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(body_map))
    }
}

async fn create_http_request(
    method: &http::Method,
    url: &str,
    workspace_id: &str,
    api_authed: &ApiAuthed,
    body_json: Option<serde_json::Value>,
) -> Result<reqwest::Response, Error> {
    let client = &crate::HTTP_CLIENT;
    let mut request_builder = match method {
        &http::Method::GET => client.get(url),
        &http::Method::POST => client.post(url),
        &http::Method::PUT => client.put(url),
        &http::Method::DELETE => client.delete(url),
        &http::Method::PATCH => client.patch(url),
        _ => return Err(Error::invalid_params(
            format!("Unsupported HTTP method: {}", method),
            None
        )),
    };

    // Add authorization header
    let authed = Authed::from(api_authed.clone());
    let token = create_jwt_token(authed, workspace_id, 3600, None, None, None, None).await
        .map_err(|e| Error::internal_error(e.to_string(), None))?;
    request_builder = request_builder.header("Authorization", format!("Bearer {}", token));

    // Add body if present
    if let Some(body) = body_json {
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .json(&body);
    }

    request_builder.send().await.map_err(|e| {
        Error::internal_error(format!("Failed to execute request: {}", e), None)
    })
}