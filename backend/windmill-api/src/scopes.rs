/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde::{Deserialize, Serialize};
use windmill_common::error::{Error, Result};

/// Comprehensive scope system for JWT token authorization
///
/// Scopes follow the format: {domain}:{action}[:{resource}]
/// Examples:
/// - "jobs:read" - Read access to jobs
/// - "scripts:write:f/folder/*" - Write access to scripts in a folder
/// - "*" - Full access (superuser)

const RUN_KEYWORD: [&'static str; 4] = ["/run", "/execute", "/restart", "/resume"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeDefinition {
    pub domain: String,
    pub action: String,
    pub resource: Option<String>,
}

impl ScopeDefinition {
    pub fn new(domain: &str, action: &str, resource: Option<&str>) -> Self {
        Self {
            domain: domain.to_string(),
            action: action.to_string(),
            resource: resource.map(|s| s.to_string()),
        }
    }

    pub fn from_scope_string(scope: &str) -> Result<Self> {
        let parts: Vec<&str> = scope.split(':').collect();

        match parts.len() {
            2 => Ok(Self::new(parts[0], parts[1], None)),
            3 => Ok(Self::new(parts[0], parts[1], Some(parts[2]))),
            _ => Err(Error::BadRequest(format!(
                "Invalid scope format: {}",
                scope
            ))),
        }
    }
}

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
    JobsU,
}

impl ScopeDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Jobs => "jobs",
            Self::JobsU => "jobs_u",
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
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "jobs" => Some(Self::Jobs),
            "jobs_u" => Some(Self::JobsU),
            "scripts" => Some(Self::Scripts),
            "flows" => Some(Self::Flows),
            "apps" => Some(Self::Apps),
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
            "audit" => Some(Self::Audit),
            "settings" => Some(Self::Settings),
            "workers" => Some(Self::Workers),
            "service_logs" => Some(Self::ServiceLogs),
            "configs" => Some(Self::Configs),
            "oauth" => Some(Self::OAuth),
            "ai" => Some(Self::AI),
            "indexer" | "srch" => Some(Self::Indexer),
            "teams" => Some(Self::Teams),
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
            _ => None,
        }
    }
}

/// Available scope actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeAction {
    Read,   // GET operations, list, view
    Write,  // POST, PUT, PATCH operations, create, update
    Delete, // DELETE operations
    Run,    // Special action for running (scripts, flows, etc.)
    Admin,  // Administrative operations within the domain
}

impl ScopeAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Delete => "delete",
            Self::Run => "run",
            Self::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "delete" => Some(Self::Delete),
            "run" => Some(Self::Run),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    /// Check if this action includes another action
    /// Admin includes all actions
    pub fn includes(&self, other: &ScopeAction) -> bool {
        match (self, other) {
            (ScopeAction::Admin, _) => true,
            (a, b) => a == b,
        }
    }
}

pub fn check_route_access(
    token_scopes: &[String],
    route_path: &str,
    http_method: &str,
) -> Result<()> {
    // Special case: "*" scope grants all access
    if token_scopes.contains(&"*".to_string()) {
        return Ok(());
    }

    // Map HTTP method to scope action (considering route context)
    let required_action = map_http_method_to_action(http_method, route_path);

    // Find the domain for this route
    let required_domain = extract_domain_from_route(route_path)?;
    let mut filter_tags = false;
    let mut restricted_scopes = false;
    // Check if any token scope grants the required access
    for scope_str in token_scopes {
        if scope_str.starts_with("if_jobs:filter_tags:") && !filter_tags {
            filter_tags = true
        } else {
            if let Ok(scope) = ScopeDefinition::from_scope_string(scope_str) {
                if scope_grants_access(&scope, required_domain, required_action, route_path)? {
                    return Ok(());
                }
            }
            if !restricted_scopes {
                restricted_scopes = true;
            }
        }
    }

    //Edge case for backward compatibility, if only scopes defined was filter tag then don't treat this we don't treat the token
    //as a restricted token
    if filter_tags && !restricted_scopes {
        return Ok(());
    }

    Err(Error::NotAuthorized(format!(
        "Access denied. Required scope: {}:{}",
        required_domain.as_str(),
        required_action.as_str()
    )))
}

