//! Transformation utilities for MCP server
//!
//! Contains functions for transforming paths, keys, and other identifiers
//! to make them compatible with MCP tool naming requirements.

use super::types::SchemaType;
use windmill_common::utils::calculate_hash;

/// Max tool name length. The MCP spec allows 64 chars, but some clients
/// (e.g. Cursor) prepend the server name to the tool name, so we use 40
/// to leave room for that prefix.
const MAX_PATH_LENGTH: usize = 40;

/// Length of the SHA256 hash suffix used for hashed names
const HASH_LEN: usize = 16;

/// Transform the path for workspace scripts/flows
///
/// This function takes a path and a type string and formats the transformed
/// path with the type prefix. This is used when listing, because we can't
/// have names with slashes. Because we replace slashes with underscores,
/// we also need to escape underscores.
///
/// For short names (≤60 chars): `s-{escaped_path}` or `f-{escaped_path}`
/// For long names (>60 chars): `S-{escaped[:42]}{sha256[:16]}` or `F-{escaped[:42]}{sha256[:16]}`
///
/// The uppercase prefix signals that the name is hashed.
pub fn transform_path(path: &str, type_str: &str) -> String {
    let escaped_path = path.replace('_', "__").replace('/', "_");
    let prefix_char = &type_str[..1];
    let short_name = format!("{}-{}", prefix_char, escaped_path);

    if short_name.len() <= MAX_PATH_LENGTH {
        return short_name;
    }

    let upper_prefix = prefix_char.to_uppercase();
    // Layout: "{Upper}-" (2 chars) + prefix_body (42 chars) + hash (16 chars) = 60
    let prefix_body_len = MAX_PATH_LENGTH - 2 - HASH_LEN;
    let hash = calculate_hash(&short_name);
    let hash_suffix = &hash[..HASH_LEN];
    let truncated = truncate_to_char_boundary(&escaped_path, prefix_body_len);
    format!("{}-{}{}", upper_prefix, truncated, hash_suffix)
}

/// Transform the path for hub scripts
///
/// For short names (≤60 chars): `hs-{id}-{summary}`
/// For long names (>60 chars): `Hs-{id}-{summary[:N]}{sha256[:16]}`
pub fn transform_hub_path(version_id: u64, summary: &str) -> String {
    let escaped_summary = summary.replace(' ', "_");
    let short_name = format!("hs-{}-{}", version_id, escaped_summary);

    if short_name.len() <= MAX_PATH_LENGTH {
        return short_name;
    }

    let hash = calculate_hash(&short_name);
    let hash_suffix = &hash[..HASH_LEN];
    // "Hs-{id}-" prefix, then fill remaining with summary + hash
    let fixed_prefix = format!("Hs-{}-", version_id);
    let available = MAX_PATH_LENGTH - fixed_prefix.len() - HASH_LEN;
    let truncated_summary = truncate_to_char_boundary(&escaped_summary, available);
    format!("{}{}{}", fixed_prefix, truncated_summary, hash_suffix)
}

/// Check if a tool name is a hashed (long) name.
/// Hashed names have an uppercase first character.
pub fn is_hashed_name(name: &str) -> bool {
    name.chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
}

/// Extract the item type from a hashed name.
/// Returns the type string ("script" or "flow") and whether it's a hub script.
pub fn parse_hashed_name(name: &str) -> Result<(&str, bool), String> {
    if name.starts_with("S-") {
        Ok(("script", false))
    } else if name.starts_with("F-") {
        Ok(("flow", false))
    } else if name.starts_with("Hs-") {
        Ok(("script", true))
    } else {
        Err(format!("Invalid hashed name prefix: {}", name))
    }
}

/// Extract the hub version_id from a hashed hub script name like `Hs-{id}-...`
pub fn extract_hub_version_id_from_hashed(name: &str) -> Result<String, String> {
    let rest = name
        .strip_prefix("Hs-")
        .ok_or_else(|| format!("Not a hashed hub name: {}", name))?;
    let id = rest
        .split('-')
        .next()
        .ok_or_else(|| format!("No version_id in hashed hub name: {}", name))?;
    if id.is_empty() {
        return Err(format!("Empty version_id in hashed hub name: {}", name));
    }
    Ok(id.to_string())
}

