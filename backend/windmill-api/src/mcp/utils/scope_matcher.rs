//! MCP Scope matching utilities
//!
//! Contains utilities for parsing and matching MCP token scopes to determine
//! which scripts, flows, and endpoints a token has access to.

use rmcp::ErrorData;

/// Configuration for MCP scopes parsed from token scopes
#[derive(Debug, Clone, Default)]
pub struct McpScopeConfig {
    /// Script paths/patterns allowed by this token
    pub scripts: Vec<String>,
    /// Flow paths/patterns allowed by this token
    pub flows: Vec<String>,
    /// Endpoint names/patterns allowed by this token
    pub endpoints: Vec<String>,
    /// Whether this is a legacy "all" scope
    pub all: bool,
    /// Whether this is a "favorites" scope
    pub favorites: bool,
    /// Whether a granular scope is detected
    pub granular: bool,
    /// Hub app filter (if any)
    pub hub_apps: Option<String>,
}

/// Parse MCP scopes from token scope strings
pub fn parse_mcp_scopes(scopes: &[String]) -> Result<McpScopeConfig, ErrorData> {
    let mut config = McpScopeConfig::default();

    for scope in scopes {
        if !scope.starts_with("mcp:") {
            continue;
        }

        if scope == "mcp:all" {
            // Legacy scope: grant access to everything
            config.all = true;
            config.scripts.push("*".to_string());
            config.flows.push("*".to_string());
            config.endpoints.push("*".to_string());
            continue;
        }

        if scope == "mcp:favorites" {
            // Legacy favorites scope
            config.favorites = true;
            continue;
        }

        // Legacy folder scope: mcp:all:f/folder/*
        if scope.starts_with("mcp:all:") {
            if let Some(folder_pattern) = scope.strip_prefix("mcp:all:") {
                // Parse as folder pattern - add to both scripts and flows. Also add all endpoints.
                config.scripts.push(folder_pattern.to_string());
                config.flows.push(folder_pattern.to_string());
                config.endpoints.push("*".to_string());
            }
            continue;
        }

        if scope.starts_with("mcp:hub:") {
            // Legacy hub scope
            if let Some(apps) = scope.strip_prefix("mcp:hub:") {
                config.hub_apps = Some(apps.to_string());
            }
            continue;
        }

        if let Some(resources) = scope.strip_prefix("mcp:scripts:") {
            // New granular script scope: mcp:scripts:path1,path2,f/folder/*
            config.scripts.extend(parse_resource_list(resources)?);
            continue;
        }

        if let Some(resources) = scope.strip_prefix("mcp:flows:") {
            // New granular flow scope: mcp:flows:path1,path2,f/folder/*
            config.flows.extend(parse_resource_list(resources)?);
            continue;
        }

        if let Some(resources) = scope.strip_prefix("mcp:endpoints:") {
            // New granular endpoint scope: mcp:endpoints:name1,name2
            config.endpoints.extend(parse_resource_list(resources)?);
            continue;
        }

        tracing::warn!("Unrecognized MCP scope format: {}", scope);
    }

    config.granular = !config.all && !config.favorites;

    Ok(config)
}

/// Parse comma-separated resource list
fn parse_resource_list(resources: &str) -> Result<Vec<String>, ErrorData> {
    if resources.is_empty() {
        return Ok(vec![]);
    }

    Ok(resources
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Check if a resource path matches any pattern in the allowed list
pub fn is_resource_allowed(resource_path: &str, allowed_patterns: &[String]) -> bool {
    if allowed_patterns.is_empty() {
        return false;
    }

    // Wildcard grants all access
    if allowed_patterns.contains(&"*".to_string()) {
        return true;
    }

    // Check against each pattern
    for pattern in allowed_patterns {
        if resource_matches_pattern(resource_path, pattern) {
            return true;
        }
    }

    false
}

/// Check if a resource path matches a pattern (supports wildcards like f/folder/*)
fn resource_matches_pattern(resource_path: &str, pattern: &str) -> bool {
    // Exact match
    if pattern == resource_path {
        return true;
    }

    // Wildcard pattern matching
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];

        if !resource_path.starts_with(prefix) {
            return false;
        }

        // If the resource is exactly the prefix, it matches
        if resource_path.len() == prefix.len() {
            return true;
        }

        // If the resource is longer, the next character must be '/' for a valid match
        // This prevents "u/user" from matching "u/use/*"
        return resource_path.chars().nth(prefix.len()) == Some('/');
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_legacy_scopes() {
        let scopes = vec!["mcp:all".to_string()];
        let config = parse_mcp_scopes(&scopes).unwrap();
        assert!(config.all);
        assert_eq!(config.scripts, vec!["*"]);
        assert_eq!(config.flows, vec!["*"]);
        assert_eq!(config.endpoints, vec!["*"]);

        let scopes = vec!["mcp:favorites".to_string()];
        let config = parse_mcp_scopes(&scopes).unwrap();
        assert!(config.favorites);

        let scopes = vec!["mcp:hub:slack".to_string()];
        let config = parse_mcp_scopes(&scopes).unwrap();
        assert_eq!(config.hub_apps, Some("slack".to_string()));
    }

    #[test]
    fn test_parse_granular_scopes() {
        let scopes = vec![
            "mcp:scripts:u/admin/script1,u/admin/script2".to_string(),
            "mcp:flows:f/automation/*".to_string(),
            "mcp:endpoints:list_jobs,get_job".to_string(),
        ];
        let config = parse_mcp_scopes(&scopes).unwrap();

        assert_eq!(config.scripts, vec!["u/admin/script1", "u/admin/script2"]);
        assert_eq!(config.flows, vec!["f/automation/*"]);
        assert_eq!(config.endpoints, vec!["list_jobs", "get_job"]);
    }

    #[test]
    fn test_resource_matching() {
        // Exact match
        assert!(resource_matches_pattern("u/admin/script", "u/admin/script"));

        // Wildcard folder match
        assert!(resource_matches_pattern("f/folder/script", "f/folder/*"));
        assert!(resource_matches_pattern(
            "f/folder/sub/script",
            "f/folder/*"
        ));

        // Should NOT match - prefix is not complete
        assert!(!resource_matches_pattern("u/username", "u/user/*"));

        // Should match - exact prefix
        assert!(resource_matches_pattern("u/user/script", "u/user/*"));
    }

    #[test]
    fn test_is_resource_allowed() {
        let patterns = vec!["u/admin/script1".to_string(), "f/folder/*".to_string()];

        assert!(is_resource_allowed("u/admin/script1", &patterns));
        assert!(is_resource_allowed("f/folder/anything", &patterns));
        assert!(!is_resource_allowed("u/other/script", &patterns));

        // Test wildcard
        let wildcard = vec!["*".to_string()];
        assert!(is_resource_allowed("any/path", &wildcard));

        // Test empty patterns
        let empty: Vec<String> = vec![];
        assert!(!is_resource_allowed("any/path", &empty));
    }
}
