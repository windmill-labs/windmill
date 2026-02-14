/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use windmill_common::error::{Error, Result};

/// Comprehensive scope system for JWT token authorization
///
/// Scopes follow the format: {domain}:{action}[:{resource}]
/// Examples:
/// - "jobs:read" - Read access to jobs
/// - "scripts:write:f/folder/*" - Write access to scripts in a folder
/// - "*" - Full access (superuser)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeDefinition {
    pub domain: String,
    pub action: String,
    pub kind: Option<String>, // For jobs:run:kind (optional)
    pub resource: Option<Vec<String>>,
}

impl ScopeDefinition {
    pub fn new(
        domain: &str,
        action: &str,
        kind: Option<&str>,
        resource: Option<Vec<String>>,
    ) -> Self {
        Self {
            domain: domain.to_string(),
            action: action.to_string(),
            kind: kind.map(|s| s.to_string()),
            resource: resource,
        }
    }

    pub fn from_scope_string(scope: &str) -> Result<Self> {
        let parts: Vec<&str> = scope.split(':').collect();

        let into_owned_vec = |resources: &str| -> Vec<String> {
            let resources = resources
                .split(",")
                .collect_vec()
                .into_iter()
                .map(ToOwned::to_owned)
                .collect_vec();

            resources
        };

        match parts.len() {
            2 => Ok(Self::new(parts[0], parts[1], None, None)), // domain:action
            3 => {
                if parts[0] == "jobs" && parts[1] == "run" {
                    Ok(Self::new(parts[0], parts[1], Some(parts[2]), None))
                } else {
                    Ok(Self::new(
                        parts[0],
                        parts[1],
                        None,
                        Some(into_owned_vec(parts[2])),
                    ))
                }
            }
            4 => {
                if parts[0] == "jobs" && parts[1] == "run" {
                    Ok(Self::new(
                        parts[0],
                        parts[1],
                        Some(parts[2]),
                        Some(into_owned_vec(parts[3])),
                    ))
                } else {
                    Err(Error::BadRequest(format!(
                        "Invalid 4-part scope: {}",
                        scope
                    )))
                }
            }
            _ => Err(Error::BadRequest(format!(
                "Invalid scope format: {}",
                scope
            ))),
        }
    }

    pub fn as_string(&self) -> String {
        match (&self.kind, &self.resource) {
            (Some(kind), Some(resource)) => {
                format!(
                    "{}:{}:{}:{}",
                    self.domain,
                    self.action,
                    kind,
                    resource.join(",")
                )
            }
            (Some(kind), None) => {
                format!("{}:{}:{}", self.domain, self.action, kind)
            }
            (None, Some(resource)) => {
                format!("{}:{}:{}", self.domain, self.action, resource.join(","))
            }
            (None, None) => format!("{}:{}", self.domain, self.action),
        }
    }

    pub fn includes(&self, other: &ScopeDefinition) -> bool {
        if self.domain != other.domain {
            return false;
        }

        match (self.action.as_str(), other.action.as_str()) {
            (a, b) if (a == "write" && b == "read") || (a == b) => {}
            _ => return false,
        }

        if self.domain == "jobs" && self.action == "run" {
            match (&self.kind, &other.kind) {
                (Some(self_kind), Some(other_kind)) => {
                    if self_kind != other_kind {
                        return false;
                    }
                }
                (Some(_), None) => {
                    return false;
                }
                (None, _) => {
                    return true;
                }
            }
        }

        match (&self.resource, &other.resource) {
            (Some(self_resources), Some(other_resources)) => {
                resources_match(self_resources, other_resources)
            }
            (Some(_), None) => false,
            (None, _) => true,
        }
    }
}

fn resources_match(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    if scope_resources.contains(&"*".to_string()) || accepted_resources.contains(&"*".to_string()) {
        return true;
    }

    if scope_resources.len() <= 4 && accepted_resources.len() <= 4 {
        return resources_match_small(scope_resources, accepted_resources);
    }

    resources_match_large(scope_resources, accepted_resources)
}

fn resources_match_small(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    for required in accepted_resources {
        for scope_resource in scope_resources {
            if resource_matches_pattern(scope_resource, required) {
                return true;
            }
        }
    }
    false
}

