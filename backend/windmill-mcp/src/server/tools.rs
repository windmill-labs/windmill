//! Tool creation utilities for MCP server
//!
//! Contains functionality for converting Windmill items (scripts, flows, hub scripts)
//! into MCP tools.

use rmcp::model::{Tool, ToolAnnotations};
use std::borrow::Cow;
use std::collections::HashMap;
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

/// OpenAI rejects a chat-completions request outright when any `tools[].function.description`
/// exceeds this, and the agent step copies an MCP tool's description into that field verbatim.
/// One over-long tool would fail the request for every tool in the step, so the hint below is
/// only ever appended when it fits in what the base description leaves.
const MAX_TOOL_DESCRIPTION_CHARS: usize = 1024;

/// Spell out, in the tool description, that optional parameters may be left out.
///
/// Models given a non-strict tool schema tend to fill every property with a type-zero
/// value (`""`, `[]`, `0`, `false`), which reaches the runnable as a real argument rather
/// than as the absent value it stands in for. `required` alone does not deter this, and a
/// client-side system prompt is weighed far less than the tool's own description.
///
/// The wording must not promise that an omitted parameter falls back to a default: only a
/// script gets that, from its own language-level default. Nothing applies schema defaults
/// server-side, so an omitted flow input is simply absent.
///
/// Returns the longest form that fits `budget` characters: naming the parameters is worth
/// more than the generic sentence alone, but the names are the unbounded part and are
/// already in the schema, so they are the first thing dropped.
fn optional_params_hint(schema: &SchemaType, budget: usize) -> Option<String> {
    const INSTRUCTION: &str = "Omit any parameter the request does not call for; leaving one out is always valid and is not the same as passing an empty value. Never pass an empty string, empty array, empty object, `false`, or `0` as a placeholder for a value you were not given.";

    let mut optional = schema
        .properties
        .keys()
        .filter(|key| !schema.required.contains(*key))
        .map(|key| format!("`{}`", key))
        .collect::<Vec<_>>();

    if optional.is_empty() {
        return None;
    }

    // `properties` is a HashMap, so its order varies per call. Sort: tool definitions sit
    // in the cached prefix of a provider request, which only matches when byte-identical.
    optional.sort_unstable();

    let enumerated = format!(
        " Optional parameters: {}. {}",
        optional.join(", "),
        INSTRUCTION
    );
    if enumerated.chars().count() <= budget {
        return Some(enumerated);
    }

    let generic = format!(" {}", INSTRUCTION);
    if generic.chars().count() <= budget {
        return Some(generic);
    }

    None
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

    let schema = item.get_schema();
    let schema_obj =
        backend.transform_schema_for_resources(&schema, resources_cache, resources_types);

    let base_description = format!(
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

    // The hint is derived from the transformed schema, so the names it lists are the ones
    // the client receives, and it only gets appended if it fits alongside the base.
    let budget = MAX_TOOL_DESCRIPTION_CHARS.saturating_sub(base_description.chars().count());
    let description = match optional_params_hint(&schema_obj, budget) {
        Some(hint) => format!("{}{}", base_description, hint),
        None => base_description,
    };

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

    fn schema(properties: &[&str], required: &[&str]) -> SchemaType {
        SchemaType {
            r#type: "object".to_string(),
            properties: properties
                .iter()
                .map(|key| (key.to_string(), serde_json::json!({ "type": "string" })))
                .collect(),
            required: required.iter().map(|key| key.to_string()).collect(),
        }
    }

    #[test]
    fn hint_lists_every_optional_param_sorted() {
        let hint = optional_params_hint(
            &schema(&["query", "sort", "filters", "page"], &["query"]),
            MAX_TOOL_DESCRIPTION_CHARS,
        )
        .expect("a schema with optional params must produce a hint");

        assert!(
            hint.starts_with(" Optional parameters: `filters`, `page`, `sort`."),
            "unexpected hint: {hint}"
        );
        assert!(!hint.contains("`query`"), "required param listed: {hint}");
    }

    /// OpenAI 400s the whole request over a 1024-char tool description, taking every other
    /// tool in the step down with it, so the hint sheds the parameter names and then itself
    /// rather than overrun the budget.
    #[test]
    fn hint_degrades_then_disappears_as_the_budget_shrinks() {
        let many = (0..200).map(|i| format!("param_{i}")).collect::<Vec<_>>();
        let names = many.iter().map(String::as_str).collect::<Vec<_>>();
        let wide = schema(&names, &[]);

        let full = optional_params_hint(&wide, usize::MAX).expect("unbounded budget lists names");
        assert!(full.contains("`param_0`"), "expected names: {full}");

        let generic = optional_params_hint(&wide, 400).expect("a tight budget keeps the sentence");
        assert!(
            !generic.contains("`param_0`"),
            "names not dropped: {generic}"
        );
        assert!(generic.chars().count() <= 400, "over budget: {generic}");

        assert_eq!(optional_params_hint(&wide, 10), None);
    }

    #[test]
    fn no_hint_when_every_param_is_required() {
        assert_eq!(
            optional_params_hint(&schema(&["query"], &["query"]), MAX_TOOL_DESCRIPTION_CHARS),
            None
        );
        assert_eq!(
            optional_params_hint(&schema(&[], &[]), MAX_TOOL_DESCRIPTION_CHARS),
            None
        );
    }
}
