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
/// For short names (≤40 chars): `s-{escaped_path}` or `f-{escaped_path}`
/// For long names (>40 chars): `S-{escaped[:22]}{sha256[:16]}` or `F-{escaped[:22]}{sha256[:16]}`
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
    // Layout: "{Upper}-" (2 chars) + prefix_body (22 chars) + hash (16 chars) = 40
    let prefix_body_len = MAX_PATH_LENGTH - 2 - HASH_LEN;
    let hash = calculate_hash(&short_name);
    let hash_suffix = &hash[..HASH_LEN];
    let truncated = truncate_to_char_boundary(&escaped_path, prefix_body_len);
    format!("{}-{}{}", upper_prefix, truncated, hash_suffix)
}

/// Transform the path for hub scripts
///
/// For short names (≤40 chars): `hs-{id}-{summary}`
/// For long names (>40 chars): `Hs-{id}-{summary[:N]}{sha256[:16]}`
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

/// Parse the prefix of any tool name (both short and hashed).
/// Returns `(type_str, is_hub, is_hashed)`.
/// Hashed names use an uppercase first character as the signal.
pub fn parse_tool_prefix(name: &str) -> Result<(&str, bool, bool), String> {
    let is_hashed = name.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false);
    let lower = name.to_ascii_lowercase();
    let (type_str, is_hub) = if lower.starts_with("hs-") {
        ("script", true)
    } else if lower.starts_with("s-") {
        ("script", false)
    } else if lower.starts_with("f-") {
        ("flow", false)
    } else {
        return Err(format!("Invalid tool name prefix: {}", name));
    };
    Ok((type_str, is_hub, is_hashed))
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

/// Extract a safe original-path prefix from a hashed tool name.
///
/// Given `S-u_admin_engineering__te<hash16>`, extracts the escaped prefix between
/// the type prefix (`S-`, `F-`, or `Hs-`) and the hash, un-escapes it, and
/// returns a prefix suitable for `WHERE path LIKE '{prefix}%'`.
///
/// Returns `None` if the name is too short or has an unrecognized prefix.
pub fn extract_path_prefix_from_hashed(name: &str) -> Option<String> {
    let prefix_len = if name.starts_with("Hs-") {
        3
    } else if name.starts_with("S-") || name.starts_with("F-") {
        2
    } else {
        return None;
    };
    if name.len() <= prefix_len + HASH_LEN {
        return None;
    }
    let escaped_prefix = &name[prefix_len..name.len() - HASH_LEN];
    if escaped_prefix.is_empty() {
        return None;
    }

    // Strip trailing underscores — they may be half of a `__` pair split by truncation
    let trimmed = escaped_prefix.trim_end_matches('_');
    if trimmed.is_empty() {
        return None;
    }

    Some(unescape_path(trimmed))
}