fn resources_match_large(scope_resources: &[String], accepted_resources: &[String]) -> bool {
    let mut exact_matches = HashSet::new();
    let mut patterns = Vec::new();

    for scope_resource in scope_resources {
        if scope_resource.contains('*') {
            patterns.push(scope_resource);
        } else {
            exact_matches.insert(scope_resource);
        }
    }

    for accepted_resource in accepted_resources {
        if exact_matches.contains(accepted_resource) {
            return true;
        }

        for pattern in &patterns {
            if resource_matches_pattern(pattern, accepted_resource) {
                return true;
            }
        }
    }

    false
}

fn resource_matches_pattern(scope_resource: &str, accepted_resource: &str) -> bool {
    if scope_resource == accepted_resource {
        return true;
    }

    let matches_wildcard = |pattern: &str, resource: &str| -> bool {
        if !pattern.ends_with("/*") {
            return false;
        }

        let prefix = &pattern[..pattern.len() - 2];

        if !resource.starts_with(prefix) {
            return false;
        }

        // If the resource is exactly the prefix, it matches
        if resource.len() == prefix.len() {
            return true;
        }

        // If the resource is longer, the next character must be '/' for a valid match
        // This prevents "u/user" from matching "u/use/*"
        resource.chars().nth(prefix.len()) == Some('/')
    };

    // Check if either resource is a wildcard pattern and matches the other
    matches_wildcard(scope_resource, accepted_resource)
        || matches_wildcard(accepted_resource, scope_resource)
}

// ─────────────────────────────────────────────────────────────────
// Route-level scope checking
// ─────────────────────────────────────────────────────────────────

/// Available scope domains (top-level API categories)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeDomain {
    // Core resource domains
    Jobs,
    Scripts,
    Flows,
    Apps,
    Variables,
    Resources,
    Schedules,
    Folders,
    Users,
    Groups,
    Workspaces,

    // Trigger domains
    HttpTriggers,
    WebsocketTriggers,
    KafkaTriggers,
    NatsTriggers,
    MqttTriggers,
    SqsTriggers,
    GcpTriggers,
    PostgresTriggers,
    EmailTriggers,

    // Native trigger domains
    NativeTriggers,

    // System domains
    Audit,
    Settings,
    Workers,
    ServiceLogs,
    Configs,
    OAuth,
    AI,

    Indexer,
    Teams,   // Microsoft Teams integration
    GitSync, // Git synchronization

    // Special domains
    Capture,           // Webhook capture
    Drafts,            // Draft resources
    Favorites,         // User favorites
    Inputs,            // Input templates
    JobHelpers,        // Job helper functions
    ConcurrencyGroups, // Concurrency groups
    Oidc,              // OpenID Connect
    Openapi,           // OpenAPI generation

    // Additional domains
    Acls,         // Granular access control lists
    RawApps,      // Raw application data
    AgentWorkers, // Agent workers management
    Mcp,          // MCP
}

