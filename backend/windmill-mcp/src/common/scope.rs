//! MCP Scope matching utilities
//!
//! Contains utilities for parsing and matching MCP token scopes to determine
//! which scripts, flows, and endpoints a token has access to.

/// Configuration for MCP scopes parsed from token scopes
#[derive(Debug, Clone, Default)]
pub struct McpScopeConfig {
    /// Script paths/patterns allowed by this token
    pub scripts: Vec<String>,
    /// Flow paths/patterns allowed by this token
    pub flows: Vec<String>,
    /// Endpoint names/patterns allowed by this token
    pub endpoints: Vec<String>,
    /// Datatable names/patterns this token may read/write. Only meaningful when
    /// `datatables_restricted` is set; otherwise datatable access is unrestricted.
    pub datatables: Vec<String>,
    /// Whether any `mcp:datatables:` scope was present. Absent = opt-out (all
    /// datatables allowed, backward compatible); present = restrict to the list.
    pub datatables_restricted: bool,
    /// Whether this is a legacy "all" scope
    pub all: bool,
    /// Whether this is a "favorites" scope
    pub favorites: bool,
    /// Whether a granular scope is detected
    pub granular: bool,
    /// Hub app filter (if any)
    pub hub_apps: Option<String>,
}

impl McpScopeConfig {
    /// Check if a resource is allowed based on its type and path
    pub fn is_allowed(&self, resource_type: &str, path: &str) -> bool {
        if self.all {
            return true;
        }

        let patterns = match resource_type {
            "script" => &self.scripts,
            "flow" => &self.flows,
            "endpoint" => &self.endpoints,
            "datatable" => &self.datatables,
            _ => return false,
        };

        is_resource_allowed(path, patterns)
    }

    /// Whether the token may touch datatable `name`. Datatable filtering is
    /// opt-in: a token with no `mcp:datatables:` scope (`datatables_restricted`
    /// false) may reach every datatable, so existing tokens keep working. Once a
    /// restriction is present, only the listed names/patterns (or `*`) pass.
    pub fn is_datatable_allowed(&self, name: &str) -> bool {
        if self.all || !self.datatables_restricted {
            return true;
        }
        is_resource_allowed(name, &self.datatables)
    }

    /// Directional subset check: does this config grant at least everything
    /// `requested` grants? Used to enforce monotonic containment when an MCP
    /// OAuth approval mints a token (the granted scopes must be within the
    /// approving token's own scopes).
    ///
    /// Unlike `is_allowed` (which tests a single concrete path with OR
    /// semantics), this requires every requested pattern to be covered by some
    /// caller pattern — so `mcp:scripts:f/x` cannot widen into `mcp:scripts:*`.
    pub fn contains(&self, requested: &McpScopeConfig) -> bool {
        if self.all {
            return true;
        }
        if requested.all {
            return false;
        }
        if requested.favorites && !self.favorites {
            return false;
        }
        if let Some(req_hub) = requested.hub_apps.as_ref() {
            match self.hub_apps.as_ref() {
                Some(caller_hub) => {
                    let caller_apps: std::collections::HashSet<&str> =
                        caller_hub.split(',').map(|s| s.trim()).collect();
                    if !req_hub
                        .split(',')
                        .map(|s| s.trim())
                        .all(|a| caller_apps.contains(a))
                    {
                        return false;
                    }
                }
                None => return false,
            }
        }
        // Datatable containment: an unrestricted request grants every datatable,
        // so a restricted caller (without `*`) cannot cover it. A restricted
        // request must be covered by the caller's list — treating an unrestricted
        // caller as `*`.
        if !requested.datatables_restricted {
            if self.datatables_restricted && !self.datatables.iter().any(|d| d == "*") {
                return false;
            }
        } else {
            let caller = if self.datatables_restricted {
                self.datatables.clone()
            } else {
                vec!["*".to_string()]
            };
            if !resource_list_covers(&caller, &requested.datatables) {
                return false;
            }
        }

        resource_list_covers(&self.scripts, &requested.scripts)
            && resource_list_covers(&self.flows, &requested.flows)
            && resource_list_covers(&self.endpoints, &requested.endpoints)
    }
}