fn map_http_method_to_action(method: &str, route_path: &str) -> ScopeAction {
    match method.to_uppercase().as_str() {
        "GET" | "HEAD" | "OPTIONS" => ScopeAction::Read,
        "POST" => {
            // POST can be create (write) or run depending on the endpoint
            if RUN_KEYWORD.iter().any(|path| route_path.contains(path)) {
                ScopeAction::Run
            } else {
                ScopeAction::Write
            }
        }
        "PUT" | "PATCH" => ScopeAction::Write,
        "DELETE" => ScopeAction::Delete,
        _ => ScopeAction::Read,
    }
}

const SCRIPT_JOBS: [&'static str; 5] = [
    "jobs/run/p",
    "jobs/run/h",
    "jobs/run_wait_result/p",
    "jobs/run_wait_result/h",
    "jobs/run/preview",
];

const FLOW_JOBS: [&'static str; 6] = [
    "jobs/run/f",
    "jobs/run_wait_result/f",
    "jobs/run/preview_flow",
    "jobs/restart/f",
    "jobs/flow/resume",
    "jobs/flow/user_states",
];

fn is_script_or_flow_domain(route_path: &str) -> Option<ScopeDomain> {
    if route_path.starts_with("jobs") {
        if SCRIPT_JOBS.iter().any(|path| route_path.starts_with(path)) {
            return Some(ScopeDomain::Scripts);
        } else if FLOW_JOBS.iter().any(|path| route_path.starts_with(path)) {
            return Some(ScopeDomain::Flows);
        }
    }

    return None;
}

fn extract_domain_from_route(route_path: &str) -> Result<ScopeDomain> {
    // Examples:
    // - /api/w/workspace/jobs/123 -> jobs domain (workspaced)
    // - /api/teams/sync -> teams domain (global)
    // - /api/srch/index/search -> indexer domain (global)
    let parts: Vec<&str> = route_path.split('/').collect();

    let domain = if parts.len() >= 5 && parts[1] == "api" && parts[2] == "w" {
        let domain_part = parts[4];

        let domain = match is_script_or_flow_domain(&parts[4..].join("/")) {
            None => ScopeDomain::from_str(domain_part),
            domain => domain,
        };

        domain
    } else if parts.len() >= 3 && parts[1] == "api" {
        ScopeDomain::from_str(parts[2])
    } else {
        None
    };

    if let Some(domain) = domain {
        return Ok(domain);
    }

    Err(Error::BadRequest(format!(
        "Could not extract domain from route: {}",
        route_path
    )))
}

fn scope_grants_access(
    scope: &ScopeDefinition,
    required_domain: ScopeDomain,
    required_action: ScopeAction,
    route_path: &str,
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

    if !scope_action.includes(&required_action) {
        return Ok(false);
    }

    // Check resource path match if specified
    if let Some(scope_resource) = &scope.resource {
        return Ok(resource_path_matches(scope_resource, route_path));
    }

    // No resource specified means access to entire domain
    Ok(true)
}

fn resource_path_matches(scope_resource: &str, route_path: &str) -> bool {
    if scope_resource == "*" {
        return true;
    }

    if scope_resource.ends_with("*") {
        let prefix = &scope_resource[..scope_resource.len() - 1];
        return route_path.contains(prefix);
    }

    route_path.contains(scope_resource)
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
        assert_eq!(scope.resource, None);

        let scope = ScopeDefinition::from_scope_string("scripts:run:f/folder/*").unwrap();
        assert_eq!(scope.domain, "scripts");
        assert_eq!(scope.action, "run");
        assert_eq!(scope.resource, Some("f/folder/*".to_string()));
    }

    #[test]
    fn test_scope_action_hierarchy() {
        assert!(ScopeAction::Admin.includes(&ScopeAction::Read));
        assert!(ScopeAction::Admin.includes(&ScopeAction::Write));
        assert!(ScopeAction::Admin.includes(&ScopeAction::Delete));
        assert!(ScopeAction::Write.includes(&ScopeAction::Read));
        assert!(!ScopeAction::Read.includes(&ScopeAction::Write));
    }

    #[test]
    fn test_route_domain_extraction() {
        let domain = extract_domain_from_route("/api/w/test_workspace/jobs/123").unwrap();
        assert_eq!(domain, ScopeDomain::Jobs);

        let domain =
            extract_domain_from_route("/api/w/test_workspace/scripts/test_script").unwrap();
        assert_eq!(domain, ScopeDomain::Scripts);
    }

    #[test]
    fn test_wildcard_scope_access() {
        let scopes = vec!["*".to_string()];

        assert!(check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "GET").is_ok());
    }

    #[test]
    fn test_specific_scope_access() {
        let scopes = vec!["jobs:read".to_string()];

        assert!(check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "GET").is_ok());

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
}
