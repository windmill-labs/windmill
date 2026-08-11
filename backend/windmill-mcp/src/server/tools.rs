//! Tool creation utilities for MCP server
//!
//! Contains functionality for converting Windmill items (scripts, flows, hub scripts)
//! into MCP tools.

use rmcp::model::{Tool, ToolAnnotations};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::common::schema::{convert_schema_to_schema_type, make_schema_compatible};
use crate::common::transform::{transform_hub_path, transform_path};
use crate::common::types::{
    FlowInfo, HubScriptInfo, ResourceInfo, ResourceType, SchemaType, ScriptInfo, ToolableItem,
};
use crate::server::backend::McpBackend;

/// Implementation of ToolableItem for ScriptInfo
impl ToolableItem for ScriptInfo {
    fn get_transformed_path(&self) -> String {
        transform_path(&self.path, "script")
    }

    fn get_full_path(&self) -> &str {
        &self.path
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
    fn get_transformed_path(&self) -> String {
        transform_path(&self.path, "flow")
    }

    fn get_full_path(&self) -> &str {
        &self.path
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
    fn get_transformed_path(&self) -> String {
        let summary = self.summary.as_deref().unwrap_or("No summary");
        transform_hub_path(self.version_id, summary)
    }

    fn get_full_path(&self) -> &str {
        self.summary.as_deref().unwrap_or("No summary")
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

/// Spell out, in the tool's own description, which parameters may be left out.
///
/// A tool schema that does not opt into strict function calling lets the model omit
/// anything outside `required`, but models routinely fill every advertised property
/// with a placeholder (`""`, `[]`, `false`, `0`) instead, which the script then reads
/// as an explicit empty value rather than an absent one. The tool description carries
/// far more weight for that decision than the caller's system prompt, so the rule has
/// to live here rather than being left to whoever configures the client.
///
/// Kept terse: it repeats on every tool in the list. It also promises nothing about
/// what an omitted parameter becomes -- a script falls back to its signature default,
/// a flow input is simply absent.
///
/// Returns `None` when there is no optional parameter to mention.
fn optional_params_hint(input_schema: &Map<String, Value>) -> Option<String> {
    let properties = input_schema.get("properties")?.as_object()?;

    let required: HashSet<&str> = input_schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|names| names.iter().filter_map(|n| n.as_str()).collect())
        .unwrap_or_default();

    let mut optional: Vec<&str> = properties
        .keys()
        .map(String::as_str)
        .filter(|name| !required.contains(name))
        .collect();
    if optional.is_empty() {
        return None;
    }
    // `properties` comes from an unordered map, so sort for a stable description.
    optional.sort_unstable();

    let names = optional
        .iter()
        .map(|name| format!("`{}`", name))
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!(
        " Optional parameters: {}. Omit any you were not given a value for rather than sending `\"\"`, `[]`, `{{}}`, `false`, or `0` as a placeholder.",
        names
    ))
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
    let path = item.get_transformed_path();
    let item_type = item.item_type();
    let mut description = format!(
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

    if let Some(hint) = optional_params_hint(&input_schema_map) {
        description.push_str(&hint);
    }

    let title = {
        let summary = item.get_summary();
        if summary == "No summary" {
            item.get_full_path().to_string()
        } else {
            summary.to_string()
        }
    };

    Tool::new(
        Cow::Owned(path),
        Cow::Owned(description),
        Arc::new(input_schema_map),
    )
    .with_title(title.clone())
    .with_annotations(
        ToolAnnotations::with_title(title)
            .read_only(false) // Can modify environment
            .destructive(true) // Can potentially be destructive
            .idempotent(false) // Are not guaranteed to be idempotent
            .open_world(true), // Can interact with external services
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input_schema(raw: Value) -> Map<String, Value> {
        raw.as_object().unwrap().clone()
    }

    #[test]
    fn hint_lists_optional_params_and_never_a_required_one() {
        let hint = optional_params_hint(&input_schema(json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "page": { "type": "number", "default": 1 },
                "filters": { "type": "object" },
            },
            "required": ["query"],
        })))
        .expect("a schema with optional params gets a hint");

        assert!(
            hint.contains("Optional parameters: `filters`, `page`."),
            "{hint}"
        );
        assert!(!hint.contains("`query`"), "{hint}");
    }

    #[test]
    fn no_hint_when_every_param_is_required() {
        assert!(optional_params_hint(&input_schema(json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"],
        })))
        .is_none());
    }
}