/// Every requested pattern must be covered by some caller pattern.
fn resource_list_covers(caller: &[String], requested: &[String]) -> bool {
    requested
        .iter()
        .all(|req| caller.iter().any(|c| pattern_covers(c, req)))
}

/// Directional: does the single caller pattern cover `requested`? `caller` may
/// be `*`, an exact path/name, or a `<prefix>/*` subtree; `requested` may itself
/// be a subtree wildcard, in which case the whole requested subtree must fall
/// within the caller's. Mirrors the route-scope containment in windmill-api-auth.
fn pattern_covers(caller: &str, requested: &str) -> bool {
    if caller == "*" || caller == requested {
        return true;
    }
    // An exact caller pattern only covers itself (handled above); a wildcard
    // requested can never be covered by a non-`*` exact caller.
    let Some(prefix) = caller.strip_suffix("/*") else {
        return false;
    };
    let requested_base = requested.strip_suffix("/*").unwrap_or(requested);
    requested_base == prefix
        || (requested_base.starts_with(prefix)
            && requested_base.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// Parse MCP scopes from token scope strings
pub fn parse_mcp_scopes(scopes: &[String]) -> Result<McpScopeConfig, String> {
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
            config.datatables.push("*".to_string());
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
            config.scripts.extend(parse_resource_list(resources));
            continue;
        }

        if let Some(resources) = scope.strip_prefix("mcp:flows:") {
            // New granular flow scope: mcp:flows:path1,path2,f/folder/*
            config.flows.extend(parse_resource_list(resources));
            continue;
        }

        if let Some(resources) = scope.strip_prefix("mcp:endpoints:") {
            // New granular endpoint scope: mcp:endpoints:name1,name2
            config.endpoints.extend(parse_resource_list(resources));
            continue;
        }

        if let Some(resources) = scope.strip_prefix("mcp:datatables:") {
            // Datatable restriction: mcp:datatables:name1,name2 or mcp:datatables:*
            config.datatables_restricted = true;
            config.datatables.extend(parse_resource_list(resources));
            continue;
        }

        tracing::warn!("Unrecognized MCP scope format: {}", scope);
    }

    config.granular = !config.all && !config.favorites;

    Ok(config)
}