impl ScopeDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Jobs => "jobs",
            Self::Scripts => "scripts",
            Self::Flows => "flows",
            Self::Apps => "apps",
            Self::Variables => "variables",
            Self::Resources => "resources",
            Self::Schedules => "schedules",
            Self::Folders => "folders",
            Self::Users => "users",
            Self::Groups => "groups",
            Self::Workspaces => "workspaces",
            Self::HttpTriggers => "http_triggers",
            Self::WebsocketTriggers => "websocket_triggers",
            Self::KafkaTriggers => "kafka_triggers",
            Self::NatsTriggers => "nats_triggers",
            Self::MqttTriggers => "mqtt_triggers",
            Self::SqsTriggers => "sqs_triggers",
            Self::GcpTriggers => "gcp_triggers",
            Self::PostgresTriggers => "postgres_triggers",
            Self::EmailTriggers => "email_triggers",
            Self::NativeTriggers => "native_triggers",
            Self::Audit => "audit",
            Self::Settings => "settings",
            Self::Workers => "workers",
            Self::ServiceLogs => "service_logs",
            Self::Configs => "configs",
            Self::OAuth => "oauth",
            Self::AI => "ai",
            Self::Capture => "capture",
            Self::Drafts => "drafts",
            Self::Favorites => "favorites",
            Self::Inputs => "inputs",
            Self::JobHelpers => "job_helpers",
            Self::ConcurrencyGroups => "concurrency_groups",
            Self::Oidc => "oidc",
            Self::Openapi => "openapi",
            Self::Acls => "acls",
            Self::RawApps => "raw_apps",
            Self::AgentWorkers => "agent_workers",
            Self::Indexer => "indexer",
            Self::Teams => "teams",
            Self::GitSync => "git_sync",
            Self::Mcp => "mcp",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "jobs" | "jobs_u" => Some(Self::Jobs),
            "scripts" => Some(Self::Scripts),
            "flows" => Some(Self::Flows),
            "apps" | "apps_u" => Some(Self::Apps),
            "variables" => Some(Self::Variables),
            "resources" => Some(Self::Resources),
            "schedules" => Some(Self::Schedules),
            "folders" => Some(Self::Folders),
            "users" => Some(Self::Users),
            "groups" => Some(Self::Groups),
            "workspaces" => Some(Self::Workspaces),
            "http_triggers" => Some(Self::HttpTriggers),
            "websocket_triggers" => Some(Self::WebsocketTriggers),
            "kafka_triggers" => Some(Self::KafkaTriggers),
            "nats_triggers" => Some(Self::NatsTriggers),
            "mqtt_triggers" => Some(Self::MqttTriggers),
            "sqs_triggers" => Some(Self::SqsTriggers),
            "gcp_triggers" => Some(Self::GcpTriggers),
            "postgres_triggers" => Some(Self::PostgresTriggers),
            "email_triggers" => Some(Self::EmailTriggers),
            "audit" => Some(Self::Audit),
            "settings" => Some(Self::Settings),
            "workers" => Some(Self::Workers),
            "service_logs" => Some(Self::ServiceLogs),
            "configs" => Some(Self::Configs),
            "oauth" => Some(Self::OAuth),
            "ai" => Some(Self::AI),
            "indexer" | "srch" => Some(Self::Indexer),
            "teams" => Some(Self::Teams),
            "native_triggers" => Some(Self::NativeTriggers),
            "git_sync" | "github_app" => Some(Self::GitSync),
            "capture" => Some(Self::Capture),
            "drafts" => Some(Self::Drafts),
            "favorites" => Some(Self::Favorites),
            "inputs" => Some(Self::Inputs),
            "job_helpers" => Some(Self::JobHelpers),
            "concurrency_groups" => Some(Self::ConcurrencyGroups),
            "oidc" => Some(Self::Oidc),
            "openapi" => Some(Self::Openapi),
            "acls" => Some(Self::Acls),
            "raw_apps" => Some(Self::RawApps),
            "agent_workers" => Some(Self::AgentWorkers),
            "mcp" => Some(Self::Mcp),
            _ => None,
        }
    }
}

/// Available scope actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeAction {
    Read,  // GET operations, list, view
    Write, // POST, PUT, PATCH, DELETE operations, create, update, delete
    Run,   // Special action for running (scripts, flows, etc.)
}

impl ScopeAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Run => "run",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "delete" => Some(Self::Write),
            "run" => Some(Self::Run),
            _ => None,
        }
    }

    /// Check if this action includes another action
    /// Write includes Read
    pub fn includes(&self, other: &ScopeAction) -> bool {
        match (self, other) {
            (ScopeAction::Write, ScopeAction::Read) => true,
            (ScopeAction::Run, ScopeAction::Read) => true,
            (a, b) => a == b,
        }
    }
}

pub fn check_route_access(
    token_scopes: &[String],
    route_path: &str,
    http_method: &str,
) -> Result<()> {
    // Map HTTP method to scope action (considering route context)
    let required_action = map_http_method_to_action(http_method, route_path);

    // Find the domain and kind for this route
    let (required_domain, required_kind, route_suffix) = extract_domain_from_route(route_path)?;

    // Backward compatibility: MCP handlers expect unusual scope actions: all, favorites, hub.
    if required_domain == ScopeDomain::Mcp {
        return Ok(());
    }

    // tracing::error!("Checking route access {:?} {:?} {:?} {:?}", required_action, required_domain, required_kind, route_suffix);
    let mut is_scoped_token = false;
    // Check if any token scope grants the required access
    for scope_str in token_scopes {
        if !scope_str.starts_with("if_jobs:filter_tags:") {
            if let Ok(scope) = ScopeDefinition::from_scope_string(scope_str) {
                // tracing::error!("Checking scope {:?} for required domain {:?} and action {:?} and kind {:?} and route suffix {:?}", scope, required_domain, required_action, required_kind, route_suffix);
                if scope_grants_access(
                    &scope,
                    required_domain,
                    required_action,
                    required_kind.as_deref(),
                    route_suffix.as_deref(),
                )? {
                    // tracing::error!("Scope grants access: {:?}", scope);
                    return Ok(());
                }
            }
            if !is_scoped_token {
                is_scoped_token = true;
            }
        }
    }

    //Edge case for backward compatibility, if only scopes defined was filter tag then don't treat this we don't treat the token
    //as a restricted token
    if !is_scoped_token {
        return Ok(());
    }
    let scope_display = if let Some(kind) = required_kind {
        format!(
            "{}:{}:{}",
            required_domain.as_str(),
            required_action.as_str(),
            kind
        )
    } else {
        format!("{}:{}", required_domain.as_str(), required_action.as_str())
    };

    Err(Error::NotAuthorized(format!(
        "Access denied. Required scope: {}",
        scope_display
    )))
}

