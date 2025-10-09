//! Convert between MCP and Windmill AI tool formats

use crate::ai::types::{ToolDef, ToolDefFunction};
use anyhow::{Context, Result};
use rmcp::model::Tool as McpTool;
use serde_json::value::RawValue;
use windmill_common::worker::to_raw_value;

/// Convert an MCP tool to Windmill's ToolDef format
///
/// # Arguments
/// * `mcp_tool` - The MCP tool to convert
/// * `resource_path` - The resource path to use as a prefix for the tool name
///
/// # Returns
/// A ToolDef that can be used in Windmill's AI agent system
pub fn mcp_tool_to_tooldef(mcp_tool: &McpTool, resource_path: &str) -> Result<ToolDef> {
    // Create a unique tool name by prefixing with a sanitized resource path
    // This prevents naming conflicts between different MCP servers
    let sanitized_resource = resource_path
        .replace('/', "_")
        .replace('.', "_")
        .trim_start_matches('_')
        .to_string();

    let tool_name = if sanitized_resource.is_empty() {
        mcp_tool.name.to_string()
    } else {
        format!("mcp_{}_{}", sanitized_resource, mcp_tool.name)
    };

    // Convert the input schema to RawValue
    // MCP uses JSON Schema which is compatible with OpenAPI schema
    let parameters: Box<RawValue> = to_raw_value(&mcp_tool.input_schema);

    // Build the description from title and description
    let description = if let Some(title) = &mcp_tool.title {
        if let Some(desc) = &mcp_tool.description {
            Some(format!("{}: {}", title, desc))
        } else {
            Some(title.to_string())
        }
    } else {
        mcp_tool.description.as_ref().map(|d| d.to_string())
    };

    let tool_def_function = ToolDefFunction {
        name: tool_name.clone(),
        description,
        parameters,
    };

    Ok(ToolDef {
        r#type: "function".to_string(),
        function: tool_def_function,
    })
}

/// Convert OpenAI-style tool call arguments to MCP format
///
/// OpenAI sends arguments as a JSON string, MCP expects a Map
///
/// # Arguments
/// * `args_str` - JSON string of arguments from OpenAI tool call
///
/// # Returns
/// A Map suitable for MCP tool calls
pub fn openai_args_to_mcp_args(
    args_str: &str,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>> {
    if args_str.trim().is_empty() {
        return Ok(None);
    }

    let args_value: serde_json::Value = serde_json::from_str(args_str)
        .context("Failed to parse tool call arguments")?;

    match args_value {
        serde_json::Value::Object(map) => Ok(Some(map)),
        serde_json::Value::Null => Ok(None),
        _ => Ok(Some(
            vec![("value".to_string(), args_value)]
                .into_iter()
                .collect(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_mcp_tool_to_tooldef() {
        let input_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        });

        let mcp_tool = McpTool {
            name: "search".into(),
            title: Some("Search Tool".to_string()),
            description: Some("Search for information".into()),
            input_schema: Arc::new(input_schema.as_object().unwrap().clone()),
            output_schema: None,
            icons: None,
            annotations: None,
        };

        let tooldef = mcp_tool_to_tooldef(&mcp_tool, "f/team/server").unwrap();

        assert_eq!(tooldef.function.name, "mcp_f_team_server_search");
        assert_eq!(
            tooldef.function.description.as_deref(),
            Some("Search Tool: Search for information")
        );
    }

    #[test]
    fn test_openai_args_to_mcp_args() {
        let args_str = r#"{"query": "test", "limit": 10}"#;
        let result = openai_args_to_mcp_args(args_str).unwrap();

        assert!(result.is_some());
        let map = result.unwrap();
        assert_eq!(map.get("query").unwrap().as_str(), Some("test"));
        assert_eq!(map.get("limit").unwrap().as_i64(), Some(10));
    }

    #[test]
    fn test_openai_args_empty() {
        let result = openai_args_to_mcp_args("").unwrap();
        assert!(result.is_none());
    }
}