/// Parse comma-separated resource list
fn parse_resource_list(resources: &str) -> Vec<String> {
    if resources.is_empty() {
        return vec![];
    }

    resources
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
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

    #[test]
    fn test_scope_config_is_allowed() {
        let mut config = McpScopeConfig::default();
        config.scripts.push("u/admin/*".to_string());
        config.flows.push("f/automation/*".to_string());

        assert!(config.is_allowed("script", "u/admin/test"));
        assert!(!config.is_allowed("script", "u/other/test"));
        assert!(config.is_allowed("flow", "f/automation/test"));
        assert!(!config.is_allowed("flow", "f/other/test"));
    }

    fn cfg(scopes: &[&str]) -> McpScopeConfig {
        parse_mcp_scopes(&scopes.iter().map(|s| s.to_string()).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn test_contains_subset_and_widening() {
        // mcp:all contains anything.
        assert!(cfg(&["mcp:all"]).contains(&cfg(&["mcp:scripts:f/x"])));
        assert!(cfg(&["mcp:all"]).contains(&cfg(&["mcp:all"])));

        // A wildcard caller covers narrower requests, but not other domains/all.
        let star = cfg(&["mcp:scripts:*"]);
        assert!(star.contains(&cfg(&["mcp:scripts:f/x"])));
        assert!(star.contains(&cfg(&["mcp:scripts:*"])));
        assert!(!star.contains(&cfg(&["mcp:all"])));
        assert!(!star.contains(&cfg(&["mcp:flows:f/x"])));

        // The core regression: a single-path caller must NOT widen into `*` or
        // into another path.
        let narrow = cfg(&["mcp:scripts:f/x"]);
        assert!(narrow.contains(&cfg(&["mcp:scripts:f/x"])));
        assert!(!narrow.contains(&cfg(&["mcp:scripts:*"])));
        assert!(!narrow.contains(&cfg(&["mcp:scripts:f/y"])));
        assert!(!narrow.contains(&cfg(&["mcp:all"])));

        // Subtree wildcard covers paths within it but not a sibling subtree.
        let subtree = cfg(&["mcp:scripts:f/team/*"]);
        assert!(subtree.contains(&cfg(&["mcp:scripts:f/team/sub"])));
        assert!(subtree.contains(&cfg(&["mcp:scripts:f/team/sub/*"])));
        assert!(!subtree.contains(&cfg(&["mcp:scripts:f/other/x"])));
    }

    #[test]
    fn test_datatable_scope_parsing_and_allow() {
        // No datatable scope => unrestricted (opt-in), every datatable allowed.
        let c = cfg(&["mcp:endpoints:queryDataTable"]);
        assert!(!c.datatables_restricted);
        assert!(c.is_datatable_allowed("main"));
        assert!(c.is_datatable_allowed("anything"));

        // mcp:all grants all datatables.
        assert!(cfg(&["mcp:all"]).is_datatable_allowed("main"));

        // Explicit restriction to specific names.
        let c = cfg(&["mcp:datatables:main,analytics"]);
        assert!(c.datatables_restricted);
        assert!(c.is_datatable_allowed("main"));
        assert!(c.is_datatable_allowed("analytics"));
        assert!(!c.is_datatable_allowed("secret"));

        // Wildcard restriction allows all.
        let c = cfg(&["mcp:datatables:*"]);
        assert!(c.datatables_restricted);
        assert!(c.is_datatable_allowed("anything"));
    }

    #[test]
    fn test_contains_datatables() {
        // Unrestricted caller covers a restricted request.
        assert!(cfg(&["mcp:endpoints:*"]).contains(&cfg(&["mcp:datatables:main"])));
        // Restricted caller covers a subset request.
        assert!(cfg(&["mcp:datatables:main,analytics"]).contains(&cfg(&["mcp:datatables:main"])));
        // Restricted caller must NOT widen into a sibling or into unrestricted.
        assert!(!cfg(&["mcp:datatables:main"]).contains(&cfg(&["mcp:datatables:analytics"])));
        assert!(!cfg(&["mcp:datatables:main"]).contains(&cfg(&["mcp:endpoints:queryDataTable"])));
        // `*` restriction covers both a subset and an unrestricted request.
        assert!(cfg(&["mcp:datatables:*"]).contains(&cfg(&["mcp:datatables:main"])));
        assert!(cfg(&["mcp:datatables:*"]).contains(&cfg(&[])));
        // mcp:all covers any datatable request.
        assert!(cfg(&["mcp:all"]).contains(&cfg(&["mcp:datatables:main"])));
    }

    #[test]
    fn test_contains_favorites_and_endpoints() {
        assert!(cfg(&["mcp:favorites"]).contains(&cfg(&["mcp:favorites"])));
        // A caller without favorites cannot grant favorites.
        assert!(!cfg(&["mcp:scripts:*"]).contains(&cfg(&["mcp:favorites"])));

        // Endpoint names match exactly (or via `*`).
        let ep = cfg(&["mcp:endpoints:getVariable"]);
        assert!(ep.contains(&cfg(&["mcp:endpoints:getVariable"])));
        assert!(!ep.contains(&cfg(&["mcp:endpoints:getResource"])));
        assert!(!ep.contains(&cfg(&["mcp:all"])));
        // mcp:all grants all endpoints.
        assert!(cfg(&["mcp:all"]).contains(&cfg(&["mcp:endpoints:getResource"])));
    }
}
