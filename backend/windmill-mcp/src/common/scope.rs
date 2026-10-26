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
    /// Whether this is a legacy "all" scope
    pub all: bool,
    /// Whether this is a "favorites" scope
    pub favorites: bool,
    /// Whether a granular scope is detected
    pub granular: bool,
    /// Hub app filter (if any)
    pub hub_apps: Option<String>,
    /// The workspaces a token without a bound workspace may act in
    /// (`mcp:workspaces:a,b`); `None` is every workspace its owner belongs to.
    pub workspaces: Option<Vec<String>>,
    /// `mcp:run` (legacy: `mcp:all` + `mcp:read_run`): reads, plus running deployed scripts
    /// and flows, and nothing else.
    pub read_run: bool,
    /// `mcp:read` (legacy: `mcp:all` + `mcp:read_only`): reads alone, as the token's read-only
    /// flag does. An OAuth grant carries scopes and no flag, so this is how a sign-in asks
    /// for read-only access.
    pub read_only: bool,
    /// No endpoint tools on any URL: set by the read and run access levels and
    /// `mcp:runnables_as_tools`, and by the server on `/api/mcp` for a selection.
    pub no_endpoints: bool,
}

impl McpScopeConfig {
    pub fn allows_workspace(&self, workspace_id: &str) -> bool {
        self.workspaces
            .as_ref()
            .is_none_or(|ws| ws.iter().any(|w| w == workspace_id))
    }

    /// Whether the token grants access to *any* concrete resource of this type by
    /// path. Used to decide whether to advertise the run-by-path tools in
    /// multi-workspace mode (a `mcp:scripts:*`-only token should see
    /// `runScriptByPath` even without an endpoint scope). `mcp:all` grants
    /// everything; `mcp:favorites` does NOT — favorites are an enumerated set the
    /// caller can only reach through the per-item tools, not by naming an
    /// arbitrary path, so it grants nothing here (mirrors `is_allowed`, which
    /// returns false for a favorites token).
    pub fn has_any(&self, resource_type: &str) -> bool {
        if self.all {
            return true;
        }
        match resource_type {
            "script" => !self.scripts.is_empty(),
            "flow" => !self.flows.is_empty(),
            "endpoint" => !self.endpoints.is_empty(),
            _ => false,
        }
    }

    /// Check if a resource is allowed based on its type and path
    pub fn is_allowed(&self, resource_type: &str, path: &str) -> bool {
        if self.all {
            return true;
        }

        let patterns = match resource_type {
            "script" => &self.scripts,
            "flow" => &self.flows,
            "endpoint" => &self.endpoints,
            _ => return false,
        };

        is_resource_allowed(path, patterns)
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
        if let Some(caller_ws) = self.workspaces.as_ref() {
            match requested.workspaces.as_ref() {
                Some(req_ws) if req_ws.iter().all(|w| caller_ws.contains(w)) => {}
                _ => return false,
            }
        }
        if self.read_only && !requested.read_only {
            return false;
        }
        if self.read_run && !(requested.read_run || requested.read_only) {
            return false;
        }
        if self.no_endpoints && !requested.no_endpoints {
            return false;
        }
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

/// Marks a selection made as "scripts and flows as tools": it gets no endpoint tools on any
/// URL, which an older selection, carrying none of the newer scopes, still does.
pub const RUNNABLES_AS_TOOLS_SCOPE: &str = "mcp:runnables_as_tools";

/// Added server-side to an `mcp:all` OAuth grant so the tools server's URLs serve it the AI
/// session's toolset. Grants nothing: an un-narrowed `mcp:all` token without it predates the
/// session toolset there, and must keep the tools server's endpoint tools.
pub const SESSION_SCOPE: &str = "mcp:session";

/// Parse MCP scopes from token scope strings
pub fn parse_mcp_scopes(scopes: &[String]) -> Result<McpScopeConfig, String> {
    let mut config = McpScopeConfig::default();
    let (mut read, mut run) = (false, false);

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
            config.scripts.extend(parse_resource_list(resources));
            continue;
        }

        if let Some(resources) = scope.strip_prefix("mcp:flows:") {
            // New granular flow scope: mcp:flows:path1,path2,f/folder/*
            config.flows.extend(parse_resource_list(resources));
            continue;
        }

        if scope == "mcp:read" {
            read = true;
            continue;
        }

        if scope == "mcp:run" {
            run = true;
            continue;
        }

        // `mcp:read_run` and `mcp:read_only` narrow the `mcp:all` beside them: the access
        // levels before `mcp:read` / `mcp:run`, still carried by tokens issued then.
        if scope == "mcp:read_run" {
            config.read_run = true;
            config.no_endpoints = true;
            continue;
        }

        if scope == "mcp:read_only" {
            config.read_only = true;
            config.no_endpoints = true;
            continue;
        }

        if scope == RUNNABLES_AS_TOOLS_SCOPE {
            config.no_endpoints = true;
            continue;
        }

        if scope == SESSION_SCOPE {
            continue;
        }

        if let Some(ids) = scope.strip_prefix("mcp:workspaces:") {
            config
                .workspaces
                .get_or_insert_with(Vec::new)
                .extend(parse_resource_list(ids));
            continue;
        }

        if let Some(resources) = scope.strip_prefix("mcp:endpoints:") {
            // New granular endpoint scope: mcp:endpoints:name1,name2
            config.endpoints.extend(parse_resource_list(resources));
            continue;
        }

        tracing::warn!("Unrecognized MCP scope format: {}", scope);
    }