const SCRIPT_JOBS: [&'static str; 8] = [
    "jobs/run/p",
    "jobs/run/h",
    "jobs/run_wait_result/p",
    "jobs/run_wait_result/h",
    "jobs/run/preview_bundle",
    "jobs/run/preview",
    "jobs/run_and_stream/p",
    "jobs/run_and_stream/h",
];

const FLOW_JOBS: [&'static str; 6] = [
    "jobs/run/f",
    "jobs/run_wait_result/f",
    "jobs/run/preview_flow",
    "jobs/restart/f",
    "jobs/flow/resume",
    "jobs/run_and_stream/f",
];

lazy_static::lazy_static! {
    static ref RUN_PATH_ACTIONS: Vec<&'static str> = {
        let mut v = vec!["jobs/resume/", "jobs/run/batch_rerun_jobs", "jobs/run/workflow_as_code", "jobs/run/dependencies","jobs/run/flow_dependencies", "apps_u/execute_component"];

        v.extend(SCRIPT_JOBS);
        v.extend(FLOW_JOBS);
        v
    };
}

fn map_http_method_to_action(method: &str, route_path: &str) -> ScopeAction {
    if RUN_PATH_ACTIONS
        .iter()
        .any(|run_path| route_path.contains(run_path))
    {
        return ScopeAction::Run;
    }

    match method.to_uppercase().as_str() {
        "GET" | "HEAD" | "OPTIONS" => ScopeAction::Read,
        "POST" | "PUT" | "PATCH" | "DELETE" => ScopeAction::Write,
        _ => ScopeAction::Read,
    }
}

/// Checks the route path to determine the runnable kind (either "flows" or "scripts").
///
/// The order of checks is important:
/// - Flow-related paths are checked first to avoid false positives, as some flow paths
///   (e.g., `/run_preview_flow`) share prefixes with script paths (e.g., `/run_preview`).
///
/// Returns `"flows"` or `"scripts"` based on the match, or `None` if no match is found.
fn determine_kind_from_route(route_path: &str) -> Option<String> {
    if route_path.starts_with("jobs") {
        if FLOW_JOBS.iter().any(|path| route_path.starts_with(path)) {
            return Some("flows".to_string());
        } else if SCRIPT_JOBS.iter().any(|path| route_path.starts_with(path)) {
            return Some("scripts".to_string());
        }
    }
    None
}

fn extract_domain_from_route(
    route_path: &str,
) -> Result<(ScopeDomain, Option<String>, Option<String>)> {
    // Examples:
    // - /api/w/workspace/jobs/123 -> jobs domain (workspaced)
    // - /api/teams/sync -> teams domain (global)
    // - /api/srch/index/search -> indexer domain (global)
    let parts: Vec<&str> = route_path.split('/').collect();

    let (domain, kind, route_suffix) = if parts.len() >= 5 && parts[1] == "api" && parts[2] == "w" {
        let domain_part = parts[4];
        let route_suffix = &parts[4..].join("/");

        let domain = ScopeDomain::from_str(domain_part);

        let kind = determine_kind_from_route(&route_suffix);

        (domain, kind, Some(route_suffix.to_owned()))
    } else if parts.len() >= 3 && parts[1] == "api" {
        (
            ScopeDomain::from_str(parts[2]),
            None,
            Some(parts[2..].join("/")),
        )
    } else {
        (None, None, None)
    };

    if let Some(domain) = domain {
        // tracing::error!("Extracted domain {:?} from route {:?} with kind {:?} and route suffix {:?}", domain, route_path, kind, route_suffix);
        return Ok((domain, kind, route_suffix));
    }

    Err(Error::BadRequest(format!(
        "Could not extract domain from route: {}",
        route_path
    )))
}

