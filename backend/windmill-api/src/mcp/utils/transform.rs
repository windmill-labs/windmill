//! Transformation utilities for MCP server
//!
//! Contains functions for transforming paths, keys, and other identifiers
//! to make them compatible with MCP tool naming requirements.

use super::models::SchemaType;

// MCP clients do not allow names longer than 60 characters
const MAX_PATH_LENGTH: usize = 60;

/// Transform the path for workspace scripts/flows
///
/// This function takes a path and a type string and formats the transformed
/// path with the type prefix. This is used when listing, because we can't
/// have names with slashes. Because we replace slashes with underscores,
/// we also need to escape underscores.
pub fn transform_path(path: &str, type_str: &str) -> String {
    let escaped_path = path.replace('_', "__").replace('/', "_");
    // first letter of type_str is used as prefix, only one letter to avoid reaching 60 char name limit
    let transformed_path = format!("{}-{}", &type_str[..1], escaped_path);
    if transformed_path.len() > MAX_PATH_LENGTH {
        let suffix = "_TRUNC";
        return format!(
            "{}{}",
            &transformed_path[..MAX_PATH_LENGTH - suffix.len()],
            suffix
        );
    }
    transformed_path
}

/// Reverse the transformation of a path
///
/// This function takes a transformed path and reverses the transformation
/// applied by `transform_path`. It checks if the path starts with "h"
/// (indicating a Hub script) and removes the prefix if present.
/// It then determines the type of the item (script or flow) based on the prefix.
/// This is used in call_tool to get the original path, and the type of the item.
///
/// Returns: (type, original_path, is_hub)
pub fn reverse_transform(transformed_path: &str) -> Result<(&str, String, bool), String> {
    let is_hub = transformed_path.starts_with("h");
    let transformed_path = if is_hub {
        transformed_path[1..].to_string()
    } else {
        transformed_path.to_string()
    };
    let type_str = if transformed_path.starts_with("s-") {
        "script"
    } else if transformed_path.starts_with("f-") {
        "flow"
    } else {
        return Err(format!(
            "Invalid prefix in transformed path: {}",
            transformed_path
        ));
    };

    let mangled_path = &transformed_path[2..];

    let original_path = if is_hub {
        let parts = mangled_path.split("-").collect::<Vec<&str>>();
        if parts.is_empty() {
            return Err(format!("Invalid transformed path: {}", transformed_path));
        }
        parts[0].to_string()
    } else {
        const TEMP_PLACEHOLDER: &str = "@@UNDERSCORE@@";
        mangled_path
            .replace("__", TEMP_PLACEHOLDER)
            .replace('_', "/")
            .replace(TEMP_PLACEHOLDER, "_")
    };

    Ok((type_str, original_path, is_hub))
}

/// Apply key transformation to a key
///
/// This function takes a key and replaces spaces with underscores.
/// It also removes any characters that are not alphanumeric or underscores.
/// This is used when listing, because we can't have names with spaces
/// or special characters in the schema properties.
pub fn apply_key_transformation(key: &str) -> String {
    key.replace(' ', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
}

/// Reverse the transformation of a key
///
/// This function takes a transformed key and a schema object and reverses
/// the transformation applied by `apply_key_transformation`. This can be
/// subject to collisions, but it's unlikely and is ok for our use case.
pub fn reverse_transform_key(transformed_key: &str, schema_obj: &Option<SchemaType>) -> String {
    let schema_obj = match schema_obj {
        Some(s) => s,
        None => {
            // No schema available, return the key as is (best guess)
            return transformed_key.to_string();
        }
    };

    for original_key_in_schema in schema_obj.properties.keys() {
        // Apply the SAME forward transformation to the schema key
        let potential_transformed_key = apply_key_transformation(original_key_in_schema);

        // If it matches the key we received, we found the likely original
        if potential_transformed_key == transformed_key {
            return original_key_in_schema.clone();
        }
    }

    transformed_key.to_string()
}