/// Extract a safe original-path prefix from a hashed workspace tool name.
///
/// Given `S-u_admin_engineering__te<hash16>`, extracts the escaped prefix between
/// `S-` and the hash, un-escapes it, and returns a prefix suitable for
/// `WHERE path LIKE '{prefix}%'`.
///
/// Returns `None` if the name is too short to contain a meaningful prefix.
pub fn extract_path_prefix_from_hashed(name: &str) -> Option<String> {
    // Strip the 2-char type prefix ("S-" or "F-") and the 16-char hash suffix
    if name.len() <= 2 + HASH_LEN {
        return None;
    }
    let escaped_prefix = &name[2..name.len() - HASH_LEN];
    if escaped_prefix.is_empty() {
        return None;
    }

    // Strip trailing underscores — they may be half of a `__` pair split by truncation
    let trimmed = escaped_prefix.trim_end_matches('_');
    if trimmed.is_empty() {
        return None;
    }

    // Un-escape: `__` → `_`, standalone `_` → `/`
    const TEMP_PLACEHOLDER: &str = "@@UNDERSCORE@@";
    let prefix = trimmed
        .replace("__", TEMP_PLACEHOLDER)
        .replace('_', "/")
        .replace(TEMP_PLACEHOLDER, "_");

    Some(prefix)
}