const RUN_WHITELISTED_GET_PATHS: [&'static str; 19] = [
    "jobs_u/get_flow/",
    "jobs_u/get_root_job_id/",
    "jobs_u/get/",
    "jobs_u/get_logs/",
    "jobs_u/get_args/",
    "jobs_u/get_flow_debug_info/",
    "jobs_u/completed/get/",
    "jobs_u/completed/get_result/",
    "jobs_u/completed/get_result_maybe/",
    "jobs_u/getupdate/",
    "jobs_u/getupdate_sse/",
    "jobs_u/get_log_file/",
    "jobs/result_by_id/",
    "jobs/resume_urls/",
    "jobs/flow/user_states/",
    "jobs/job_signature/",
    "jobs/completed/get/",
    "jobs/completed/get_result/",
    "jobs/completed/get_result_maybe/",
];

fn scope_grants_access(
    scope: &ScopeDefinition,
    required_domain: ScopeDomain,
    required_action: ScopeAction,
    required_kind: Option<&str>,
    route_path: Option<&str>,
) -> Result<bool> {
    // Check domain match
    let scope_domain = ScopeDomain::from_str(&scope.domain)
        .ok_or_else(|| Error::BadRequest(format!("Invalid scope domain: {}", scope.domain)))?;

    if scope_domain != required_domain {
        return Ok(false);
    }

    // Check action match (with hierarchical permissions)
    let scope_action = ScopeAction::from_str(&scope.action)
        .ok_or_else(|| Error::BadRequest(format!("Invalid scope action: {}", scope.action)))?;

    if !scope_action.includes(&required_action)
        && !(scope_domain == ScopeDomain::Jobs
            && required_action == ScopeAction::Read
            && route_path.is_some_and(|p| {
                RUN_WHITELISTED_GET_PATHS
                    .iter()
                    .any(|path| p.starts_with(path))
            }))
    {
        return Ok(false);
    }

    if scope_domain == ScopeDomain::Jobs && required_action == ScopeAction::Run {
        match (&scope.kind, required_kind) {
            (Some(scope_kind), Some(req_kind)) => {
                if scope_kind != req_kind {
                    return Ok(false);
                }
            }
            (None, _) => {}
            (Some(_), None) => {
                return Ok(false);
            }
        }
    }

    // No resource specified means access to entire domain
    Ok(true)
}

