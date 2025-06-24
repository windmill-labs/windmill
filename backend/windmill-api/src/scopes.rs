/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    pub fn to_scope_string(&self) -> String {
        match &self.resource {
            Some(resource) => format!("{}:{}:{}", self.domain, self.action, resource),
            None => format!("{}:{}", self.domain, self.action),
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
    Integrations,
    OAuth,
    AI,
    Embeddings,

    // Special domains
    Admin,       // Administrative functions
    Capture,     // Webhook capture
    Drafts,      // Draft resources
    Favorites,   // User favorites
    Inputs,      // Input templates
    JobMetrics,  // Job metrics and statistics
    JobHelpers,  // Job helper functions
    Concurrency, // Concurrency groups
    Oidc,        // OpenID Connect
    Openapi,     // OpenAPI documentation
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
            Self::Audit => "audit",
            Self::Settings => "settings",
            Self::Workers => "workers",
            Self::ServiceLogs => "service_logs",
            Self::Configs => "configs",
            Self::Integrations => "integrations",
            Self::OAuth => "oauth",
            Self::AI => "ai",
            Self::Embeddings => "embeddings",
            Self::Admin => "admin",
            Self::Capture => "capture",
            Self::Drafts => "drafts",
            Self::Favorites => "favorites",
            Self::Inputs => "inputs",
            Self::JobMetrics => "job_metrics",
            Self::JobHelpers => "job_helpers",
            Self::Concurrency => "concurrency",
            Self::Oidc => "oidc",
            Self::Openapi => "openapi",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "jobs" => Some(Self::Jobs),
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
            "integrations" => Some(Self::Integrations),
            "oauth" => Some(Self::OAuth),
            "ai" => Some(Self::AI),
            "embeddings" => Some(Self::Embeddings),
            "admin" => Some(Self::Admin),
            "capture" => Some(Self::Capture),
            "drafts" => Some(Self::Drafts),
            "favorites" => Some(Self::Favorites),
            "inputs" => Some(Self::Inputs),
            "job_metrics" => Some(Self::JobMetrics),
            "job_helpers" => Some(Self::JobHelpers),
            "concurrency" => Some(Self::Concurrency),
            "oidc" => Some(Self::Oidc),
            "openapi" => Some(Self::Openapi),
            _ => None,
        }
    }
}

/// Available scope actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeAction {
    Read,    // GET operations, list, view
    Write,   // POST, PUT, PATCH operations, create, update
    Delete,  // DELETE operations
    Execute, // Special action for running/executing (scripts, flows, etc.)
    Admin,   // Administrative operations within the domain
}

impl ScopeAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Delete => "delete",
            Self::Execute => "execute",
            Self::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "delete" => Some(Self::Delete),
            "execute" => Some(Self::Execute),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    /// Check if this action includes another action
    /// Admin includes all actions, Write includes Read, etc.
    pub fn includes(&self, other: &ScopeAction) -> bool {
        match (self, other) {
            (ScopeAction::Admin, _) => true,
            (ScopeAction::Write, ScopeAction::Read) => true,
            (a, b) => a == b,
        }
    }
}

/// Scope matcher for validating token scopes against required scopes
pub struct ScopeMatcher {
    route_mappings: HashMap<String, (ScopeDomain, ScopeAction)>,
}

impl ScopeMatcher {
    pub fn new() -> Self {
        let mut matcher = Self { route_mappings: HashMap::new() };
        matcher.initialize_route_mappings();
        matcher
    }

    /// Initialize route to scope mappings based on Windmill's API structure
    fn initialize_route_mappings(&mut self) {
        // This is now simplified - we'll use a more dynamic approach in extract_domain_from_route
        // Just store a few key special cases here

        // Special execution routes that map to different actions
        self.add_route_mapping(
            "/api/w/:workspace_id/jobs/run",
            ScopeDomain::Jobs,
            ScopeAction::Execute,
        );
        self.add_route_mapping(
            "/api/w/:workspace_id/scripts/run",
            ScopeDomain::Scripts,
            ScopeAction::Execute,
        );
        self.add_route_mapping(
            "/api/w/:workspace_id/flows/run",
            ScopeDomain::Flows,
            ScopeAction::Execute,
        );
    }

    fn add_route_mapping(&mut self, route: &str, domain: ScopeDomain, action: ScopeAction) {
        self.route_mappings
            .insert(route.to_string(), (domain, action));
    }

    /// Check if token scopes allow access to a specific route
    pub fn check_route_access(
        &self,
        token_scopes: &[String],
        route_path: &str,
        http_method: &str,
    ) -> Result<()> {
        // Special case: "*" scope grants all access
        if token_scopes.contains(&"*".to_string()) {
            return Ok(());
        }

        // Map HTTP method to scope action (considering route context)
        let required_action = self.map_http_method_to_action(http_method, route_path);

        // Find the domain for this route
        let required_domain = self.extract_domain_from_route(route_path)?;

        // Check if any token scope grants the required access
        for scope_str in token_scopes {
            if let Ok(scope) = ScopeDefinition::from_scope_string(scope_str) {
                if self.scope_grants_access(&scope, required_domain, required_action, route_path)? {
                    return Ok(());
                }
            }
        }

        Err(Error::BadRequest(format!(
            "Access denied. Required scope: {}:{}",
            required_domain.as_str(),
            required_action.as_str()
        )))
    }