    // The access levels rank below `mcp:all`, so beside it they add nothing, and `mcp:run`
    // covers `mcp:read`: a client requesting every level gets the highest one.
    if (read || run) && !config.all {
        config.all = true;
        config.scripts.push("*".to_string());
        config.flows.push("*".to_string());
        config.endpoints.push("*".to_string());
        config.no_endpoints = true;
        if run {
            config.read_run = true;
        } else {
            config.read_only = true;
        }
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
    fn test_has_any() {
        // mcp:all grants everything by path.
        assert!(cfg(&["mcp:all"]).has_any("script"));

        // mcp:favorites grants NO arbitrary-path access (favorites are reached
        // via per-item tools, not by naming a path) — matches is_allowed.
        let fav = cfg(&["mcp:favorites"]);
        assert!(!fav.has_any("script"));
        assert!(!fav.has_any("flow"));
        assert!(!fav.is_allowed("script", "f/anything/x"));

        // Granular: only the resource types with at least one pattern.
        let scripts_only = cfg(&["mcp:scripts:f/team/*"]);
        assert!(scripts_only.has_any("script"));
        assert!(!scripts_only.has_any("flow"));
        assert!(!scripts_only.has_any("endpoint"));

        let endpoints_only = cfg(&["mcp:endpoints:runScriptByPath"]);
        assert!(!endpoints_only.has_any("script"));
        assert!(endpoints_only.has_any("endpoint"));
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

    #[test]
    fn test_workspaces_scope() {
        let two = cfg(&["mcp:all", "mcp:workspaces:dev,staging"]);
        assert!(two.allows_workspace("dev") && !two.allows_workspace("prod"));
        assert!(cfg(&["mcp:all"]).allows_workspace("prod"));
        // A workspace list only narrows: it cannot be dropped or widened by a grant.
        assert!(two.contains(&cfg(&["mcp:all", "mcp:workspaces:dev"])));
        assert!(!two.contains(&cfg(&["mcp:all", "mcp:workspaces:dev,prod"])));
        assert!(!two.contains(&cfg(&["mcp:all"])));
        assert!(cfg(&["mcp:all"]).contains(&two));
    }

    #[test]
    fn test_read_run_scope_only_narrows() {
        let read_run = cfg(&["mcp:all", "mcp:read_run"]);
        assert!(read_run.read_run && read_run.all);
        assert!(read_run.contains(&cfg(&["mcp:all", "mcp:read_run"])));
        assert!(!read_run.contains(&cfg(&["mcp:all"])));
        assert!(read_run.contains(&cfg(&["mcp:all", "mcp:read_only"])));
        let read_only = cfg(&["mcp:all", "mcp:read_only"]);
        assert!(!read_only.contains(&cfg(&["mcp:all", "mcp:read_run"])));
    }

    #[test]
    fn test_access_levels_rank_and_match_legacy_scopes() {
        let level = |scopes: &[&str]| {
            let c = cfg(scopes);
            (c.all, c.read_only, c.read_run, c.no_endpoints)
        };
        assert_eq!(level(&["mcp:read"]), level(&["mcp:all", "mcp:read_only"]));
        assert_eq!(level(&["mcp:run"]), level(&["mcp:all", "mcp:read_run"]));
        assert_eq!(level(&["mcp:read", "mcp:run"]), level(&["mcp:run"]));
        assert_eq!(
            level(&["mcp:all", "mcp:read", "mcp:run"]),
            level(&["mcp:all"])
        );
        assert!(cfg(&["mcp:run"]).contains(&cfg(&["mcp:read"])));
        assert!(!cfg(&["mcp:read"]).contains(&cfg(&["mcp:run"])));
        assert!(cfg(&["mcp:all", "mcp:read_run"]).contains(&cfg(&["mcp:run"])));
    }

    #[test]
    fn test_runnables_as_tools_scope_only_narrows() {
        let fenced = cfg(&["mcp:favorites", "mcp:runnables_as_tools"]);
        assert!(fenced.no_endpoints);
        assert!(cfg(&["mcp:all", "mcp:read_only"]).no_endpoints);
        assert!(cfg(&["mcp:all", "mcp:read_run"]).no_endpoints);
        assert!(fenced.contains(&cfg(&["mcp:favorites", "mcp:runnables_as_tools"])));
        assert!(!fenced.contains(&cfg(&["mcp:favorites"])));
        assert!(cfg(&["mcp:all"]).contains(&fenced));
    }
}