/// Un-escape a mangled path segment: `__` → `_`, standalone `_` → `/`.
fn unescape_path(s: &str) -> String {
    const TEMP_PLACEHOLDER: &str = "@@UNDERSCORE@@";
    s.replace("__", TEMP_PLACEHOLDER)
        .replace('_', "/")
        .replace(TEMP_PLACEHOLDER, "_")
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
/// resolved via `parse_tool_prefix` + path enumeration in the runner.
pub fn reverse_transform(transformed_path: &str) -> Result<(&str, String, bool), String> {
    let (type_str, is_hub, is_hashed) = parse_tool_prefix(transformed_path)?;

    if is_hashed {
        return Err(
            "Hashed names cannot be reverse-transformed directly; use path enumeration instead"
                .to_string(),
        );
    }

    // Strip the prefix: "hs-" (3 chars) for hub, "s-"/"f-" (2 chars) for others
    let prefix_len = if is_hub { 3 } else { 2 };
    let mangled_path = &transformed_path[prefix_len..];

    let original_path = if is_hub {
        let parts = mangled_path.split("-").collect::<Vec<&str>>();
        if parts.is_empty() {
            return Err(format!("Invalid transformed path: {}", transformed_path));
        }
        parts[0].to_string()
    } else {
        unescape_path(mangled_path)
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
        let (_, _, is_hashed) = parse_tool_prefix(&result).unwrap();
        assert!(is_hashed);
    }

    #[test]
    fn test_transform_path_long_flow_is_hashed() {
        let long_path = "f/engineering/team/automation/very_long_flow_name_that_exceeds_limit";
        let result = transform_path(long_path, "flow");
        assert_eq!(result.len(), MAX_PATH_LENGTH);
        assert!(result.starts_with("F-"));
        let (_, _, is_hashed) = parse_tool_prefix(&result).unwrap();
        assert!(is_hashed);
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
        let (_, _, is_hashed) = parse_tool_prefix(&result).unwrap();
        assert!(!is_hashed);
    }

    #[test]
    fn test_transform_hub_path_long_is_hashed() {
        let result = transform_hub_path(
            12345,
            "Send Slack Message To Channel With Very Long Description That Exceeds Limit",
        );
        assert_eq!(result.len(), MAX_PATH_LENGTH);
        assert!(result.starts_with("Hs-12345-"));
        let (_, _, is_hashed) = parse_tool_prefix(&result).unwrap();
        assert!(is_hashed);
    }

    #[test]
    fn test_extract_hub_version_id_from_hashed() {
        let name = "Hs-12345-Send_Slack_Message_To_Ch9e8d7c6b5a4f3e2d";
        let id = extract_hub_version_id_from_hashed(name).unwrap();
        assert_eq!(id, "12345");
    }

    #[test]
    fn test_parse_tool_prefix() {
        let (t, hub, hashed) = parse_tool_prefix("S-something").unwrap();
        assert_eq!(t, "script");
        assert!(!hub);
        assert!(hashed);

        let (t, hub, hashed) = parse_tool_prefix("F-something").unwrap();
        assert_eq!(t, "flow");
        assert!(!hub);
        assert!(hashed);

        let (t, hub, hashed) = parse_tool_prefix("Hs-12345-something").unwrap();
        assert_eq!(t, "script");
        assert!(hub);
        assert!(hashed);

        let (t, hub, hashed) = parse_tool_prefix("s-u_admin_script").unwrap();
        assert_eq!(t, "script");
        assert!(!hub);
        assert!(!hashed);

        let (t, hub, hashed) = parse_tool_prefix("f-f_folder_flow").unwrap();
        assert_eq!(t, "flow");
        assert!(!hub);
        assert!(!hashed);

        let (t, hub, hashed) = parse_tool_prefix("hs-12345-summary").unwrap();
        assert_eq!(t, "script");
        assert!(hub);
        assert!(!hashed);
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
        let (_, _, is_hashed) = parse_tool_prefix(&hashed).unwrap();
        assert!(is_hashed);

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
    fn test_extract_path_prefix_rejects_invalid_prefix() {
        assert!(extract_path_prefix_from_hashed("x-something").is_none());
        assert!(extract_path_prefix_from_hashed("").is_none());
        assert!(extract_path_prefix_from_hashed("S-").is_none());
    }

    #[test]
    fn test_extract_path_prefix_handles_hs_prefix() {
        // Hs- is 3 chars, not 2 — ensure the prefix is stripped correctly
        let hashed = transform_hub_path(12345, "a]very long hub script summary that exceeds the limit");
        let (_, is_hub, is_hashed) = parse_tool_prefix(&hashed).unwrap();
        assert!(is_hub);
        assert!(is_hashed);

        let prefix = extract_path_prefix_from_hashed(&hashed);
        // Should not start with 's' (leftover from Hs- if sliced at index 2)
        if let Some(ref p) = prefix {
            assert!(
                !p.starts_with('s'),
                "prefix '{}' should not start with 's' from mis-sliced Hs- prefix",
                p
            );
        }
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