    fn map_http_method_to_action(&self, method: &str, route_path: &str) -> ScopeAction {
        // Special cases for execution endpoints
        if route_path.contains("/run") || route_path.contains("/execute") {
            return ScopeAction::Execute;
        }

        // Special cases for cancellation, resumption, etc.
        if route_path.contains("/cancel") || route_path.contains("/resume") {
            return ScopeAction::Write;
        }

        // Map HTTP methods to actions
        match method.to_uppercase().as_str() {
            "GET" | "HEAD" | "OPTIONS" => ScopeAction::Read,
            "POST" => {
                // POST can be create (write) or execute depending on the endpoint
                if route_path.ends_with("/run") || route_path.contains("/execute") {
                    ScopeAction::Execute
                } else {
                    ScopeAction::Write
                }
            }
            "PUT" | "PATCH" => ScopeAction::Write,
            "DELETE" => ScopeAction::Delete,
            _ => ScopeAction::Read, // Default to read for unknown methods
        }
    }

    fn extract_domain_from_route(&self, route_path: &str) -> Result<ScopeDomain> {
        // Extract the domain from the route path
        // Example: /api/w/workspace/jobs/123 -> jobs domain
        let parts: Vec<&str> = route_path.split('/').collect();

        if parts.len() >= 5 && parts[1] == "api" && parts[2] == "w" {
            let domain_part = parts[4];

            if let Some(domain) = ScopeDomain::from_str(domain_part) {
                return Ok(domain);
            }
        }

        Err(Error::BadRequest(format!(
            "Could not extract domain from route: {}",
            route_path
        )))
    }

    fn scope_grants_access(
        &self,
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
            return Ok(self.resource_path_matches(scope_resource, route_path));
        }

        // No resource specified means access to entire domain
        Ok(true)
    }

    fn resource_path_matches(&self, scope_resource: &str, route_path: &str) -> bool {
        // Handle wildcard patterns
        if scope_resource == "*" {
            return true;
        }

        // Handle folder paths (f/folder_name/*)
        if scope_resource.starts_with("f/") && scope_resource.ends_with("/*") {
            let folder_path = &scope_resource[2..scope_resource.len() - 2];
            return route_path.contains(&format!("f/{}", folder_path));
        }

        // Handle specific resource paths
        if scope_resource.ends_with("*") {
            let prefix = &scope_resource[..scope_resource.len() - 1];
            return route_path.contains(prefix);
        }

        // Exact match
        route_path.contains(scope_resource)
    }
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

    let matcher = ScopeMatcher::new();
    matcher.check_route_access(scopes, route_path, http_method)
}

/// Create standard scopes for common operations
pub fn create_standard_scopes() -> Vec<String> {
    let mut scopes = Vec::new();

    // Core resource scopes
    for domain in [
        ScopeDomain::Jobs,
        ScopeDomain::Scripts,
        ScopeDomain::Flows,
        ScopeDomain::Apps,
        ScopeDomain::Variables,
        ScopeDomain::Resources,
        ScopeDomain::Schedules,
        ScopeDomain::Folders,
        ScopeDomain::Users,
        ScopeDomain::Groups,
    ] {
        for action in [
            ScopeAction::Read,
            ScopeAction::Write,
            ScopeAction::Delete,
            ScopeAction::Execute,
        ] {
            scopes.push(format!("{}:{}", domain.as_str(), action.as_str()));
        }
    }

    // Admin scopes
    for domain in [
        ScopeDomain::Jobs,
        ScopeDomain::Scripts,
        ScopeDomain::Flows,
        ScopeDomain::Apps,
        ScopeDomain::Variables,
        ScopeDomain::Resources,
        ScopeDomain::Schedules,
        ScopeDomain::Folders,
        ScopeDomain::Users,
        ScopeDomain::Groups,
        ScopeDomain::Workspaces,
    ] {
        scopes.push(format!("{}:admin", domain.as_str()));
    }

    scopes
}

/// Legacy scope checking function for backward compatibility
pub fn check_legacy_scopes(token_scopes: Option<&[String]>, required_scope: &str) -> Result<()> {
    let scopes = match token_scopes {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(()),
    };

    // Check for exact match or wildcard
    if scopes.contains(&"*".to_string()) || scopes.contains(&required_scope.to_string()) {
        return Ok(());
    }

    Err(Error::BadRequest(format!(
        "Missing required scope: {}",
        required_scope
    )))
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

        let scope = ScopeDefinition::from_scope_string("scripts:execute:f/folder/*").unwrap();
        assert_eq!(scope.domain, "scripts");
        assert_eq!(scope.action, "execute");
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
        let matcher = ScopeMatcher::new();
        let domain = matcher
            .extract_domain_from_route("/api/w/test_workspace/jobs/123")
            .unwrap();
        assert_eq!(domain, ScopeDomain::Jobs);

        let domain = matcher
            .extract_domain_from_route("/api/w/test_workspace/scripts/test_script")
            .unwrap();
        assert_eq!(domain, ScopeDomain::Scripts);
    }

    #[test]
    fn test_wildcard_scope_access() {
        let matcher = ScopeMatcher::new();
        let scopes = vec!["*".to_string()];

        assert!(matcher
            .check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "GET")
            .is_ok());
    }

    #[test]
    fn test_specific_scope_access() {
        let matcher = ScopeMatcher::new();
        let scopes = vec!["jobs:read".to_string()];

        assert!(matcher
            .check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "GET")
            .is_ok());

        assert!(matcher
            .check_route_access(&scopes, "/api/w/test_workspace/jobs/123", "DELETE")
            .is_err());
    }
}
