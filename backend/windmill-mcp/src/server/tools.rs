//! Tool creation utilities for MCP server
//!
//! Contains functionality for converting Windmill items (scripts, flows, hub scripts)
//! into MCP tools.

use rmcp::model::{Tool, ToolAnnotations};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::schema::{convert_schema_to_schema_type, make_schema_compatible};
use crate::common::transform::transform_path;
use crate::common::types::{
    FlowInfo, HubScriptInfo, ResourceInfo, ResourceType, SchemaType, ScriptInfo, ToolableItem,
};
use crate::server::backend::McpBackend;

/// Implementation of ToolableItem for ScriptInfo
impl ToolableItem for ScriptInfo {
    fn get_path_or_id(&self) -> String {
        transform_path(&self.path, "script")
    }

    fn get_summary(&self) -> &str {
        self.summary.as_deref().unwrap_or("No summary")
    }

    fn get_description(&self) -> &str {
        self.description.as_deref().unwrap_or("No description")
    }

    fn get_schema(&self) -> SchemaType {
        convert_schema_to_schema_type(self.schema.clone())
    }

    fn is_hub(&self) -> bool {
        false
    }

    fn item_type(&self) -> &'static str {
        "script"
    }

    fn get_integration_type(&self) -> Option<String> {
        None
    }
}

/// Implementation of ToolableItem for FlowInfo
impl ToolableItem for FlowInfo {
    fn get_path_or_id(&self) -> String {
        transform_path(&self.path, "flow")
    }

    fn get_summary(&self) -> &str {
        self.summary.as_deref().unwrap_or("No summary")
    }

    fn get_description(&self) -> &str {
        self.description.as_deref().unwrap_or("No description")
    }

    fn get_schema(&self) -> SchemaType {
        convert_schema_to_schema_type(self.schema.clone())
    }

    fn is_hub(&self) -> bool {
        false
    }

    fn item_type(&self) -> &'static str {
        "flow"
    }

    fn get_integration_type(&self) -> Option<String> {
        None
    }
}

/// Implementation of ToolableItem for HubScriptInfo
impl ToolableItem for HubScriptInfo {
    fn get_path_or_id(&self) -> String {
        let id = self.version_id;
        let summary = self.summary.as_deref().unwrap_or("No summary");
        format!("hs-{}-{}", id, summary.replace(" ", "_"))
    }

    fn get_summary(&self) -> &str {
        self.summary.as_deref().unwrap_or("No summary")
    }

    fn get_description(&self) -> &str {
        self.description.as_deref().unwrap_or("No description")
    }

    fn get_schema(&self) -> SchemaType {
        match serde_json::from_value::<SchemaType>(self.schema.clone().unwrap_or_default()) {
            Ok(schema_type) => schema_type,
            Err(_) => SchemaType::default(),
        }
    }

    fn is_hub(&self) -> bool {
        true
    }

    fn item_type(&self) -> &'static str {
        "script"
    }

    fn get_integration_type(&self) -> Option<String> {
        self.app.clone()
    }
}

/// Create an MCP Tool from a ToolableItem
///
/// The resources_cache should be pre-populated with all resource types
/// that may be referenced by the item's schema.
pub fn create_tool_from_item<T: ToolableItem, B: McpBackend>(
    item: &T,
    backend: &B,
    resources_cache: &HashMap<String, Vec<ResourceInfo>>,
    resources_types: &[ResourceType],
) -> Tool {
    let is_hub = item.is_hub();
    let path = item.get_path_or_id();
    let item_type = item.item_type();
    let description = format!(
        "This is a {} named `{}` with the following description: `{}`.{}",
        item_type,
        item.get_summary(),
        item.get_description(),
        if is_hub {
            format!(
                " It is a tool used for the following app: {}",
                item.get_integration_type()
                    .unwrap_or("No integration type".to_string())
            )
        } else {
            "".to_string()
        }
    );

    let schema = item.get_schema();
    let schema_obj =
        backend.transform_schema_for_resources(&schema, resources_cache, resources_types);

    let input_schema_map = match serde_json::to_value(schema_obj) {
        Ok(mut value) => {
            make_schema_compatible(&mut value);
            match value {
                serde_json::Value::Object(map) => map,
                _ => {
                    tracing::warn!(
                        "Schema object for tool '{}' did not serialize to a JSON object, using empty schema.",
                        path
                    );
                    serde_json::Map::new()
                }
            }
        }
        Err(e) => {
            tracing::error!(
                "Failed to serialize schema object for tool '{}': {}. Using empty schema.",
                path,
                e
            );
            serde_json::Map::new()
        }
    };

    Tool {
        name: Cow::Owned(path),
        description: Some(Cow::Owned(description)),
        input_schema: Arc::new(input_schema_map),
        title: Some(item.get_summary().to_string()),
        output_schema: None,
        icons: None,
        annotations: Some(ToolAnnotations {
            title: Some(item.get_summary().to_string()),
            read_only_hint: Some(false),  // Can modify environment
            destructive_hint: Some(true), // Can potentially be destructive
            idempotent_hint: Some(false), // Are not guaranteed to be idempotent
            open_world_hint: Some(true),  // Can interact with external services
        }),
        meta: None,
        execution: None,
    }
}
