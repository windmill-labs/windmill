//! Schema transformation utilities for MCP server
//!
//! Contains functions for transforming Windmill schemas into MCP-compatible formats,
//! including resource enrichment and schema conversion utilities.

use rmcp::ErrorData;
use serde_json::Value;
use std::collections::HashMap;
use windmill_common::db::UserDB;
use windmill_common::scripts::Schema;

use super::database::get_resources;
use super::models::{ResourceInfo, ResourceType, SchemaType};
use super::transform::apply_key_transformation;
use crate::db::ApiAuthed;

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

/// Transform the schema for resources by enriching with resource information
pub async fn transform_schema_for_resources(
    schema: &SchemaType,
    user_db: &UserDB,
    authed: &ApiAuthed,
    w_id: &str,
    resources_cache: &mut HashMap<String, Vec<ResourceInfo>>,
    resources_types: &Vec<ResourceType>,
) -> Result<SchemaType, ErrorData> {
    let mut schema_obj: SchemaType = schema.clone();

    // replace invalid char in property key with underscore
    let replacements: Vec<(String, String, Value)> = schema_obj
        .properties
        .iter()
        .filter_map(|(key, value)| {
            if key.chars().any(|c| !c.is_alphanumeric() && c != '_') {
                let new_key = apply_key_transformation(key);
                Some((key.clone(), new_key, value.clone()))
            } else {
                None
            }
        })
        .collect();

    for (old_key, new_key, value) in replacements {
        schema_obj.properties.remove(&old_key);
        schema_obj.properties.insert(new_key, value);
    }

    for (_key, prop_value) in schema_obj.properties.iter_mut() {
        if let Value::Object(prop_map) = prop_value {
            // if property is a resource, fetch the resource type infos, and add each available resource to the description
            if let Some(format_value) = prop_map.get("format") {
                if let Value::String(format_str) = format_value {
                    if format_str.starts_with("resource-") {
                        let resource_type_key =
                            format_str.split("-").last().unwrap_or_default().to_string();
                        let resource_type = resources_types
                            .iter()
                            .find(|rt| rt.name == resource_type_key);
                        let resource_type_obj = resource_type.cloned();

                        if !resources_cache.contains_key(&resource_type_key) {
                            let available_resources =
                                get_resources(user_db, authed, &w_id, &resource_type_key).await;

                            match available_resources {
                                Ok(cache_data) => {
                                    resources_cache.insert(resource_type_key.clone(), cache_data);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to fetch resource cache data: {}", e);
                                    continue; // Skip this property if fetching failed
                                }
                            }
                        }

                        if let Some(resource_cache) = resources_cache.get(&resource_type_key) {
                            let resources_count = resource_cache.len();
                            let description = match resource_type_obj {
                                Some(resource_type_obj) => format!(
                                    "This is a resource named `{}` with the following description: `{}`.\\nThe path of the resource should be used to specify the resource.\\n{}",
                                    resource_type_obj.name,
                                    resource_type_obj.description.as_deref().unwrap_or("No description"),
                                    if resources_count == 0 {
                                        "This resource does not have any available instances, you should create one from your windmill workspace."
                                    } else if resources_count > 1 {
                                        "This resource has multiple available instances, you should precisely select the one you want to use."
                                    } else {
                                        "There is 1 resource available."
                                    }
                                ),
                                None => "An object parameter.".to_string()
                            };
                            prop_map
                                .insert("type".to_string(), Value::String("string".to_string()));
                            prop_map.insert("description".to_string(), Value::String(description));
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

                                prop_map.insert(
                                    "description".to_string(),
                                    Value::String(format!(
                                        "{}\\nHere are the available resources, in the format title:path. Title can be empty. Path should be used to specify the resource:\\n{}",
                                        prop_map.get("description").unwrap_or(&Value::String("No description".to_string())),
                                        resources_description
                                    )),
                                );
                            }
                        }
                    }
                }
            }
        } else {
            tracing::warn!(
                "Schema property value is not a JSON object: {:?}",
                prop_value
            );
        }
    }

    Ok(schema_obj)
}