/// Truncate a string to at most `max_len` bytes, ensuring we don't split a UTF-8 character.
fn truncate_to_char_boundary(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        return s;
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
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
///
/// Note: This only works for non-hashed (short) names. Hashed names must be
/// resolved via `is_hashed_name` + path enumeration in the runner.
pub fn reverse_transform(transformed_path: &str) -> Result<(&str, String, bool), String> {
    if is_hashed_name(transformed_path) {
        return Err(
            "Hashed names cannot be reverse-transformed directly; use path enumeration instead"
                .to_string(),
        );
    }

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
            return transformed_key.to_string();
        }
    };

    for original_key_in_schema in schema_obj.properties.keys() {
        let potential_transformed_key = apply_key_transformation(original_key_in_schema);

        if potential_transformed_key == transformed_key {
            return original_key_in_schema.clone();
        }
    }

    transformed_key.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_path_short() {
        assert_eq!(
            transform_path("u/admin/script", "script"),
            "s-u_admin_script"
        );
        assert_eq!(transform_path("f/folder/flow", "flow"), "f-f_folder_flow");
        assert_eq!(transform_path("my_script", "script"), "s-my__script");
    }

    #[test]
    fn test_transform_path_long_is_hashed() {
        let long_path = "u/engineering/team/automation/very_long_script_name_that_exceeds_limit";
        let result = transform_path(long_path, "script");
        assert_eq!(result.len(), MAX_PATH_LENGTH);
        assert!(result.starts_with("S-"));
        assert!(is_hashed_name(&result));
    }

    #[test]
    fn test_transform_path_long_flow_is_hashed() {
        let long_path = "f/engineering/team/automation/very_long_flow_name_that_exceeds_limit";
        let result = transform_path(long_path, "flow");
        assert_eq!(result.len(), MAX_PATH_LENGTH);
        assert!(result.starts_with("F-"));
        assert!(is_hashed_name(&result));
    }

    #[test]
    fn test_transform_path_hashing_is_deterministic() {
        let path = "u/engineering/team/automation/very_long_script_name_that_exceeds_limit";
        let a = transform_path(path, "script");
        let b = transform_path(path, "script");
        assert_eq!(a, b);
    }

    #[test]
    fn test_transform_path_different_long_paths_differ() {
        let a = transform_path(
            "u/engineering/team/automation/very_long_script_name_that_exceeds_limit_a",
            "script",
        );
        let b = transform_path(
            "u/engineering/team/automation/very_long_script_name_that_exceeds_limit_b",
            "script",
        );
        assert_ne!(a, b);
    }

    #[test]
    fn test_transform_hub_path_short() {
        let result = transform_hub_path(12345, "Send Slack Message");
        assert_eq!(result, "hs-12345-Send_Slack_Message");
        assert!(!is_hashed_name(&result));
    }

    #[test]
    fn test_transform_hub_path_long_is_hashed() {
        let result = transform_hub_path(
            12345,
            "Send Slack Message To Channel With Very Long Description That Exceeds Limit",
        );
        assert_eq!(result.len(), MAX_PATH_LENGTH);
        assert!(result.starts_with("Hs-12345-"));
        assert!(is_hashed_name(&result));
    }

    #[test]
    fn test_extract_hub_version_id_from_hashed() {
        let name = "Hs-12345-Send_Slack_Message_To_Ch9e8d7c6b5a4f3e2d";
        let id = extract_hub_version_id_from_hashed(name).unwrap();
        assert_eq!(id, "12345");
    }

    #[test]
    fn test_is_hashed_name() {
        assert!(is_hashed_name("S-u_admin_script_abc123"));
        assert!(is_hashed_name("F-f_folder_flow_abc123"));
        assert!(is_hashed_name("Hs-12345-summary"));
        assert!(!is_hashed_name("s-u_admin_script"));
        assert!(!is_hashed_name("f-f_folder_flow"));
        assert!(!is_hashed_name("hs-12345-summary"));
    }

    #[test]
    fn test_parse_hashed_name() {
        let (t, hub) = parse_hashed_name("S-something").unwrap();
        assert_eq!(t, "script");
        assert!(!hub);

        let (t, hub) = parse_hashed_name("F-something").unwrap();
        assert_eq!(t, "flow");
        assert!(!hub);

        let (t, hub) = parse_hashed_name("Hs-12345-something").unwrap();
        assert_eq!(t, "script");
        assert!(hub);
    }

    #[test]
    fn test_reverse_transform_short_names() {
        let (type_str, path, is_hub) = reverse_transform("s-u_admin_script").unwrap();
        assert_eq!(type_str, "script");
        assert_eq!(path, "u/admin/script");
        assert!(!is_hub);

        let (type_str, path, is_hub) = reverse_transform("f-f_folder_flow").unwrap();
        assert_eq!(type_str, "flow");
        assert_eq!(path, "f/folder/flow");
        assert!(!is_hub);
    }

    #[test]
    fn test_extract_path_prefix_from_hashed() {
        // Generate a real hashed name and verify prefix extraction
        let long_path = "u/admin/engineering/team/automation/very_long_script";
        let hashed = transform_path(long_path, "script");
        assert!(is_hashed_name(&hashed));

        let prefix = extract_path_prefix_from_hashed(&hashed).unwrap();
        // The original path should start with the extracted prefix
        assert!(
            long_path.starts_with(&prefix),
            "path '{}' should start with prefix '{}'",
            long_path,
            prefix
        );
    }

    #[test]
    fn test_extract_path_prefix_underscore_in_path() {
        let long_path = "u/admin/my_team/automation/very_long_script_name_here";
        let hashed = transform_path(long_path, "script");
        let prefix = extract_path_prefix_from_hashed(&hashed).unwrap();
        assert!(
            long_path.starts_with(&prefix),
            "path '{}' should start with prefix '{}'",
            long_path,
            prefix
        );
    }

    #[test]
    fn test_reverse_transform_rejects_hashed_names() {
        assert!(reverse_transform("S-something").is_err());
        assert!(reverse_transform("F-something").is_err());
        assert!(reverse_transform("Hs-12345-something").is_err());
    }

    #[test]
    fn test_apply_key_transformation() {
        assert_eq!(apply_key_transformation("my key"), "my_key");
        assert_eq!(apply_key_transformation("key!@#"), "key");
        assert_eq!(apply_key_transformation("key_123"), "key_123");
    }
}