/// Helper function to check if scopes allow access to a route
pub fn check_scopes_for_route(
    token_scopes: Option<&[String]>,
    route_path: &str,
    http_method: &str,
) -> Result<()> {
    // If no scopes defined, allow access (backward compatibility)
    let scopes = match token_scopes {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };

    check_route_access(scopes, route_path, http_method)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_definition_parsing() {
        let scope = ScopeDefinition::from_scope_string("jobs:read").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "read");
        assert_eq!(scope.kind, None);
        assert_eq!(scope.resource, None);

        let scope = ScopeDefinition::from_scope_string("jobs:run:scripts:f/folder/*").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("scripts".to_string()));
        assert_eq!(scope.resource, Some(vec!["f/folder/*".to_string()]));

        // Test jobs:run:kind parsing
        let scope = ScopeDefinition::from_scope_string("jobs:run:scripts").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("scripts".to_string()));
        assert_eq!(scope.resource, None);

        // Test jobs:run:kind:resource parsing
        let scope = ScopeDefinition::from_scope_string("jobs:run:flows:f/folder/*").unwrap();
        assert_eq!(scope.domain, "jobs");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.kind, Some("flows".to_string()));
        assert_eq!(scope.resource, Some(vec!["f/folder/*".to_string()]));

        // Test comma-separated resources parsing
        let scope =
            ScopeDefinition::from_scope_string("scripts:read:path1,path2,f/folder/*").unwrap();
        assert_eq!(scope.domain, "scripts");
        assert_eq!(scope.action, "read");
        assert_eq!(scope.kind, None);
        assert_eq!(
            scope.resource,
            Some(vec![
                "path1".to_string(),
                "path2".to_string(),
                "f/folder/*".to_string()
            ])
        );
    }

    #[test]
    fn test_scope_action_hierarchy() {
        assert!(ScopeAction::Write.includes(&ScopeAction::Read));
        assert!(!ScopeAction::Read.includes(&ScopeAction::Write));
        assert!(ScopeAction::Run.includes(&ScopeAction::Read));
        assert!(!ScopeAction::Run.includes(&ScopeAction::Write));
    }

    #[test]
    fn test_route_domain_extraction() {
        let (domain, kind, route_suffix) =
            extract_domain_from_route("/api/w/test_workspace/jobs/123").unwrap();
        assert_eq!(domain, ScopeDomain::Jobs);
        assert_eq!(kind, None);
        assert_eq!(route_suffix, Some("jobs/123".to_string()));

        let (domain, kind, route_suffix) =
            extract_domain_from_route("/api/w/test_workspace/scripts/test_script").unwrap();
        assert_eq!(domain, ScopeDomain::Scripts);
        assert_eq!(kind, None);
        assert_eq!(route_suffix, Some("scripts/test_script".to_string()));
    }

    #[test]
    fn test_specific_scope_access() {
        let scopes = vec!["jobs:read".to_string()];

        assert!(check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "GET").is_ok());

        // DELETE now requires write permission, so it should still fail with read-only scope
        assert!(check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "DELETE").is_err());
    }

    #[test]
    fn test_new_domain_parsing() {
        // Test that new domains are properly parsed
        assert_eq!(ScopeDomain::from_str("acls"), Some(ScopeDomain::Acls));
        assert_eq!(
            ScopeDomain::from_str("raw_apps"),
            Some(ScopeDomain::RawApps)
        );
        assert_eq!(
            ScopeDomain::from_str("agent_workers"),
            Some(ScopeDomain::AgentWorkers)
        );

        // Test that string conversion works both ways
        assert_eq!(ScopeDomain::Acls.as_str(), "acls");
        assert_eq!(ScopeDomain::RawApps.as_str(), "raw_apps");
        assert_eq!(ScopeDomain::AgentWorkers.as_str(), "agent_workers");
    }

    #[test]
    fn test_resource_array_matching() {
        // Test wildcard access
        let scope_all = ScopeDefinition::new("scripts", "read", None, Some(vec!["*".to_string()]));
        let required = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        assert!(scope_all.includes(&required));

        // Test exact matches
        let scope_exact = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        let required_subset =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path1".to_string()]));
        assert!(scope_exact.includes(&required_subset));

        // Test partial match - should grant access if ANY required resource matches
        let scope_limited =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path1".to_string()]));
        let required_partial = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        assert!(scope_limited.includes(&required_partial)); // path1 matches, so access granted

        // Test no match - scope doesn't cover any of the required resources
        let scope_different =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path3".to_string()]));
        let required_no_match = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        assert!(!scope_different.includes(&required_no_match));

        // Test pattern matching
        let scope_pattern = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["f/folder/*".to_string()]),
        );
        let required_in_folder = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["f/folder/script1".to_string()]),
        );
        assert!(scope_pattern.includes(&required_in_folder));

        // Test mixed patterns and exact matches
        let scope_mixed = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["exact_path".to_string(), "f/folder/*".to_string()]),
        );
        let required_mixed1 = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["exact_path".to_string()]),
        );
        let required_mixed2 = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["f/folder/script2".to_string()]),
        );
        assert!(scope_mixed.includes(&required_mixed1));
        assert!(scope_mixed.includes(&required_mixed2));
    }

    #[test]
    fn test_efficiency_small_vs_large_arrays() {
        // Test small array optimization path
        let scope_small = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["path1".to_string(), "path2".to_string()]),
        );
        let required_small =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path1".to_string()]));
        assert!(scope_small.includes(&required_small));

        // Test large array optimization path
        let large_scope_vec: Vec<String> = (0..10).map(|i| format!("path{}", i)).collect();
        let scope_large = ScopeDefinition::new("scripts", "read", None, Some(large_scope_vec));
        let required_large =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["path5".to_string()]));
        assert!(scope_large.includes(&required_large));
    }

    #[test]
    fn test_user_example_case() {
        let user_scope =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["u/dieri/*".to_string()]));
        let required_mixed = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["u/dadad/wqdq".to_string(), "u/*".to_string()]),
        );
        assert!(user_scope.includes(&required_mixed));

        let scope_specific = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["folder/file1".to_string()]),
        );
        let required_multi = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["folder/file1".to_string(), "other/file2".to_string()]),
        );
        assert!(scope_specific.includes(&required_multi));

        let scope_broad =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["u/*".to_string()]));
        let required_specific = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["u/dieri/script.py".to_string()]),
        );
        assert!(scope_broad.includes(&required_specific));

        let scope_specific_path = ScopeDefinition::new(
            "scripts",
            "read",
            None,
            Some(vec!["u/dieri/script.py".to_string()]),
        );
        let required_broad =
            ScopeDefinition::new("scripts", "read", None, Some(vec!["u/*".to_string()]));
        assert!(scope_specific_path.includes(&required_broad));
    }
}
